mod chrome;
mod client;
mod completion;
mod fonts;
mod icons;
mod inspector;
mod inspector_widgets;
mod options;
mod reports;
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
        ShowOptions,
        Complete
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
    document_id: String,
    viewport: Viewport,
    parent: String,
    selected: Option<String>,
    selected_ids: Vec<String>,
    collapsed: BTreeSet<String>,
    sidebar_width: f32,
    inspector_width: f32,
    tab: String,
    section: String,
    draft: Option<Draft>,
    export_settings: reports::ExportState,
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
    #[serde(default)]
    tables: Option<Value>,
}
struct Wirecraft {
    connected_document: bool,
    initial_document_id: String,
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
    editor_bounds: Rc<Cell<Bounds<Pixels>>>,
    drag_start: Option<[f64; 2]>,
    drag_delta: [f64; 2],
    drag_ids: BTreeSet<String>,
    resize_start: Option<(String, [f64; 2], Rect)>,
    selection_end: Option<[f64; 2]>,
    pan_start: Option<([f64; 2], Viewport)>,
    selection_start: Option<[f64; 2]>,
    focus: FocusHandle,
    status: String,
    endpoint: String,
    token: String,
    output: String,
    tree: Vec<(String, usize)>,
    collapsed: BTreeSet<String>,
    sidebar_width: f32,
    inspector_width: f32,
    panel_drag: Option<(bool, f32, f32)>,
    archetypes: Vec<Value>,
    generated_diagnostics: Vec<Value>,
    generated_revision: u64,
    inspector: Vec<workbench::FormField>,
    completion: Option<completion::Pending>,
    completion_generation: u64,
    function_header: String,
    tables: inspector::Tables,
    advanced: bool,
    editor_committed: bool,
    editor_request_pending: bool,
    submitted_editor: Option<Value>,
    submitted_revision: Option<u64>,
    menu: Option<String>,
    query: Entity<InputState>,
    palette_index: usize,
    palette_scroll: ScrollHandle,
    report: Value,
    analysis_layer: String,
    analysis_column: usize,
    options: options::OptionsEditor,
    font_choices: Vec<String>,
    type_choice: Option<(usize, bool)>,
    mcp_status: Value,
    export_settings: reports::ExportState,
    export_image: Option<Arc<Image>>,
    export_page: usize,
    export_pages: usize,
    export_generation: u64,
    export_summary: String,
    printers: Vec<Value>,
    management_backlog: bool,
    archetype_selected: Option<String>,
    archetype_preview: Option<Arc<scene::Scene>>,
}
impl Wirecraft {
    fn new(
        client: Client,
        rx: async_channel::Receiver<Update>,
        session: UiSession,
        settings: wirecraft_runtime::config::Settings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Entity name"));
        let editor = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("plain")
                .tab_size(TabSize {
                    tab_size: settings.editor_indent_spaces,
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
        let options = options::OptionsEditor::new(settings, window, cx);
        let inspector = workbench::make_fields(window, cx);
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search commands…"));
        cx.subscribe_in(&query, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.submit_command(window, cx);
            } else if matches!(event, InputEvent::Change) {
                this.palette_index = 0;
                this.palette_scroll.scroll_to_item(0);
            }
            cx.notify();
        })
        .detach();
        let focus = cx.focus_handle();
        focus.focus(window);
        let this = Self {
            connected_document: false,
            initial_document_id: session.document_id,
            client,
            snapshot,
            scene: Arc::new(scene::Scene::new(&Document::new(""), "root")),
            viewport: session.viewport,
            parent: if session.parent.is_empty() {
                "root".into()
            } else {
                session.parent
            },
            selected: if session.selected_ids.is_empty() {
                session.selected.into_iter().collect()
            } else {
                session.selected_ids.into_iter().collect()
            },
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
            editor_bounds: Rc::new(Cell::new(Bounds::default())),
            drag_start: None,
            drag_delta: [0., 0.],
            drag_ids: BTreeSet::new(),
            resize_start: None,
            selection_end: None,
            pan_start: None,
            selection_start: None,
            focus,
            status: "Starting local backend…".into(),
            endpoint: String::new(),
            token: String::new(),
            output: String::new(),
            tree: vec![],
            collapsed: session.collapsed,
            sidebar_width: if session.sidebar_width > 0. {
                session.sidebar_width
            } else {
                250.
            },
            inspector_width: if session.inspector_width > 0. {
                session.inspector_width
            } else {
                360.
            },
            panel_drag: None,
            archetypes: vec![],
            generated_diagnostics: vec![],
            generated_revision: 0,
            inspector,
            completion: None,
            completion_generation: 0,
            function_header: String::new(),
            tables: inspector::Tables::default(),
            advanced: false,
            editor_committed: false,
            editor_request_pending: false,
            submitted_editor: None,
            submitted_revision: None,
            menu: None,
            query,
            palette_index: 0,
            palette_scroll: ScrollHandle::new(),
            report: Value::Null,
            analysis_layer: "principal".into(),
            analysis_column: 0,
            options,
            font_choices: vec![],
            type_choice: None,
            mcp_status: json!({"enabled":true}),
            export_settings: session.export_settings,
            export_image: None,
            export_page: 0,
            export_pages: 0,
            export_generation: 0,
            export_summary: String::new(),
            printers: vec![],
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
                    this.completion = None;
                    this.persist_ui(cx);
                    cx.notify();
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
                self.tool("mcp.status");
            }
            Update::Error { message, commit } => {
                self.completion = None;
                self.status = message;
                if commit {
                    self.editor_request_pending = false;
                    self.submitted_editor = None;
                    self.submitted_revision = None;
                }
            }
            Update::Result {
                command,
                value,
                revision,
                commit,
            } => {
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
                    && commit
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
                        "revision",
                        "diagnostics",
                        "statusChanges",
                    ] {
                        props.as_object_mut().unwrap().remove(k);
                    }
                    if value["entity"]["id"] == self.snapshot.document.root_node_id {
                        props["masterRevision"] = value
                            .get("masterRevision")
                            .cloned()
                            .unwrap_or_else(|| self.snapshot.document.master_revision.clone());
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
                    self.archetype_preview = Some(Arc::new(
                        scene::Scene::new(&document, &document.root_node_id)
                            .with_appearance(self.options.saved.appearance.clone()),
                    ));
                }
                if command == "editor.complete" {
                    self.accept_completions(&value, cx);
                }
                if command == "editor.context"
                    && value["revision"].as_u64() == Some(self.snapshot.revision)
                    && value["nodeId"].as_str() == self.editor_entity.as_deref()
                    && value["section"] == self.section
                {
                    self.function_header = value["header"].as_str().unwrap_or("").into();
                }
                if command == "export.preview"
                    && value["generation"].as_u64() == Some(self.export_generation)
                {
                    self.export_pages = value["pages"].as_u64().unwrap_or(0) as usize;
                    self.export_summary = reports::preview_summary(&value["summary"]);
                    if value["summary"]["clipped"] == true {
                        self.status="Drawing exceeds the fixed sheet. Enable split or increase scale denominator.".into();
                    }
                    use base64::Engine;
                    self.export_image = value["pngBase64"]
                        .as_str()
                        .and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok())
                        .map(|bytes| Arc::new(Image::from_bytes(ImageFormat::Png, bytes)));
                }
                if command == "settings.replace" {
                    self.apply_settings(value.clone(), window, cx);
                }
                if command == "printers.list" {
                    self.printers = value["printers"].as_array().cloned().unwrap_or_default();
                }
                if command == "mcp.status" {
                    self.mcp_status = value.clone();
                }
                if command == "mcp.set_enabled" {
                    self.tool("mcp.status");
                }
                if command == "analysis.report" {
                    self.report = value.clone();
                }
                if commit {
                    self.editor_committed = true;
                    self.submitted_revision = revision;
                }
                if command == "archetypes.list" {
                    self.archetypes = value["archetypes"].as_array().cloned().unwrap_or_default();
                }
                if command == "ai.submit"
                    && let Some(prompt) = value["prompt"].as_str()
                {
                    cx.write_to_clipboard(ClipboardItem::new_string(prompt.to_string()));
                    self.status = "Complete assistance prompt copied to clipboard".into();
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
                let document_changed = (self.connected_document
                    && snapshot.document.id != self.snapshot.document.id)
                    || (!self.connected_document
                        && !self.initial_document_id.is_empty()
                        && self.initial_document_id != snapshot.document.id);
                self.connected_document = true;
                if snapshot.document.id != self.snapshot.document.id {
                    self.editor_entity = None;
                }
                self.snapshot = Arc::new(*snapshot);
                if document_changed {
                    self.parent = self.snapshot.document.root_node_id.clone();
                    self.selected.clear();
                    self.viewport = Viewport::default();
                }
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
                if document_changed {
                    self.fit(&Fit, window, cx);
                }
                if let Some(mut draft) = self.pending_draft.take()
                    && draft.document_id == self.snapshot.document.id
                    && self.snapshot.document.nodes.contains_key(&draft.node_id)
                {
                    self.selected = BTreeSet::from([draft.node_id.clone()]);
                    self.section = draft.section.clone();
                    self.load_editor(window, cx);
                    if let (Ok(mut baseline), Ok(mut properties)) = (
                        serde_json::from_str::<Value>(&draft.base_properties),
                        serde_json::from_str::<Value>(&draft.properties),
                    ) {
                        for key in ["revision", "diagnostics", "statusChanges"] {
                            if baseline.get(key) == properties.get(key) {
                                if let Some(o) = baseline.as_object_mut() {
                                    o.remove(key);
                                }
                                if let Some(o) = properties.as_object_mut() {
                                    o.remove(key);
                                }
                            }
                        }
                        draft.base_properties = serde_json::to_string_pretty(&baseline).unwrap();
                        draft.properties = serde_json::to_string_pretty(&properties).unwrap();
                    }
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
                    if let Some(tables) = draft.tables {
                        self.tables.restore(&tables, window, cx);
                    } else if let Ok(props) = serde_json::from_str::<Value>(&draft.properties) {
                        self.tables = inspector::Tables::load(&props, window, cx);
                    }
                    self.properties.update(cx, |state, cx| {
                        state.set_value(draft.properties, window, cx)
                    });
                    if !matches {
                        self.edit_revision = u64::MAX;
                        self.status="Recovered draft conflicts with the graph. Copy it or reload the entity.".into();
                    }
                }
                let clean = !self.has_draft(cx);
                let committed_unchanged = self.editor_committed
                    && self
                        .submitted_editor
                        .as_ref()
                        .is_some_and(|submitted| submitted == &self.editor_fingerprint(cx));
                if self.editor_committed && self.submitted_revision == Some(self.snapshot.revision)
                {
                    self.edit_revision = self.snapshot.revision;
                }
                if clean
                    || committed_unchanged
                    || self.editor_entity.as_ref() != self.selected.first()
                {
                    self.load_editor(window, cx);
                } else {
                    self.status =
                        "Draft retained. Apply checks for concurrent project changes.".into();
                }
                if self.editor_committed {
                    self.submitted_editor = None;
                    self.submitted_revision = None;
                    self.editor_committed = false;
                    self.editor_request_pending = false;
                }
            }
        }
        cx.notify();
    }
    fn rebuild(&mut self) {
        self.scene = Arc::new(
            scene::Scene::new(&self.snapshot.document, &self.parent)
                .with_appearance(self.options.saved.appearance.clone()),
        );
        self.tree.clear();
        let mut stack = vec![(self.snapshot.document.root_node_id.clone(), 0)];
        while let Some((id, depth)) = stack.pop() {
            if let Some(n) = self.snapshot.document.nodes.get(&id) {
                self.tree.push((id.clone(), depth));
                if !self.collapsed.contains(&id) {
                    let mut children = n
                        .children
                        .iter()
                        .filter_map(|id| self.snapshot.document.nodes.get(id))
                        .collect::<Vec<_>>();
                    children.sort_by_cached_key(|n| {
                        (
                            if n.is_link() {
                                3
                            } else if n.is_composite() {
                                0
                            } else if n.kind == wirecraft_core::NodeKind::Type {
                                1
                            } else {
                                2
                            },
                            n.name.to_lowercase(),
                        )
                    });
                    stack.extend(
                        children
                            .into_iter()
                            .rev()
                            .map(|n| (n.id.clone(), depth + 1)),
                    );
                }
            }
        }
    }
    fn persist_ui(&self, cx: &App) {
        let state = UiSession {
            document_id: self.snapshot.document.id.clone(),
            viewport: self.viewport,
            parent: self.parent.clone(),
            selected: self.selected.first().cloned(),
            selected_ids: self.selected.iter().cloned().collect(),
            collapsed: self.collapsed.clone(),
            sidebar_width: self.sidebar_width,
            inspector_width: self.inspector_width,
            tab: self.tab.clone(),
            section: self.section.clone(),
            export_settings: self.export_settings.clone(),
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
                    tables: Some(self.tables.draft(cx)),
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
        self.completion = None;
        self.function_header.clear();
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
                "revision",
                "diagnostics",
                "statusChanges",
            ] {
                props.as_object_mut().unwrap().remove(k);
            }
            if n.id == self.snapshot.document.root_node_id {
                props["masterRevision"] = self.snapshot.document.master_revision.clone();
            }
            self.base_properties = serde_json::to_string_pretty(&props).unwrap();
            let mut all = serde_json::to_value(n).unwrap();
            if n.id == self.snapshot.document.root_node_id {
                all["masterRevision"] = self.snapshot.document.master_revision.clone();
            }
            self.tables = inspector::Tables::load(&all, window, cx);
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
                .update(cx, |state, cx| state.set_highlighter(language.clone(), cx));
            if language == "c"
                && !n.is_link()
                && !matches!(
                    n.kind,
                    wirecraft_core::NodeKind::Type | wirecraft_core::NodeKind::Note
                )
                && self.snapshot.document.effective(&n.id, "technologyId") == "c-native"
                && ["source", "initialization"].contains(&self.section.as_str())
            {
                self.client.send(
                    "editor.context",
                    json!({"nodeId":n.id,"section":self.section}),
                );
            }
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
    fn editor_fingerprint(&self, cx: &App) -> Value {
        json!({"nodeId":self.editor_entity,"name":self.name.read(cx).value().to_string(),"text":self.editor.read(cx).value().to_string(),"section":self.section,"properties":self.merged_properties(cx),"tables":self.tables.draft(cx)})
    }
    fn apply(&mut self, _: &Apply, window: &mut Window, cx: &mut Context<Self>) {
        if self.editor_request_pending {
            self.status = "An editor commit is already pending".into();
            cx.notify();
            return;
        }
        if let Err(error) = self.tables.merge(&mut json!({}), cx) {
            self.status = error;
            cx.notify();
            return;
        }
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
            match workbench::property_changes(&properties, &self.base_properties) {
                Ok(changes) => patch.as_object_mut().unwrap().extend(changes),
                Err(error) => {
                    self.status = error;
                    cx.notify();
                    return;
                }
            }
        }
        if text != self.base_text {
            patch["text"][&self.section] = json!(text);
        }
        if patch.as_object().unwrap().is_empty() {
            self.load_editor(window, cx);
            self.persist_ui(cx);
            cx.notify();
            return;
        }
        let request = Request {
            command: "threadwork.update_entity".into(),
            arguments: json!({"nodeId":id,"patch":patch}),
            expected_revision: Some(self.edit_revision),
        };
        self.editor_request_pending = true;
        self.submitted_editor = Some(self.editor_fingerprint(cx));
        let _ = self.client.sender.send(Message::Commit(request));
        self.status = "Applying…".into();
        cx.notify();
    }
    fn has_draft(&self, cx: &App) -> bool {
        self.name.read(cx).value().as_ref() != self.base_name
            || self.editor.read(cx).value().as_ref() != self.base_text
            || self.merged_properties(cx) != self.base_properties
            || self.tables.changed(cx)
    }
    fn select(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.has_draft(cx) || self.editor_request_pending {
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
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open a Threadworks .orch project".into()),
        });
        cx.spawn_in(window, async move |this, cx| match paths.await {
            Ok(Ok(Some(paths))) => {
                let _=this.update_in(cx,|this,window,cx|{if let Some(path)=paths.first(){
                    let arguments=json!({"path":path});
                    if this.snapshot.dirty||this.has_draft(cx){
                        let answer=window.prompt(PromptLevel::Warning,"Open another design?",Some("This discards current unsaved graph changes and the editor draft."),&["Discard and open","Cancel"],cx);
                        cx.spawn_in(window,async move|this,cx|{if answer.await.ok()==Some(0){let _=this.update_in(cx,|this,_,_|{let mut arguments=arguments;arguments["discardChanges"]=json!(true);this.client.send("file.open",arguments);});}}).detach();
                    }else{this.client.send("file.open",arguments);}
                }});
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
        self.tables.merge(&mut json!({}), cx)?;
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
                patch
                    .as_object_mut()
                    .unwrap()
                    .extend(workbench::property_changes(
                        &properties,
                        &self.base_properties,
                    )?);
            }
            if text != self.base_text {
                patch["text"][&self.section] = json!(text);
            }
            if !patch.as_object().unwrap().is_empty() {
                a["nodeId"] = json!(id);
                a["patch"] = patch;
            }
        }
        Ok(a)
    }
    fn commit_save(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) {
        if self.editor_request_pending {
            self.status = "An editor commit is already pending".into();
            cx.notify();
            return;
        }
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
                self.submitted_editor = if self.has_draft(cx) {
                    Some(self.editor_fingerprint(cx))
                } else {
                    None
                };
                self.editor_request_pending = true;
                let _ = self.client.sender.send(Message::Commit(request));
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
    fn new_project(&mut self, _: &New, window: &mut Window, cx: &mut Context<Self>) {
        if self.snapshot.dirty || self.has_draft(cx) {
            let answer = window.prompt(
                PromptLevel::Warning,
                "Create a new design?",
                Some("This discards the current unsaved graph changes and editor draft."),
                &["Discard and create", "Cancel"],
                cx,
            );
            cx.spawn_in(window, async move |this, cx| {
                if answer.await.ok() == Some(0) {
                    let _ = this.update_in(cx, |this, _, _| {
                        this.client.send("file.new", json!({"discardChanges":true}))
                    });
                }
            })
            .detach();
        } else {
            self.client.send("file.new", json!({}));
        }
    }
    fn undo(&mut self, _: &Undo, _: &mut Window, _: &mut Context<Self>) {
        self.client.send("edit.undo", json!({}));
    }
    fn redo(&mut self, _: &Redo, _: &mut Window, _: &mut Context<Self>) {
        self.client.send("edit.redo", json!({}));
    }
    fn delete(&mut self, _: &DeleteSelection, _: &mut Window, cx: &mut Context<Self>) {
        if self.has_draft(cx) || self.editor_request_pending {
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
        let b = self.bounds.get();
        self.viewport = self.scene.fitted([
            f64::from(f32::from(b.size.width)),
            f64::from(f32::from(b.size.height)),
        ]);
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
        if self.has_draft(cx) || self.editor_request_pending {
            self.status = "Apply the editor draft before changing selection".into();
            cx.notify();
            return;
        }
        let world = self.viewport.world(local);
        let tolerance = 6. / self.viewport.zoom;
        let hits = self.scene.query(Rect {
            x: world[0] - tolerance,
            y: world[1] - tolerance,
            width: tolerance * 2.,
            height: tolerance * 2.,
        });
        let hit = hits
            .iter()
            .filter(|id| {
                self.scene
                    .nodes
                    .get(*id)
                    .is_some_and(|n| n.2.contains(world))
            })
            .max_by_key(|id| {
                self.scene
                    .drawing
                    .shapes
                    .get(*id)
                    .map(|s| s.depth)
                    .unwrap_or(0)
            })
            .cloned()
            .or_else(|| {
                hits.iter().find_map(|id| {
                    self.scene
                        .drawing
                        .annotations
                        .get(id)
                        .filter(|a| a.rect.contains(world))
                        .map(|a| a.target.clone())
                })
            })
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
                    if let Some(shape) = self.scene.drawing.shapes.get(&id) {
                        let r = shape.rect;
                        if (world[0] - r.x - r.width).abs() < 10. / self.viewport.zoom
                            && (world[1] - r.y - r.height).abs() < 10. / self.viewport.zoom
                        {
                            self.resize_start = Some((id, world, r));
                            cx.notify();
                            return;
                        }
                        if shape.composite && world[0] < r.x + 28. && world[1] < r.y + 28. {
                            let expanded = !shape.expanded;
                            let layout = &self.snapshot.document.nodes[&id].layout;
                            self.client.send("threadwork.update_entity",json!({"nodeId":id,"patch":{"layout":{"isExpanded":expanded,"width":if expanded{layout.open_width}else{layout.closed_width},"height":if expanded{layout.open_height}else{layout.closed_height}}}}));
                            cx.notify();
                            return;
                        }
                    }
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
                            self.drag_ids = self
                                .selected
                                .iter()
                                .flat_map(|id| self.snapshot.document.descendants(id))
                                .collect();
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
        } else if let Some((_, start, _)) = &self.resize_start {
            let world = self.viewport.world(local);
            self.drag_delta = [world[0] - start[0], world[1] - start[1]];
            cx.notify();
        } else if let Some(start) = self.drag_start {
            let world = self.viewport.world(local);
            self.drag_delta = [world[0] - start[0], world[1] - start[1]];
            cx.notify();
        } else if let Some(start) = self.selection_start {
            let end = self.viewport.world(local);
            self.selection_end = Some(end);
            let rect = Rect {
                x: start[0].min(end[0]),
                y: start[1].min(end[1]),
                width: (end[0] - start[0]).abs(),
                height: (end[1] - start[1]).abs(),
            };
            self.selected = self
                .scene
                .query(rect)
                .into_iter()
                .filter(|id| self.scene.nodes.contains_key(id))
                .collect();
            cx.notify();
        }
    }
    fn up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((id, _, r)) = self.resize_start.take() {
            let n = &self.snapshot.document.nodes[&id];
            let width = (r.width + self.drag_delta[0]).max(100.);
            let height = (r.height + self.drag_delta[1]).max(70.);
            let mut layout = json!({"width":width,"height":height});
            if n.is_composite() && n.layout.is_expanded {
                layout["openWidth"] = json!(width);
                layout["openHeight"] = json!(height);
            } else {
                layout["closedWidth"] = json!(width);
                layout["closedHeight"] = json!(height);
            }
            self.client.send(
                "threadwork.update_entity",
                json!({"nodeId":id,"patch":{"layout":layout}}),
            );
        } else if self.drag_start.take().is_some()
            && (self.drag_delta[0].abs() > 0.5 || self.drag_delta[1].abs() > 0.5)
        {
            let mut ids = self.selected.clone();
            let mut stack = self.selected.iter().cloned().collect::<Vec<_>>();
            while let Some(id) = stack.pop() {
                if let Some(n) = self.snapshot.document.nodes.get(&id) {
                    for child in &n.children {
                        if ids.insert(child.clone()) {
                            stack.push(child.clone());
                        }
                    }
                }
            }
            let changes=ids.iter().filter_map(|id|self.snapshot.document.nodes.get(id)).filter(|n|!n.is_link()).map(|n|json!({"nodeId":n.id,"patch":{"layout":{"x":n.layout.x+self.drag_delta[0],"y":n.layout.y+self.drag_delta[1]}}})).collect::<Vec<_>>();
            self.client
                .send("entities.patch", json!({"changes":changes}));
        }
        self.drag_delta = [0., 0.];
        self.drag_ids.clear();
        self.pan_start = None;
        self.selection_start = None;
        self.selection_end = None;
        self.persist_ui(cx);
        cx.notify();
    }
    fn wheel(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let delta = event
            .delta
            .pixel_delta(px(20. * self.options.saved.appearance.wheel_rows / 3.));
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
        if self.has_draft(cx) || self.editor_request_pending {
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
        .bg(crate::chrome::color(0x242c36))
        .hover(|s| s.bg(crate::chrome::color(0x34414d)))
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            .bg(crate::chrome::color(0x171d25))
            .child(
                div()
                    .text_lg()
                    .text_color(crate::chrome::color(0x75d4d6))
                    .child("Wirecraft"),
            )
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
                    .when(self.tab == tab, |b| b.bg(crate::chrome::color(0x255452)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.tab = tab.into();
                        if tab == "Export" {
                            this.refresh_preview();
                            this.tool("printers.list");
                        }
                        if tab == "MCP" {
                            this.tool("mcp.status");
                        }
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
                        let node = &this.snapshot.document.nodes[&id];
                        let name = node.name.clone();
                        let composite = node.is_composite();
                        let kind = node.kind;
                        let collapsed = this.collapsed.contains(&id);
                        let toggle = id.clone();
                        let target = id.clone();
                        let ticket = workbench::Ticket {
                            id: id.clone(),
                            name: name.clone(),
                        };
                        div()
                            .id(i)
                            .h(px(28.))
                            .pl(px(10. + depth as f32 * 14.))
                            .pr_2()
                            .overflow_hidden()
                            .when(this.selected.contains(&id), |b| {
                                b.bg(crate::chrome::color(0x253f45))
                            })
                            .cursor_pointer()
                            .on_click(_cx.listener(move |this, _, window, cx| {
                                this.select(id.clone(), window, cx)
                            }))
                            .on_drag(ticket, |ticket, _, _, cx| cx.new(|_| ticket.clone()))
                            .on_drop(_cx.listener(
                                move |this, ticket: &workbench::Ticket, _, cx| {
                                    if !this.has_draft(cx) {
                                        this.client.send(
                                            "threadwork.move_entity",
                                            json!({"nodeId":ticket.id,"parentId":target}),
                                        );
                                    } else {
                                        this.status =
                                            "Apply the editor draft before moving entities".into();
                                        cx.notify();
                                    }
                                },
                            ))
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id("tree-toggle")
                                    .w(px(14.))
                                    .cursor_pointer()
                                    .on_click(_cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        if composite {
                                            if !this.collapsed.insert(toggle.clone()) {
                                                this.collapsed.remove(&toggle);
                                            }
                                            this.rebuild();
                                            this.persist_ui(cx);
                                            cx.notify();
                                        }
                                    }))
                                    .child(if composite {
                                        if collapsed { "▸" } else { "▾" }
                                    } else {
                                        ""
                                    }),
                            )
                            .child(icons::icon(if kind == wirecraft_core::NodeKind::Type {
                                "type"
                            } else if kind == wirecraft_core::NodeKind::Link {
                                "link"
                            } else {
                                "node"
                            }))
                            .child(name)
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .flex_1();
        let sidebar = div()
            .w(px(self.sidebar_width))
            .flex_none()
            .flex()
            .flex_col()
            .bg(crate::chrome::color(0x191f28))
            .when(self.tab == "Editor", |b| {
                b.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_xs()
                        .text_color(crate::chrome::color(0x70d5c6))
                        .child(format!("Selected entities · {}", self.selected.len())),
                )
                .child(
                    uniform_list(
                        "selected-entities",
                        self.selected.len(),
                        cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                            this.selected
                                .iter()
                                .skip(range.start)
                                .take(range.len())
                                .enumerate()
                                .map(|(i, id)| {
                                    let select = id.clone();
                                    div()
                                        .id(i)
                                        .h(px(28.))
                                        .pl_3()
                                        .cursor_pointer()
                                        .child(
                                            this.snapshot
                                                .document
                                                .nodes
                                                .get(id)
                                                .map(|n| n.name.clone())
                                                .unwrap_or_else(|| "Restoring selection…".into()),
                                        )
                                        .on_click(cx.listener(move |this, _, w, cx| {
                                            this.select(select.clone(), w, cx)
                                        }))
                                })
                                .collect::<Vec<_>>()
                        }),
                    )
                    .h(px(180.))
                    .flex_none(),
                )
            })
            .child(
                div()
                    .p_3()
                    .text_color(crate::chrome::color(0x7f9bae))
                    .child("Entity Hierarchy Tree"),
            )
            .child(tree);
        let mut content = div().flex_1().min_w_0().min_h_0().flex().flex_col();
        if self.tab == "Designer" {
            let scene = self.scene.clone();
            let bounds = self.bounds.clone();
            let view = self.viewport;
            let selected = self.selected.clone();
            let drag = if self.resize_start.is_some() {
                [0., 0.]
            } else {
                self.drag_delta
            };
            let moving = self.drag_ids.clone();
            let resize = self
                .resize_start
                .as_ref()
                .map(|(_, _, r)| (*r, self.drag_delta));
            let selection = self.selection_start.zip(self.selection_end);
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
                        .child(button("sheet", "Sheet").on_click(
                            cx.listener(|this, _, w, cx| this.show_export(&ShowExport, w, cx)),
                        ))
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
                .child(
                    div()
                        .id("canvas")
                        .flex_1()
                        .min_h_0()
                        .overflow_hidden()
                        .bg(crate::chrome::color(0x10161e))
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
                                    scene.paint(b, view, &selected, &moving, drag, window, cx);
                                    if let Some((r, delta)) = resize {
                                        let p = view.screen([r.x, r.y]);
                                        window.paint_quad(quad(
                                            Bounds::new(
                                                b.origin + point(px(p[0] as f32), px(p[1] as f32)),
                                                size(
                                                    px(
                                                        ((r.width + delta[0]).max(100.) * view.zoom)
                                                            as f32,
                                                    ),
                                                    px(
                                                        ((r.height + delta[1]).max(70.) * view.zoom)
                                                            as f32,
                                                    ),
                                                ),
                                            ),
                                            px(4.),
                                            rgba(0x70d5c620),
                                            px(2.),
                                            crate::chrome::color(0x70d5c6),
                                            BorderStyle::Dashed,
                                        ));
                                    }
                                    if let Some((a, z)) = selection {
                                        let a = view.screen(a);
                                        let z = view.screen(z);
                                        window.paint_quad(quad(
                                            Bounds::new(
                                                b.origin
                                                    + point(
                                                        px(a[0].min(z[0]) as f32),
                                                        px(a[1].min(z[1]) as f32),
                                                    ),
                                                size(
                                                    px((a[0] - z[0]).abs() as f32),
                                                    px((a[1] - z[1]).abs() as f32),
                                                ),
                                            ),
                                            px(0.),
                                            rgba(0x70d5c630),
                                            px(1.),
                                            crate::chrome::color(0x70d5c6),
                                            BorderStyle::Dashed,
                                        ));
                                    }
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
            content = content.child(self.render_export(cx));
        } else if self.tab == "Options" {
            content = content.child(self.render_options(cx));
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
                    div().id(i).p_2().mb_2().bg(crate::chrome::color(0x3c2831)).cursor_pointer().on_click(cx.listener(move|this,_,window,cx|this.reveal_diagnostic(&diagnostic,window,cx))).child(label)
                })).child(self.output.clone()));
        } else {
            content = content.child(self.render_mcp(cx));
        }
        div().on_mouse_move(cx.listener(Self::move_panel)).on_mouse_up(MouseButton::Left,cx.listener(Self::end_panel)).capture_action(cx.listener(|this,_:&gpui_component::input::MoveUp,w,cx|{if !this.completion_action("up",w,cx){cx.propagate();}})).capture_action(cx.listener(|this,_:&gpui_component::input::MoveDown,w,cx|{if !this.completion_action("down",w,cx){cx.propagate();}})).capture_action(cx.listener(|this,action:&gpui_component::input::Enter,w,cx|{if action.secondary && this.tab=="Editor" && this.menu.is_none(){cx.stop_propagation();this.completion=None;this.apply(&Apply,w,cx);}else if !this.completion_action("enter",w,cx){cx.propagate();}})).capture_action(cx.listener(|this,_:&gpui_component::input::Escape,w,cx|{if !this.completion_action("escape",w,cx){cx.propagate();}})).capture_action(cx.listener(|this,_:&gpui_component::input::IndentInline,w,cx|{if !this.completion_action("tab",w,cx){cx.propagate();}})).key_context("Wirecraft").track_focus(&self.focus).on_action(cx.listener(Self::open)).on_action(cx.listener(Self::save)).on_action(cx.listener(Self::save_as)).on_action(cx.listener(Self::new_project)).on_action(cx.listener(Self::undo)).on_action(cx.listener(Self::redo)).on_action(cx.listener(Self::delete)).on_action(cx.listener(Self::select_mode)).on_action(cx.listener(Self::node_mode)).on_action(cx.listener(Self::link_mode)).on_action(cx.listener(Self::fit)).on_action(cx.listener(Self::apply)).on_action(cx.listener(Self::copy)).on_action(cx.listener(Self::cut)).on_action(cx.listener(Self::paste)).on_action(cx.listener(Self::palette)).on_action(cx.listener(Self::show_export)).on_action(cx.listener(Self::show_options)).on_action(cx.listener(Self::request_completion)).on_action(|_:&Quit,_,cx|cx.quit()).relative().size_full().flex().flex_col().bg(crate::chrome::color(0x10161e)).text_color(crate::chrome::color(0xe1e9ef)).text_sm().child(self.render_menubar(cx)).child(toolbar).child(tabs).child(div().flex().flex_1().min_h_0().when(["Designer","Editor","Tools"].contains(&self.tab.as_str()), |b| b.child(sidebar).child(self.panel_separator(false,cx))).child(content)).child(div().px_3().py_2().bg(crate::chrome::color(0x171d25)).child(format!("{}   |   rev {}   |   {:.0}%   |   Scroll: pan · Ctrl scroll: zoom · Double click: enter/edit",self.status,self.snapshot.revision,self.viewport.zoom*100.))).children(self.render_completion(window,cx)).children(self.render_menu(cx))
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
    let initial_settings = settings.clone();
    let (client, rx, worker) = Client::start(state_dir.clone(), settings, project);
    gpui::Application::new()
        .with_assets(icons::Assets)
        .run(move |cx| {
            if let Err(error) = fonts::register(cx) {
                eprintln!("Font registration: {error}");
            }
            gpui_component::init(cx);
            crate::chrome::set_light(initial_settings.appearance.theme == "light");
            Theme::change(
                if initial_settings.appearance.theme == "light" {
                    ThemeMode::Light
                } else {
                    ThemeMode::Dark
                },
                None,
                cx,
            );
            Theme::global_mut(cx).mono_font_family =
                initial_settings.appearance.editor_font.clone().into();
            cx.bind_keys([
                KeyBinding::new("ctrl-shift-p", CommandPalette, Some("Wirecraft")),
                KeyBinding::new("ctrl-space", Complete, Some("Wirecraft")),
                KeyBinding::new(
                    "ctrl-home",
                    gpui_component::input::MoveToStart,
                    Some("Input"),
                ),
                KeyBinding::new("ctrl-end", gpui_component::input::MoveToEnd, Some("Input")),
                KeyBinding::new(
                    "ctrl-shift-home",
                    gpui_component::input::SelectToStart,
                    Some("Input"),
                ),
                KeyBinding::new(
                    "ctrl-shift-end",
                    gpui_component::input::SelectToEnd,
                    Some("Input"),
                ),
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
                    app_id: Some("wirecraft".into()),
                    window_decorations: if cfg!(target_os = "linux") {
                        Some(WindowDecorations::Client)
                    } else {
                        None
                    },
                    window_min_size: Some(size(px(900.), px(600.))),
                    ..Default::default()
                },
                move |window, cx| {
                    window.set_window_title("Wirecraft");
                    let view = cx.new(|cx| {
                        Wirecraft::new(client, rx, session, initial_settings, window, cx)
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("open Wirecraft window");
            cx.activate(true);
        });
    let _ = worker.join();
    Ok(())
}
