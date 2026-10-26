mod client;
mod icons;
mod scene;
mod workbench;
use client::{Client, Message, Update};
use gpui::{prelude::*, *};
use gpui_component::{
    Root, Theme, ThemeMode,
    input::{Input, InputEvent, InputState, TabSize},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{cell::Cell, collections::BTreeSet, path::PathBuf, rc::Rc, sync::Arc};
use wirecraft_core::{
    Document,
    viewport::{Rect, Viewport},
};
use wirecraft_runtime::service::{Request, Snapshot};

actions!(
    wirecraft,
    [
        Open,
        Save,
        SaveAs,
        New,
        Undo,
        Redo,
        DeleteSelection,
        SelectMode,
        NodeMode,
        LinkMode,
        Fit,
        Apply,
        Quit,
        CopyEntities,
        CutEntities,
        PasteEntities,
        CommandPalette,
        ShowExport,
        ShowOptions
    ]
);
#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Select,
    Node,
    Type,
    Link,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct UiSession {
    viewport: Viewport,
    parent: String,
    selected: Option<String>,
    tab: String,
    section: String,
    draft: Option<Draft>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Draft {
    document_id: String,
    node_id: String,
    section: String,
    name: String,
    text: String,
    properties: String,
    base_name: String,
    base_text: String,
    base_properties: String,
}
struct Wirecraft {
    client: Client,
    snapshot: Arc<Snapshot>,
    scene: Arc<scene::Scene>,
    viewport: Viewport,
    parent: String,
    selected: BTreeSet<String>,
    mode: Mode,
    link_source: Option<String>,
    tab: String,
    section: String,
    name: Entity<InputState>,
    editor: Entity<InputState>,
    properties: Entity<InputState>,
    base_name: String,
    base_text: String,
    base_properties: String,
    edit_revision: u64,
    pending_draft: Option<Draft>,
    editor_entity: Option<String>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    drag_start: Option<[f64; 2]>,
    drag_delta: [f64; 2],
    pan_start: Option<([f64; 2], Viewport)>,
    selection_start: Option<[f64; 2]>,
    focus: FocusHandle,
    status: String,
    endpoint: String,
    token: String,
    output: String,
    tree: Vec<(String, usize)>,
    archetypes: Vec<Value>,
    generated_diagnostics: Vec<Value>,
    generated_revision: u64,
    inspector: Vec<workbench::FormField>,
    advanced: bool,
    editor_committed: bool,
    menu: Option<String>,
    query: Entity<InputState>,
    report: Value,
    analysis_layer: String,
    management_backlog: bool,
    archetype_selected: Option<String>,
    archetype_preview: Option<Arc<scene::Scene>>,
}
impl Wirecraft {
    fn new(
        client: Client,
        rx: async_channel::Receiver<Update>,
        session: UiSession,
        indent: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Entity name"));
        let editor = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("plain")
                .tab_size(TabSize {
                    tab_size: indent,
                    hard_tabs: false,
                })
        });
        let properties = cx.new(|cx| InputState::new(window, cx).code_editor("json"));
        let snapshot = Arc::new(Snapshot {
            document: Document::new("Connecting…"),
            revision: 0,
            dirty: false,
            path: None,
            can_undo: false,
            can_redo: false,
        });
        let inspector = workbench::make_fields(window, cx);
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search commands…"));
        cx.subscribe(&query, |_, _, _: &InputEvent, cx| cx.notify())
            .detach();
        let focus = cx.focus_handle();
        focus.focus(window);
        let this = Self {
            client,
            snapshot,
            scene: Arc::new(scene::Scene::new(&Document::new(""), "root")),
            viewport: session.viewport,
            parent: if session.parent.is_empty() {
                "root".into()
            } else {
                session.parent
            },
            selected: session.selected.into_iter().collect(),
            mode: Mode::Select,
            link_source: None,
            tab: if session.tab.is_empty() {
                "Designer".into()
            } else {
                session.tab
            },
            section: if session.section.is_empty() {
                "source".into()
            } else {
                session.section
            },
            name,
            editor,
            properties,
            base_name: String::new(),
            base_text: String::new(),
            base_properties: String::new(),
            edit_revision: 0,
            pending_draft: session.draft,
            editor_entity: None,
            bounds: Rc::new(Cell::new(Bounds::default())),
            drag_start: None,
            drag_delta: [0., 0.],
            pan_start: None,
            selection_start: None,
            focus,
            status: "Starting local backend…".into(),
            endpoint: String::new(),
            token: String::new(),
            output: String::new(),
            tree: vec![],
            archetypes: vec![],
            generated_diagnostics: vec![],
            generated_revision: 0,
            inspector,
            advanced: false,
            editor_committed: false,
            menu: None,
            query,
            report: Value::Null,
            analysis_layer: "principal".into(),
            management_backlog: false,
            archetype_selected: None,
            archetype_preview: None,
        };
        cx.spawn_in(window, async move |this, cx| {
            while let Ok(update) = rx.recv().await {
                if this
                    .update_in(cx, |this, window, cx| this.receive(update, window, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        for input in [&this.name, &this.editor, &this.properties]
            .into_iter()
            .chain(this.inspector.iter().map(|f| &f.input))
        {
            cx.subscribe(input, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.persist_ui(cx);
                }
            })
            .detach();
        }
        cx.on_app_quit(|this, cx| {
            this.persist_ui(cx);
            let _ = this.client.sender.send(Message::Shutdown);
            async {}
        })
        .detach();
        this
    }
    fn receive(&mut self, update: Update, window: &mut Window, cx: &mut Context<Self>) {
        match update {
            Update::Connected { endpoint, token } => {
                self.endpoint = endpoint;
                self.token = token;
                self.status = "Connected".into();
            }
            Update::Error(e) => {
                self.status = e;
            }
            Update::Result { command, value } => {
                self.status = match command.as_str() {
                    "file.save" | "file.commit_and_save" => "Project saved".into(),
                    "threadwork.update_entity" | "entities.patch" => "Changes applied".into(),
                    _ => format!("{command} completed"),
                };
                if ["threadwork.create_entity", "threadwork.create_link"]
                    .contains(&command.as_str())
                    && let Some(id) = value["entity"]["id"].as_str()
                {
                    self.selected = BTreeSet::from([id.into()]);
                    self.editor_entity = None;
                }
                if (command == "threadwork.update_entity" || command == "file.commit_and_save")
                    && value["entity"]["id"].as_str() == self.selected.first().map(String::as_str)
                {
                    self.base_name = value["entity"]["name"].as_str().unwrap_or("").into();
                    self.base_text = value["entity"]["text"][&self.section]
                        .as_str()
                        .unwrap_or("")
                        .into();
                    let mut props = value["entity"].clone();
                    for k in [
                        "id",
                        "name",
                        "parentId",
                        "children",
                        "incomingLinks",
                        "outgoingLinks",
                        "text",
                        "modified",
                    ] {
                        props.as_object_mut().unwrap().remove(k);
                    }
                    self.base_properties = serde_json::to_string_pretty(&props).unwrap();
                }
                if command == "entities.copy" || command == "entities.cut" {
                    cx.write_to_clipboard(ClipboardItem::new_string(value.to_string()));
                }
                if [
                    "entities.paste",
                    "archetypes.import",
                    "archetypes.import_file",
                ]
                .contains(&command.as_str())
                {
                    self.selected = value["nodeIds"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|id| id.as_str().map(str::to_string))
                        .collect();
                    self.editor_entity = None;
                }
                if command == "archetypes.preview"
                    && value["id"].as_str() == self.archetype_selected.as_deref()
                    && let Ok(document) =
                        serde_json::from_value::<Document>(value["document"].clone())
                {
                    self.archetype_preview = Some(Arc::new(scene::Scene::new(
                        &document,
                        &document.root_node_id,
                    )));
                }
                if command == "analysis.report" {
                    self.report = value.clone();
                }
                if ["threadwork.update_entity", "file.commit_and_save"].contains(&command.as_str())
                {
                    self.editor_committed = true;
                }
                if command == "archetypes.list" {
                    self.archetypes = value["archetypes"].as_array().cloned().unwrap_or_default();
                }
                if command == "export.markdown"
                    && let Some(text) = value["markdown"].as_str()
                {
                    cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
                }
                if command == "editor.analyze_generated" {
                    self.generated_revision = value["revision"].as_u64().unwrap_or(0);
                    self.generated_diagnostics =
                        value["diagnostics"].as_array().cloned().unwrap_or_default();
                    for d in &mut self.generated_diagnostics {
                        d["positionEncoding"] = value["positionEncoding"].clone();
                    }
                }
                if command.starts_with("lsp.")
                    || command.starts_with("jobs.")
                    || command == "threadwork.analyze_network"
                    || command == "threadwork.validate_design"
                    || command == "analysis.report"
                    || command == "editor.analyze_generated"
                {
                    self.output = serde_json::to_string_pretty(&value).unwrap_or_default();
                }
                if let Some(warnings) = value["_wirecraftWarnings"].as_array() {
                    self.status = warnings
                        .iter()
                        .filter_map(|w| w["message"].as_str())
                        .collect::<Vec<_>>()
                        .join("; ");
                }
            }
            Update::Snapshot(snapshot) => {
                self.snapshot = Arc::new(*snapshot);
                if self.snapshot.revision != self.generated_revision {
                    self.generated_diagnostics.clear();
                }
                if !self.snapshot.document.nodes.contains_key(&self.parent) {
                    self.parent = self.snapshot.document.root_node_id.clone();
                }
                self.selected
                    .retain(|id| self.snapshot.document.nodes.contains_key(id));
                if self.selected.is_empty() {
                    self.selected.insert(self.parent.clone());
                }
                self.rebuild();
                if let Some(draft) = self.pending_draft.take()
                    && draft.document_id == self.snapshot.document.id
                    && self.snapshot.document.nodes.contains_key(&draft.node_id)
                {
                    self.selected = BTreeSet::from([draft.node_id.clone()]);
                    self.section = draft.section.clone();
                    self.load_editor(window, cx);
                    let matches = self.base_name == draft.base_name
                        && self.base_text == draft.base_text
                        && self.base_properties == draft.base_properties;
                    self.base_name = draft.base_name;
                    self.base_text = draft.base_text;
                    self.base_properties = draft.base_properties;
                    self.name
                        .update(cx, |state, cx| state.set_value(draft.name, window, cx));
                    self.editor
                        .update(cx, |state, cx| state.set_value(draft.text, window, cx));
                    self.properties.update(cx, |state, cx| {
                        state.set_value(draft.properties, window, cx)
                    });
                    if !matches {
                        self.edit_revision = u64::MAX;
                        self.status="Recovered draft conflicts with the graph. Copy it or reload the entity.".into();
                    }
                }
                let clean = !self.has_draft(cx);
                if clean
                    || self.editor_committed
                    || self.editor_entity.as_ref() != self.selected.first()
                {
                    self.editor_committed = false;
                    self.load_editor(window, cx);
                } else {
                    self.status =
                        "Draft retained. Apply checks for concurrent project changes.".into();
                }
            }
        }
        cx.notify();
    }
    fn rebuild(&mut self) {
        self.scene = Arc::new(scene::Scene::new(&self.snapshot.document, &self.parent));
        self.tree.clear();
        let mut stack = vec![(self.snapshot.document.root_node_id.clone(), 0)];
        while let Some((id, depth)) = stack.pop() {
            if let Some(n) = self.snapshot.document.nodes.get(&id) {
                self.tree.push((id.clone(), depth));
                stack.extend(n.children.iter().rev().cloned().map(|id| (id, depth + 1)));
            }
        }
    }
    fn persist_ui(&self, cx: &App) {
        let state = UiSession {
            viewport: self.viewport,
            parent: self.parent.clone(),
            selected: self.selected.first().cloned(),
            tab: self.tab.clone(),
            section: self.section.clone(),
            draft: if self.has_draft(cx) {
                self.selected.first().map(|id| Draft {
                    document_id: self.snapshot.document.id.clone(),
                    node_id: id.clone(),
                    section: self.section.clone(),
                    name: self.name.read(cx).value().to_string(),
                    text: self.editor.read(cx).value().to_string(),
                    properties: self.merged_properties(cx),
                    base_name: self.base_name.clone(),
                    base_text: self.base_text.clone(),
                    base_properties: self.base_properties.clone(),
                })
            } else {
                None
            },
        };
        let _ = self
            .client
            .sender
            .send(Message::SaveUi(serde_json::to_value(state).unwrap()));
    }
    fn load_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(n) = self
            .selected
            .first()
            .and_then(|id| self.snapshot.document.nodes.get(id))
        {
            self.editor_entity = Some(n.id.clone());
            self.base_name = n.name.clone();
            let text = serde_json::to_value(&n.text).unwrap();
            self.base_text = text[&self.section].as_str().unwrap_or("").into();
            let mut props = serde_json::to_value(n).unwrap();
            for k in [
                "id",
                "name",
                "parentId",
                "children",
                "incomingLinks",
                "outgoingLinks",
                "text",
                "modified",
            ] {
                props.as_object_mut().unwrap().remove(k);
            }
            self.base_properties = serde_json::to_string_pretty(&props).unwrap();
            let all = serde_json::to_value(n).unwrap();
            for field in &mut self.inspector {
                let value = all
                    .pointer(field.path)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                field.baseline = value.clone();
                field
                    .input
                    .update(cx, |state, cx| state.set_value(value, window, cx));
            }
            let language = self
                .snapshot
                .document
                .effective_text_language(&n.id, &self.section);
            self.editor
                .update(cx, |state, cx| state.set_highlighter(language, cx));
            self.edit_revision = self.snapshot.revision;
            self.name.update(cx, |state, cx| {
                state.set_value(self.base_name.clone(), window, cx)
            });
            self.editor.update(cx, |state, cx| {
                state.set_value(self.base_text.clone(), window, cx)
            });
            self.properties.update(cx, |state, cx| {
                state.set_value(self.base_properties.clone(), window, cx)
            });
        }
    }
    fn apply(&mut self, _: &Apply, _: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.selected.first() else {
            return;
        };
        let name = self.name.read(cx).value().to_string();
        let text = self.editor.read(cx).value().to_string();
        let properties = self.merged_properties(cx);
        let mut patch = json!({});
        if name != self.base_name {
            patch["name"] = json!(name);
        }

        if properties != self.base_properties {
            match serde_json::from_str::<Value>(&properties) {
                Ok(Value::Object(o)) => patch.as_object_mut().unwrap().extend(o),
                _ => {
                    self.status = "Properties must be a valid JSON object".into();
                    cx.notify();
                    return;
                }
            }
        }
        if text != self.base_text {
            patch["text"][&self.section] = json!(text);
        }
        if patch.as_object().unwrap().is_empty() {
            return;
        }
        let request = Request {
            command: "threadwork.update_entity".into(),
            arguments: json!({"nodeId":id,"patch":patch}),
            expected_revision: Some(self.edit_revision),
        };
        let _ = self.client.sender.send(Message::Execute(request));
        self.status = "Applying…".into();
        cx.notify();
    }
    fn has_draft(&self, cx: &App) -> bool {
        self.name.read(cx).value().as_ref() != self.base_name
            || self.editor.read(cx).value().as_ref() != self.base_text
            || self.merged_properties(cx) != self.base_properties
    }
    fn select(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.has_draft(cx) {
            self.status = "Apply the editor draft before changing entity".into();
            cx.notify();
            return;
        }
        self.selected = BTreeSet::from([id]);
        self.load_editor(window, cx);
        self.persist_ui(cx);
        cx.notify();
    }
    fn open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        if self.has_draft(cx) {
            self.status = "Apply the editor draft before opening another project".into();
            cx.notify();
            return;
        }
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open a Threadworks .orch project".into()),
        });
        cx.spawn_in(window, async move |this, cx| match paths.await {
            Ok(Ok(Some(paths))) => {
                let _ = this.update_in(cx, |this, _, _| {
                    if let Some(path) = paths.first() {
                        this.client.send("file.open", json!({"path":path}));
                    }
                });
            }
            Ok(Err(e)) => {
                let _ = this.update_in(cx, |this, _, cx| {
                    this.status = e.to_string();
                    cx.notify();
                });
            }
            _ => {}
        })
        .detach();
    }
    fn save_arguments(&self, path: Option<PathBuf>, cx: &App) -> Result<Value, String> {
        let mut a = json!({});
        if let Some(path) = path {
            a["path"] = json!(path);
        }
        if self.has_draft(cx) {
            let id = self.selected.first().ok_or("Select an entity")?;
            let mut patch = json!({});
            let name = self.name.read(cx).value().to_string();
            let text = self.editor.read(cx).value().to_string();
            let properties = self.merged_properties(cx);
            if name != self.base_name {
                patch["name"] = json!(name);
            }
            if text != self.base_text {
                patch["text"] = json!({self.section.clone():text});
            }
            if properties != self.base_properties {
                let p: Value = serde_json::from_str(&properties).map_err(|e| e.to_string())?;
                let o = p.as_object().ok_or("Properties must be an object")?;
                patch.as_object_mut().unwrap().extend(o.clone());
            }
            if text != self.base_text {
                patch["text"][&self.section] = json!(text);
            }
            a["nodeId"] = json!(id);
            a["patch"] = patch;
        }
        Ok(a)
    }
    fn commit_save(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) {
        match self.save_arguments(path, cx) {
            Ok(arguments) => {
                let request = Request {
                    command: "file.commit_and_save".into(),
                    arguments,
                    expected_revision: Some(if self.has_draft(cx) {
                        self.edit_revision
                    } else {
                        self.snapshot.revision
                    }),
                };
                let _ = self.client.sender.send(Message::Execute(request));
            }
            Err(e) => {
                self.status = e;
                cx.notify();
            }
        }
    }
    fn save(&mut self, _: &Save, window: &mut Window, cx: &mut Context<Self>) {
        if self.snapshot.path.is_some() {
            self.commit_save(None, cx);
        } else {
            self.save_as(&SaveAs, window, cx);
        }
    }
    fn save_as(&mut self, _: &SaveAs, window: &mut Window, cx: &mut Context<Self>) {
        let directory = self
            .snapshot
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let prompt = cx.prompt_for_new_path(&directory, Some("design.orch"));
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(mut path))) = prompt.await {
                if path.extension().is_none() {
                    path.set_extension("orch");
                }
                let _ = this.update_in(cx, |this, _, cx| this.commit_save(Some(path), cx));
            }
        })
        .detach();
    }
    fn new_project(&mut self, _: &New, _: &mut Window, cx: &mut Context<Self>) {
        if self.has_draft(cx) {
            self.status = "Apply the editor draft before creating a project".into();
            cx.notify();
            return;
        }
        self.client.send("file.new", json!({}));
    }
    fn undo(&mut self, _: &Undo, _: &mut Window, _: &mut Context<Self>) {
        self.client.send("edit.undo", json!({}));
    }
    fn redo(&mut self, _: &Redo, _: &mut Window, _: &mut Context<Self>) {
        self.client.send("edit.redo", json!({}));
    }
    fn delete(&mut self, _: &DeleteSelection, _: &mut Window, cx: &mut Context<Self>) {
        if self.has_draft(cx) {
            self.status = "Apply the editor draft before deleting".into();
            cx.notify();
            return;
        }
        for id in &self.selected {
            if id != &self.snapshot.document.root_node_id {
                self.client
                    .send("threadwork.delete_entity", json!({"nodeId":id}));
            }
        }
    }
    fn select_mode(&mut self, _: &SelectMode, _: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        self.mode = Mode::Select;
        self.link_source = None;
        cx.notify();
    }
    fn node_mode(&mut self, _: &NodeMode, _: &mut Window, cx: &mut Context<Self>) {
        self.mode = Mode::Node;
        cx.notify();
    }
    fn link_mode(&mut self, _: &LinkMode, _: &mut Window, cx: &mut Context<Self>) {
        self.mode = Mode::Link;
        self.link_source = None;
        cx.notify();
    }
    fn fit(&mut self, _: &Fit, _: &mut Window, cx: &mut Context<Self>) {
        if !self.scene.nodes.is_empty() {
            let min_x = self
                .scene
                .nodes
                .values()
                .map(|n| n.2.x)
                .fold(f64::INFINITY, f64::min);
            let min_y = self
                .scene
                .nodes
                .values()
                .map(|n| n.2.y)
                .fold(f64::INFINITY, f64::min);
            let max_x = self
                .scene
                .nodes
                .values()
                .map(|n| n.2.x + n.2.width)
                .fold(f64::NEG_INFINITY, f64::max);
            let max_y = self
                .scene
                .nodes
                .values()
                .map(|n| n.2.y + n.2.height)
                .fold(f64::NEG_INFINITY, f64::max);
            let b = self.bounds.get();
            let w = f64::from(f32::from(b.size.width));
            let h = f64::from(f32::from(b.size.height));
            self.viewport.zoom = ((w - 80.) / (max_x - min_x))
                .min((h - 80.) / (max_y - min_y))
                .clamp(0.1, 2.);
            self.viewport.x = 40. - min_x * self.viewport.zoom;
            self.viewport.y = 40. - min_y * self.viewport.zoom;
        }
        self.persist_ui(cx);
        cx.notify();
    }
    fn local(&self, p: Point<Pixels>) -> [f64; 2] {
        let b = self.bounds.get();
        [
            f64::from(f32::from(p.x - b.origin.x)),
            f64::from(f32::from(p.y - b.origin.y)),
        ]
    }
    fn down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.focus.focus(window);
        let local = self.local(event.position);
        if event.button == MouseButton::Middle || event.button == MouseButton::Right {
            self.pan_start = Some((local, self.viewport));
            return;
        }
        if self.has_draft(cx) {
            self.status = "Apply the editor draft before changing selection".into();
            cx.notify();
            return;
        }
        let world = self.viewport.world(local);
        let tolerance = 6. / self.viewport.zoom;
        let hits = self.scene.index.query(Rect {
            x: world[0] - tolerance,
            y: world[1] - tolerance,
            width: tolerance * 2.,
            height: tolerance * 2.,
        });
        let hit = hits
            .iter()
            .find(|id| {
                self.scene
                    .nodes
                    .get(*id)
                    .is_some_and(|n| n.2.contains(world))
            })
            .cloned()
            .or_else(|| {
                if self.mode != Mode::Select {
                    return None;
                }
                hits.iter()
                    .find(|id| {
                        self.scene.links.get(*id).is_some_and(|(_, points)| {
                            points.windows(2).any(|pair| {
                                wirecraft_core::viewport::distance_to_segment(
                                    world, pair[0], pair[1],
                                ) <= tolerance
                            })
                        })
                    })
                    .cloned()
            });
        match self.mode {
            Mode::Node | Mode::Type => {
                self.client.send("threadwork.create_entity",json!({"parentId":self.parent,"name":if self.mode==Mode::Type{"New type"}else{"New processor"},"kind":if self.mode==Mode::Type{"Type"}else{"Processor"},"attributes":{"layout":{"x":world[0],"y":world[1]}}}));
            }
            Mode::Link => {
                if let Some(id) = hit {
                    if let Some(source) = self.link_source.take() {
                        self.client.send("threadwork.create_link",json!({"name":"packet","sourceNodeId":source,"targetNodeId":id,"sourcePortName":"out","targetPortName":"in"}));
                    } else {
                        self.link_source = Some(id);
                        self.status = "Choose the target entity".into();
                    }
                }
            }
            Mode::Select => {
                if let Some(id) = hit {
                    if event.modifiers.shift {
                        if !self.selected.insert(id.clone()) {
                            self.selected.remove(&id);
                        }
                    } else if !self.selected.contains(&id) {
                        self.selected = BTreeSet::from([id.clone()]);
                    }
                    self.load_editor(window, cx);
                    if event.click_count == 2 {
                        if self.snapshot.document.nodes[&id].is_composite() {
                            self.parent = id;
                            self.rebuild();
                            self.viewport = Viewport::default();
                        } else {
                            self.tab = "Editor".into();
                        }
                    } else {
                        if self.scene.nodes.contains_key(&id) {
                            self.drag_start = Some(world);
                            self.drag_delta = [0., 0.];
                        }
                    }
                } else {
                    if !event.modifiers.shift {
                        self.selected.clear();
                    }
                    self.selection_start = Some(world);
                }
            }
        }
        cx.notify();
    }
    fn moved(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let local = self.local(event.position);
        if let Some((start, view)) = self.pan_start {
            self.viewport.x = view.x + local[0] - start[0];
            self.viewport.y = view.y + local[1] - start[1];
            cx.notify();
        } else if let Some(start) = self.drag_start {
            let world = self.viewport.world(local);
            self.drag_delta = [world[0] - start[0], world[1] - start[1]];
            cx.notify();
        } else if let Some(start) = self.selection_start {
            let end = self.viewport.world(local);
            let rect = Rect {
                x: start[0].min(end[0]),
                y: start[1].min(end[1]),
                width: (end[0] - start[0]).abs(),
                height: (end[1] - start[1]).abs(),
            };
            self.selected = self
                .scene
                .index
                .query(rect)
                .into_iter()
                .filter(|id| self.scene.nodes.contains_key(id))
                .collect();
            cx.notify();
        }
    }
    fn up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.drag_start.take().is_some()
            && (self.drag_delta[0].abs() > 0.5 || self.drag_delta[1].abs() > 0.5)
        {
            let changes=self.selected.iter().filter_map(|id|self.snapshot.document.nodes.get(id)).map(|n|json!({"nodeId":n.id,"patch":{"layout":{"x":n.layout.x+self.drag_delta[0],"y":n.layout.y+self.drag_delta[1]}}})).collect::<Vec<_>>();
            self.client
                .send("entities.patch", json!({"changes":changes}));
        }
        self.drag_delta = [0., 0.];
        self.pan_start = None;
        self.selection_start = None;
        self.persist_ui(cx);
        cx.notify();
    }
    fn wheel(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(20.));
        if event.modifiers.control {
            self.viewport.zoom_at(
                self.local(event.position),
                (f64::from(f32::from(delta.y)) * 0.005).exp(),
            );
        } else {
            self.viewport.x += f64::from(f32::from(delta.x));
            self.viewport.y += f64::from(f32::from(delta.y));
        }
        self.persist_ui(cx);
        cx.notify();
    }
    fn copy(&mut self, _: &CopyEntities, _: &mut Window, cx: &mut Context<Self>) {
        if !self.has_draft(cx) {
            self.client
                .send("entities.copy", json!({"nodeIds":self.selected}));
        }
    }
    fn cut(&mut self, _: &CutEntities, _: &mut Window, cx: &mut Context<Self>) {
        if !self.has_draft(cx) {
            self.client
                .send("entities.cut", json!({"nodeIds":self.selected}));
        }
    }
    fn paste(&mut self, _: &PasteEntities, _: &mut Window, cx: &mut Context<Self>) {
        if self.has_draft(cx) {
            self.status = "Apply the editor draft before pasting".into();
            cx.notify();
            return;
        }
        if let Some(text) = cx.read_from_clipboard().and_then(|i| i.text()) {
            match serde_json::from_str::<Value>(&text) {
                Ok(fragment) => self.client.send(
                    "entities.paste",
                    json!({"fragment":fragment,"parentId":self.parent}),
                ),
                Err(_) => {
                    self.status = "Clipboard does not contain a Wirecraft graph fragment".into();
                    cx.notify();
                }
            }
        }
    }
    fn export_dialog(&mut self, format: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let directory = self
            .snapshot
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let prompt = cx.prompt_for_new_path(&directory, Some(&format!("diagram.{format}")));
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(mut path))) = prompt.await {
                if path.extension().is_none() {
                    path.set_extension(format);
                }
                let _ = this.update_in(cx, |this, _, _| {
                    this.client.send(
                        "export.diagram",
                        json!({"nodeId":this.parent,"path":path,"format":format}),
                    )
                });
            }
        })
        .detach();
    }
    fn artifact_dialog(&mut self, kind: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        if self.has_draft(cx) {
            self.status = "Apply the editor draft before generating or saving an archetype".into();
            cx.notify();
            return;
        }
        let directory = self
            .snapshot
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let prompt = cx.prompt_for_new_path(
            &directory,
            Some(if kind == "archetype" {
                "archetype.orch"
            } else {
                "generated-project"
            }),
        );
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(path))) = prompt.await {
                let _ = this.update_in(cx, |this, _, _| {
                    if kind == "archetype" {
                        if let Some(id) = this.selected.first() {
                            this.client
                                .send("archetypes.save", json!({"nodeId":id,"path":path}));
                        }
                    } else {
                        this.client.send(
                            if kind == "c17" {
                                "compile.c17"
                            } else {
                                "generate.files"
                            },
                            json!({"nodeId":this.parent,"outputDir":path}),
                        );
                    }
                });
            }
        })
        .detach();
    }
    fn import_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import a .orch archetype".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = prompt.await
                && let Some(path) = paths.first()
            {
                let _ = this.update_in(cx, |this, _, _| {
                    this.client.send(
                        "archetypes.import_file",
                        json!({"path":path,"parentId":this.parent}),
                    )
                });
            }
        })
        .detach();
    }
    fn reveal_diagnostic(
        &mut self,
        diagnostic: &Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.has_draft(cx) {
            self.status = "Apply the editor draft before revealing another diagnostic".into();
            cx.notify();
            return;
        }
        let source = &diagnostic["source"];
        let Some(id) = source["nodeId"].as_str() else {
            return;
        };
        if !self.snapshot.document.nodes.contains_key(id) {
            return;
        }
        self.selected = BTreeSet::from([id.into()]);
        let section = source["section"].as_str().unwrap_or("source");
        self.section = if [
            "source",
            "initialization",
            "specification",
            "tests",
            "aiInstructions",
        ]
        .contains(&section)
        {
            section.into()
        } else {
            "source".into()
        };
        self.tab = "Editor".into();
        self.load_editor(window, cx);
        if ![
            "source",
            "initialization",
            "specification",
            "tests",
            "aiInstructions",
        ]
        .contains(&section)
        {
            self.properties.update(cx, |editor, cx| {
                editor.set_cursor_position(lsp_types::Position::new(0, 0), window, cx)
            });
            self.status = format!(
                "{} · {section}:{}:{}",
                self.snapshot.document.qualified_name(id),
                source["line"],
                source["column"]
            );
            self.persist_ui(cx);
            cx.notify();
            return;
        }
        let row = source["line"].as_u64().unwrap_or(1).saturating_sub(1) as usize;
        let row = row.min(
            self.editor
                .read(cx)
                .value()
                .split('\n')
                .count()
                .saturating_sub(1),
        );
        let column = source["column"].as_u64().unwrap_or(1).saturating_sub(1) as usize;
        let text = self.editor.read(cx).value();
        let encoding = diagnostic["positionEncoding"].as_str().unwrap_or("utf-16");
        let character = wirecraft_runtime::lsp::offset(text.as_ref(), row, column, encoding)
            .ok()
            .and_then(|offset| {
                wirecraft_runtime::lsp::position(text.as_ref(), offset, "utf-32").ok()
            })
            .and_then(|p| p["character"].as_u64())
            .unwrap_or(0) as u32;
        self.editor.update(cx, |editor, cx| {
            editor.set_cursor_position(lsp_types::Position::new(row as u32, character), window, cx)
        });
        self.status = format!(
            "{} · {section}:{}:{}",
            self.snapshot.document.qualified_name(id),
            row + 1,
            column + 1
        );
        self.persist_ui(cx);
        cx.notify();
    }
    fn analyze(&mut self, cpp: bool, build: bool, cx: &mut Context<Self>) {
        if self.section != "source" {
            self.status = "Select the Source section in Editor first".into();
            cx.notify();
            return;
        }
        if let Some(id) = self.selected.first() {
            let _ = self.client.sender.send(Message::Analyze {
                node_id: id.clone(),
                language: if cpp { "cpp".into() } else { "c".into() },
                text: self.editor.read(cx).value().to_string(),
                build,
            });
            self.status = "Running language tool…".into();
            cx.notify();
        }
    }
    fn tool(&mut self, command: &str) {
        self.client.send(command, json!({}));
    }
}
fn button(id: &'static str, label: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id)
        .px_3()
        .py_1()
        .rounded_sm()
        .bg(rgb(0x242c36))
        .hover(|s| s.bg(rgb(0x34414d)))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap_2()
        .when(!icons::for_button(id).is_empty(), |b| {
            b.child(icons::icon(icons::for_button(id)))
        })
        .child(label.into())
}
impl Render for Wirecraft {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = format!(
            "{}{}",
            self.snapshot.document.project_name(),
            if self.snapshot.dirty { " •" } else { "" }
        );
        let toolbar = div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .bg(rgb(0x171d25))
            .child(div().text_lg().text_color(rgb(0x75d4d6)).child("Wirecraft"))
            .child(div().flex_1().child(title))
            .child(
                button("new", "New")
                    .on_click(cx.listener(|this, _, w, cx| this.new_project(&New, w, cx))),
            )
            .child(
                button("open", "Open")
                    .on_click(cx.listener(|this, _, w, cx| this.open(&Open, w, cx))),
            )
            .child(
                button("save", "Save")
                    .on_click(cx.listener(|this, _, w, cx| this.save(&Save, w, cx))),
            )
            .child(
                button("save-as", "Save as")
                    .on_click(cx.listener(|this, _, w, cx| this.save_as(&SaveAs, w, cx))),
            )
            .child(
                button("undo", "Undo")
                    .on_click(cx.listener(|this, _, w, cx| this.undo(&Undo, w, cx))),
            )
            .child(
                button("redo", "Redo")
                    .on_click(cx.listener(|this, _, w, cx| this.redo(&Redo, w, cx))),
            );
        let mut tabs = div().flex().gap_2().px_3().py_2();
        for tab in [
            "Designer",
            "Editor",
            "Project Management",
            "Analysis",
            "Archetypes",
            "Export",
            "Tools",
            "MCP",
        ] {
            tabs = tabs.child(
                button(tab, tab)
                    .when(self.tab == tab, |b| b.bg(rgb(0x255452)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.tab = tab.into();
                        if tab == "Analysis" {
                            this.tool("analysis.report");
                        }
                        if tab == "Archetypes" {
                            this.tool("archetypes.list");
                        }
                        this.persist_ui(cx);
                        cx.notify();
                    })),
            );
        }
        let tree = uniform_list(
            "hierarchy",
            self.tree.len(),
            cx.processor(|this, range: std::ops::Range<usize>, _window, _cx| {
                range
                    .map(|i| {
                        let (id, depth) = this.tree[i].clone();
                        let name = this.snapshot.document.nodes[&id].name.clone();
                        div()
                            .id(i)
                            .h(px(28.))
                            .pl(px(10. + depth as f32 * 14.))
                            .pr_2()
                            .overflow_hidden()
                            .when(this.selected.contains(&id), |b| b.bg(rgb(0x253f45)))
                            .cursor_pointer()
                            .on_click(_cx.listener(move |this, _, window, cx| {
                                this.select(id.clone(), window, cx)
                            }))
                            .child(name)
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .flex_1();
        let sidebar = div()
            .w(px(250.))
            .flex_none()
            .flex()
            .flex_col()
            .bg(rgb(0x191f28))
            .child(
                div()
                    .p_3()
                    .text_color(rgb(0x7f9bae))
                    .child("Entity Hierarchy Tree"),
            )
            .child(tree);
        let mut content = div().flex_1().min_w_0().min_h_0().flex().flex_col();
        if self.tab == "Designer" {
            let scene = self.scene.clone();
            let bounds = self.bounds.clone();
            let view = self.viewport;
            let selected = self.selected.clone();
            let drag = self.drag_delta;
            let parent_name = self.snapshot.document.qualified_name(&self.parent);
            content = content
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .p_2()
                        .child(button("select", "Select").on_click(
                            cx.listener(|this, _, w, cx| this.select_mode(&SelectMode, w, cx)),
                        ))
                        .child(button("node", "Node").on_click(
                            cx.listener(|this, _, w, cx| this.node_mode(&NodeMode, w, cx)),
                        ))
                        .child(
                            button("type", "Type").on_click(cx.listener(|this, _, _, cx| {
                                this.mode = Mode::Type;
                                cx.notify();
                            })),
                        )
                        .child(button("link", "Link").on_click(
                            cx.listener(|this, _, w, cx| this.link_mode(&LinkMode, w, cx)),
                        ))
                        .child(
                            button("fit", "Fit")
                                .on_click(cx.listener(|this, _, w, cx| this.fit(&Fit, w, cx))),
                        )
                        .child(button("up", "Up").on_click(cx.listener(|this, _, _, cx| {
                            if let Some(p) =
                                this.snapshot.document.nodes[&this.parent].parent_id.clone()
                            {
                                this.parent = p;
                                this.rebuild();
                                this.viewport = Viewport::default();
                                this.persist_ui(cx);
                                cx.notify();
                            }
                        })))
                        .child(
                            div()
                                .px_2()
                                .child(format!("{parent_name} · {:?}", self.mode)),
                        ),
                )
                .child(div().flex().gap_2().p_2()
                   .child(button("copy-graph","Copy").on_click(cx.listener(|this,_,w,cx|this.copy(&CopyEntities,w,cx))))
                   .child(button("cut-graph","Cut").on_click(cx.listener(|this,_,w,cx|this.cut(&CutEntities,w,cx))))
                   .child(button("paste-graph","Paste").on_click(cx.listener(|this,_,w,cx|this.paste(&PasteEntities,w,cx))))
                   .child(button("move-here","Move selection here").on_click(cx.listener(|this,_,_,_|{for id in &this.selected{this.client.send("threadwork.move_entity",json!({"nodeId":id,"parentId":this.parent}));}})))
                   .child(button("align-left","Align left").on_click(cx.listener(|this,_,_,_|this.client.send("entities.align",json!({"nodeIds":this.selected,"operation":"left"})))))
                   .child(button("distribute-x","Distribute horizontally").on_click(cx.listener(|this,_,_,_|this.client.send("entities.align",json!({"nodeIds":this.selected,"operation":"distribute-horizontal"}))))))
                .child(
                    div()
                        .id("canvas")
                        .flex_1()
                        .min_h_0()
                        .overflow_hidden()
                        .bg(rgb(0x10161e))
                        .on_mouse_down(MouseButton::Left, cx.listener(Self::down))
                        .on_mouse_down(MouseButton::Middle, cx.listener(Self::down))
                        .on_mouse_down(MouseButton::Right, cx.listener(Self::down))
                        .on_mouse_move(cx.listener(Self::moved))
                        .on_mouse_up(MouseButton::Left, cx.listener(Self::up))
                        .on_mouse_up_out(MouseButton::Left, cx.listener(Self::up))
                        .on_mouse_up(MouseButton::Middle, cx.listener(Self::up))
                        .on_mouse_up(MouseButton::Right, cx.listener(Self::up))
                        .on_scroll_wheel(cx.listener(Self::wheel))
                        .child(
                            canvas(
                                move |b, _, _| {
                                    bounds.set(b);
                                    b
                                },
                                move |b, _, window, cx| {
                                    scene.paint(b, view, &selected, drag, window, cx)
                                },
                            )
                            .size_full(),
                        ),
                );
        } else if self.tab == "Editor" {
            content = content.child(self.render_editor(cx));
        } else if self.tab == "Project Management" {
            content = content.child(self.render_management(cx));
        } else if self.tab == "Analysis" {
            content = content.child(self.render_analysis(cx));
        } else if self.tab == "Archetypes" {
            content = content.child(self.render_archetypes(cx));
        } else if self.tab == "Export" {
            content=content.child(div().p_4().child("Export the focused graph as a free-size drawing. ISO sheets, rolls and poster tiling remain pending."))
              .child(div().flex().gap_2().p_3().child(button("generate-c17","Generate C17 topology").on_click(cx.listener(|this,_,window,cx|this.artifact_dialog("c17",window,cx)))).child(button("generate-files","Generate literal file entities").on_click(cx.listener(|this,_,window,cx|this.artifact_dialog("files",window,cx)))).child(button("save-archetype","Save selected subtree as archetype").on_click(cx.listener(|this,_,window,cx|this.artifact_dialog("archetype",window,cx)))))
              .child(div().flex().gap_2().p_3().child(button("export-svg","SVG").on_click(cx.listener(|this,_,window,cx|this.export_dialog("svg",window,cx))))
              .child(button("export-png","PNG").on_click(cx.listener(|this,_,window,cx|this.export_dialog("png",window,cx))))
              .child(button("export-pdf","PDF").on_click(cx.listener(|this,_,window,cx|this.export_dialog("pdf",window,cx))))
              .child(button("copy-fiches","Copy component fiches · Markdown").on_click(cx.listener(|this,_,_,_|this.client.send("export.markdown",json!({"nodeIds":this.selected}))))));
        } else if self.tab == "Tools" {
            content=content.child(div().p_3().child("Check a standalone source section or analyze the C17 graph. Graph diagnostics include the original entity, text section and line."))
             .child(div().flex().gap_2().p_3()
              .child(button("clangd-c","clangd · C").on_click(cx.listener(|this,_,_,cx|this.analyze(false,false,cx))))
              .child(button("clangd-cpp","clangd · C++").on_click(cx.listener(|this,_,_,cx|this.analyze(true,false,cx))))
              .child(button("build-c","Compiler check · C").on_click(cx.listener(|this,_,_,cx|this.analyze(false,true,cx))))
              .child(button("diagnostics","Refresh diagnostics").on_click(cx.listener(|this,_,_,_|this.tool("lsp.diagnostics"))))
              .child(button("jobs","Build jobs").on_click(cx.listener(|this,_,_,_|this.tool("jobs.list"))))
              .child(button("analyze-c17","Analyze C17 graph").on_click(cx.listener(|this,_,_,_|this.tool("editor.analyze_generated")))).child(button("stop-graph","Stop graph analyzer").on_click(cx.listener(|this,_,_,_|this.tool("editor.stop_generated")))).child(button("stop-lsp","Stop standalone").on_click(cx.listener(|this,_,_,_|this.tool("lsp.stop")))))
             .child(div().id("tool-output").flex_1().overflow_y_scroll().p_4()
                .children(self.generated_diagnostics.iter().enumerate().filter(|(_,d)|d["source"].is_object()).map(|(i,d)|{
                    let diagnostic=d.clone();let source=&d["source"];let label=format!("{}:{}:{} · {}",source["nodeId"].as_str().unwrap_or(""),source["line"],source["column"],d["message"].as_str().unwrap_or(""));
                    div().id(i).p_2().mb_2().bg(rgb(0x3c2831)).cursor_pointer().on_click(cx.listener(move|this,_,window,cx|this.reveal_diagnostic(&diagnostic,window,cx))).child(label)
                })).child(self.output.clone()));
        } else {
            let config = json!({"mcpServers":{"wirecraft":{"url":format!("{}/mcp",self.endpoint),"headers":{"Authorization":format!("Bearer {}",self.token)}}}});
            content=content.child(div().p_4().child("Local MCP server · owned by this desktop session")).child(div().p_4().child(format!("{}/mcp",self.endpoint))).child(button("copy-mcp","Copy MCP client configuration").on_click(move|_,_,cx|cx.write_to_clipboard(ClipboardItem::new_string(serde_json::to_string_pretty(&config).unwrap())))).child(div().p_4().child("Clients share this open design. Closing Wirecraft shuts down this server. The previous Threadworks server runs independently."));
        }
        div().key_context("Wirecraft").track_focus(&self.focus).on_action(cx.listener(Self::open)).on_action(cx.listener(Self::save)).on_action(cx.listener(Self::save_as)).on_action(cx.listener(Self::new_project)).on_action(cx.listener(Self::undo)).on_action(cx.listener(Self::redo)).on_action(cx.listener(Self::delete)).on_action(cx.listener(Self::select_mode)).on_action(cx.listener(Self::node_mode)).on_action(cx.listener(Self::link_mode)).on_action(cx.listener(Self::fit)).on_action(cx.listener(Self::apply)).on_action(cx.listener(Self::copy)).on_action(cx.listener(Self::cut)).on_action(cx.listener(Self::paste)).on_action(cx.listener(Self::palette)).on_action(cx.listener(Self::show_export)).on_action(cx.listener(Self::show_options)).on_action(|_:&Quit,_,cx|cx.quit()).relative().size_full().flex().flex_col().bg(rgb(0x10161e)).text_color(rgb(0xe1e9ef)).text_sm().child(self.render_menubar(cx)).child(toolbar).child(tabs).child(div().flex().flex_1().min_h_0().when(["Designer","Editor","Tools"].contains(&self.tab.as_str()), |b| b.child(sidebar)).child(content)).child(div().px_3().py_2().bg(rgb(0x171d25)).child(format!("{}   |   rev {}   |   {:.0}%   |   Scroll: pan · Ctrl scroll: zoom · Double click: enter/edit",self.status,self.snapshot.revision,self.viewport.zoom*100.))).children(self.render_menu(cx))
    }
}
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--backend") {
        return tokio::runtime::Runtime::new()?
            .block_on(wirecraft_runtime::cli::run(args.into_iter().skip(1)));
    }
    let mut state_dir = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join(".local/state")
        })
        .join("wirecraft");
    let mut project = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--state-dir" => {
                i += 1;
                state_dir = PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| anyhow::anyhow!("--state-dir needs a path"))?,
                );
            }
            "--help" => {
                println!("wirecraft-desktop [PROJECT.orch] [--state-dir DIRECTORY]");
                return Ok(());
            }
            s if s.starts_with('-') => anyhow::bail!("Unknown option {s}"),
            s => project = Some(PathBuf::from(s)),
        }
        i += 1;
    }
    let session = std::fs::read(state_dir.join("ui.json"))
        .ok()
        .and_then(|s| serde_json::from_slice(&s).ok())
        .unwrap_or_default();
    let settings = wirecraft_runtime::config::Settings::load(&state_dir.join("settings.json"))?;
    let indent = settings.editor_indent_spaces;
    let (client, rx, worker) = Client::start(state_dir.clone(), settings, project);
    gpui::Application::new().run(move |cx| {
        gpui_component::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        cx.bind_keys([
            KeyBinding::new("ctrl-shift-p", CommandPalette, Some("Wirecraft")),
            KeyBinding::new("ctrl-p", ShowExport, Some("Wirecraft")),
            KeyBinding::new("ctrl-comma", ShowOptions, Some("Wirecraft")),
            KeyBinding::new("ctrl-o", Open, Some("Wirecraft")),
            KeyBinding::new("ctrl-c", CopyEntities, Some("Wirecraft")),
            KeyBinding::new("ctrl-x", CutEntities, Some("Wirecraft")),
            KeyBinding::new("ctrl-v", PasteEntities, Some("Wirecraft")),
            KeyBinding::new("ctrl-q", Quit, Some("Wirecraft")),
            KeyBinding::new("ctrl-s", Save, Some("Wirecraft")),
            KeyBinding::new("ctrl-shift-s", SaveAs, Some("Wirecraft")),
            KeyBinding::new("ctrl-n", New, Some("Wirecraft")),
            KeyBinding::new("ctrl-z", Undo, Some("Wirecraft")),
            KeyBinding::new("ctrl-y", Redo, Some("Wirecraft")),
            KeyBinding::new("ctrl-shift-z", Redo, Some("Wirecraft")),
            KeyBinding::new("delete", DeleteSelection, Some("Wirecraft")),
            KeyBinding::new("escape", SelectMode, Some("Wirecraft")),
            KeyBinding::new("ctrl-enter", Apply, Some("Wirecraft")),
        ]);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(1400.), px(900.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title("Wirecraft");
                let view = cx.new(|cx| Wirecraft::new(client, rx, session, indent, window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("open Wirecraft window");
        cx.activate(true);
    });
    let _ = worker.join();
    Ok(())
}
