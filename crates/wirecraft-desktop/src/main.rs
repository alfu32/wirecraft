mod chrome;
mod client;
mod compiler_issues;
mod completion;
mod editor_hover;
mod editor_tokens;
mod fonts;
mod icons;
mod inspector;
mod inspector_widgets;
mod interaction;
mod navigation;
mod options;
mod preview;
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
    viewport::{Rect, SelectionOperation, Viewport},
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
        Complete,
        GoToDefinition,
        FindReferences
    ]
);
#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Select,
    Node,
    Type,
    Link,
    /// Places an unlinked Exterior; linking it publishes a port.
    Port,
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
    show_sheets: bool,
    sheet_contours: Vec<wirecraft_core::viewport::Rect>,
    /// Title block and parts list contours of the assembled sheet.
    sheet_blocks: Vec<wirecraft_core::sheets::SheetBlock>,
    link_source: Option<String>,
    /// Link mode started on empty root canvas: the link will leave a new Exterior here.
    exterior_source: Option<[f64; 2]>,
    /// World pointer position while a placement or link under construction follows it.
    hover_world: Option<[f64; 2]>,
    /// A picked-up link end being moved: the link and whether it is its source end.
    rewire: Option<(String, bool)>,
    /// The design as the pending click would leave it, routed in the background.
    preview: Option<preview::Preview>,
    preview_timer: Option<Task<()>>,
    preview_inflight: bool,
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
    pending_pick: Option<String>,
    selection_base: BTreeSet<String>,
    selection_operation: SelectionOperation,
    reroute_timer: Option<Task<()>>,
    reroute_inflight: bool,
    reroute_epoch: u64,
    drag_scene: Option<(Arc<scene::Scene>, [f64; 2])>,
    move_pending: bool,
    resize_start: Option<(String, [f64; 2], Rect, interaction::Corner)>,
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
    compiler_timer: Option<Task<()>>,
    compiler_inflight: bool,
    compiler_again: bool,
    compiler_failure: Option<String>,
    compiler_issue_cache: Vec<Value>,
    generated_revision: u64,
    inspector: Vec<workbench::FormField>,
    completion: Option<completion::Pending>,
    completion_generation: u64,
    navigation: Option<navigation::Ticket>,
    navigation_generation: u64,
    references: Option<navigation::References>,
    /// Read-only generated code of the selected entity, pinned as the editor's unnumbered
    /// first rows: its body's function header, its Type declaration or its link code.
    /// `signature_key` is the entity and section shown; a header too wide for the editor
    /// shows its `signature_wrapped` form.
    signature: Entity<InputState>,
    signature_key: Option<(String, String)>,
    signature_lines: usize,
    signature_language: SharedString,
    signature_text: String,
    signature_wrapped: Option<String>,
    signature_wraps: bool,
    /// Highlighter language of `editor`.
    editor_language: String,
    window_title: String,
    token_timer: Option<Task<()>>,
    token_ticket: Option<editor_tokens::Ticket>,
    token_dirty: bool,
    token_inflight: bool,
    token_generation: u64,
    tables: inspector::Tables,
    advanced: bool,
    editor_committed: bool,
    editor_request_pending: bool,
    submitted_editor: Option<Value>,
    submitted_revision: Option<u64>,
    menu: Option<String>,
    context_origin: Option<Point<Pixels>>,
    /// Window-space bounds of menubar labels, measured during paint, for dropdown anchoring.
    menu_anchors: Rc<std::cell::RefCell<std::collections::BTreeMap<&'static str, Bounds<Pixels>>>>,
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
        let signature = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("plain")
                .line_number(false)
                .searchable(false)
        });
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
            show_sheets: false,
            sheet_contours: Vec::new(),
            sheet_blocks: Vec::new(),
            link_source: None,
            exterior_source: None,
            hover_world: None,
            rewire: None,
            preview: None,
            preview_timer: None,
            preview_inflight: false,
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
            pending_pick: None,
            selection_base: BTreeSet::new(),
            selection_operation: SelectionOperation::Replace,
            reroute_timer: None,
            reroute_inflight: false,
            reroute_epoch: 0,
            drag_scene: None,
            move_pending: false,
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
            compiler_timer: None,
            compiler_inflight: false,
            compiler_again: false,
            compiler_failure: None,
            compiler_issue_cache: vec![],
            generated_revision: 0,
            inspector,
            completion: None,
            completion_generation: 0,
            navigation: None,
            navigation_generation: 0,
            references: None,
            signature,
            signature_key: None,
            signature_lines: 0,
            signature_language: "plain".into(),
            signature_text: String::new(),
            signature_wrapped: None,
            signature_wraps: false,
            editor_language: "plain".into(),
            window_title: String::new(),
            token_timer: None,
            token_ticket: None,
            token_dirty: false,
            token_inflight: false,
            token_generation: 0,
            tables: inspector::Tables::default(),
            advanced: false,
            editor_committed: false,
            editor_request_pending: false,
            submitted_editor: None,
            submitted_revision: None,
            menu: None,
            context_origin: None,
            menu_anchors: Rc::default(),
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
        this.attach_hover(cx);
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
                    this.queue_tokens(cx);
                    this.persist_ui(cx);
                    cx.notify();
                }
            })
            .detach();
        }
        cx.subscribe_in(
            &this.name,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur)
                    && this.name.read(cx).value().as_ref() != this.base_name
                    && !this.editor_request_pending
                {
                    this.apply(&Apply, window, cx);
                }
            },
        )
        .detach();
        for field in &this.inspector {
            let path = field.path;
            cx.subscribe_in(
                &field.input,
                window,
                move |this, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::PressEnter { .. })
                        && this.inspector.iter().any(|f| {
                            f.path == path && f.input.read(cx).value().as_ref() != f.baseline
                        })
                        && !this.editor_request_pending
                    {
                        this.apply(&Apply, window, cx);
                    }
                },
            )
            .detach();
        }
        cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() {
                this.cancel_pointer_gesture(cx);
            }
        })
        .detach();
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
            Update::Error {
                message,
                commit,
                command,
            } => {
                if command.as_deref() == Some("editor.analyze_generated") {
                    if message.starts_with("conflict:") || message.starts_with("stale_result:") {
                        self.compiler_again = true;
                    } else {
                        self.compiler_failure = Some(message.clone());
                    }
                    self.compiler_finished(cx);
                }
                if command.as_deref() == Some("editor.tokens") {
                    self.tokens_failed(message, cx);
                    return;
                }
                // The signature is background information: its failure is shown in place
                // (a component, for instance, only generates through a design using it).
                if command.as_deref() == Some("editor.signature") {
                    if !message.starts_with("conflict:") && !message.starts_with("stale_result:") {
                        let text = format!(
                            "// No generated code: {}",
                            message
                                .split_once(": ")
                                .map_or(message.as_str(), |(_, m)| m)
                        );
                        self.show_signature("plain", text, None, window, cx);
                    }
                    return;
                }
                self.completion = None;
                if command
                    .as_deref()
                    .is_some_and(|c| ["editor.definition", "editor.references"].contains(&c))
                {
                    self.navigation = None;
                }
                self.status = message;
                if self.move_pending {
                    self.clear_drag_preview();
                    self.move_pending = false;
                }
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
                    // Background information, not the outcome of a user action.
                    "editor.signature" => std::mem::take(&mut self.status),
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
                    && let Some(id) = value["entity"]["id"].as_str()
                    && self.selected.first().map(String::as_str) == Some(id)
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
                        if let Some(props) = props.as_object_mut() {
                            props.remove(k);
                        }
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
                if ["editor.definition", "editor.references"].contains(&command.as_str()) {
                    self.accept_navigation(&command, &value, window, cx);
                }
                if command == "editor.tokens" {
                    self.accept_tokens(value.clone(), cx);
                }
                if command == "editor.signature"
                    && value["revision"].as_u64() == Some(self.snapshot.revision)
                    && value["nodeId"].as_str() == self.editor_entity.as_deref()
                    && value["section"] == self.section
                {
                    self.show_signature(
                        value["language"].as_str().unwrap_or("plain").to_string(),
                        value["text"].as_str().unwrap_or("").to_string(),
                        value["wrapped"].as_str().map(str::to_string),
                        window,
                        cx,
                    );
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
                    self.compiler_failure = None;
                    self.compiler_finished(cx);
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
                let graph_changed = snapshot.document.id != self.snapshot.document.id
                    || snapshot.revision != self.snapshot.revision;
                let document_changed = (self.connected_document
                    && snapshot.document.id != self.snapshot.document.id)
                    || (!self.connected_document
                        && !self.initial_document_id.is_empty()
                        && self.initial_document_id != snapshot.document.id);
                if snapshot.revision != self.snapshot.revision || document_changed {
                    self.cancel_pointer_gesture(cx);
                    self.clear_drag_preview();
                    self.move_pending = false;
                }
                self.connected_document = true;
                if snapshot.document.id != self.snapshot.document.id {
                    self.editor_entity = None;
                }
                self.snapshot = Arc::new(*snapshot);
                if graph_changed {
                    self.compiler_failure = None;
                    self.queue_compiler_check(cx);
                }
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
                if document_changed {
                    self.selected.insert(self.parent.clone());
                }
                self.rebuild();
                if self.tab == "Designer" {
                    self.reveal_tree_selection(window, cx);
                }
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
        self.refresh_compiler_issues();
        cx.notify();
    }
    /// Recomputes page contours for the current parent; only while the overlay is shown.
    fn refresh_sheets(&mut self) {
        self.sheet_contours.clear();
        self.sheet_blocks.clear();
        if !self.show_sheets {
            return;
        }
        match wirecraft_core::sheets::plan(
            &self.snapshot.document,
            &self.parent,
            &self.export_settings.sheet,
        ) {
            Ok(plan) => {
                self.sheet_contours = plan.world_tiles;
                self.sheet_blocks = plan.world_blocks;
            }
            Err(error) => self.status = format!("Sheet contours unavailable: {}", error.message),
        }
    }
    fn rebuild(&mut self) {
        self.scene = Arc::new(
            scene::Scene::new(&self.snapshot.document, &self.parent)
                .with_appearance(self.options.saved.appearance.clone()),
        );
        self.refresh_sheets();
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
    /// Pins generated code above the editor's body for the selected entity and section;
    /// empty text hides it.
    fn show_signature(
        &mut self,
        language: impl Into<SharedString>,
        text: String,
        wrapped: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.signature_key = self
            .selected
            .first()
            .map(|id| (id.clone(), self.section.clone()));
        self.signature_text = text;
        self.signature_wrapped = wrapped;
        let text = match &self.signature_wrapped {
            Some(wrapped) if self.signature_wraps => wrapped.clone(),
            _ => self.signature_text.clone(),
        };
        self.signature_lines = if text.is_empty() {
            0
        } else {
            text.lines().count()
        };
        // A new highlighter only colors text set after it, so the text is set again.
        let language = language.into();
        let relanguage = language != self.signature_language;
        if relanguage {
            self.signature_language = language.clone();
        }
        self.signature.update(cx, |state, cx| {
            if relanguage {
                state.set_highlighter(language, cx);
            }
            if relanguage || state.value().as_ref() != text {
                state.set_value(text, window, cx);
            }
        });
        cx.notify();
    }
    /// Switches the pinned header between its generated line and its wrapped form.
    fn fit_signature(&mut self, wraps: bool, window: &mut Window, cx: &mut Context<Self>) {
        if wraps == self.signature_wraps {
            return;
        }
        self.signature_wraps = wraps;
        let text = std::mem::take(&mut self.signature_text);
        let wrapped = self.signature_wrapped.take();
        self.show_signature(self.signature_language.clone(), text, wrapped, window, cx);
    }
    fn load_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.token_inflight {
            self.token_ticket = None;
        }
        self.completion = None;
        let key = self
            .selected
            .first()
            .map(|id| (id.clone(), self.section.clone()));
        // Snapshots of the same entity keep the shown signature until the new one arrives.
        if key != self.signature_key {
            self.signature_key = None;
            self.signature_lines = 0;
            self.signature_text.clear();
            self.signature_wrapped = None;
            self.signature
                .update(cx, |state, cx| state.set_value("", window, cx));
        }
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
            // A new highlighter only colors text set after it: snapshots of the same
            // entity keep the highlighter, and a language change sets the text again.
            let relanguage = language != self.editor_language;
            if relanguage {
                self.editor
                    .update(cx, |state, cx| state.set_highlighter(language.clone(), cx));
                self.editor_language = language;
            }
            if n.is_link()
                || n.kind == wirecraft_core::NodeKind::Type
                || wirecraft_core::editor::body_language(
                    &self.snapshot.document,
                    &n.id,
                    &self.section,
                )
                .is_some()
            {
                self.client.send(
                    "editor.signature",
                    json!({"nodeId":n.id,"section":self.section}),
                );
            }
            self.edit_revision = self.snapshot.revision;
            self.name.update(cx, |state, cx| {
                state.set_value(self.base_name.clone(), window, cx)
            });
            // Snapshots reload a clean editor; setting identical text would move the
            // cursor to the start, losing a revealed or navigated position.
            self.editor.update(cx, |state, cx| {
                if relanguage || state.value().as_ref() != self.base_text {
                    state.set_value(self.base_text.clone(), window, cx)
                }
            });
            self.properties.update(cx, |state, cx| {
                state.set_value(self.base_properties.clone(), window, cx)
            });
        } else {
            self.editor_entity = None;
            self.base_name.clear();
            self.base_text.clear();
            self.base_properties = "{}".into();
            self.tables = inspector::Tables::load(&json!({"metadata":{}}), window, cx);
            for field in &mut self.inspector {
                field.baseline.clear();
                field
                    .input
                    .update(cx, |state, cx| state.set_value("", window, cx));
            }
            self.name
                .update(cx, |state, cx| state.set_value("", window, cx));
            self.editor
                .update(cx, |state, cx| state.set_value("", window, cx));
            self.properties
                .update(cx, |state, cx| state.set_value("{}", window, cx));
        }
        self.queue_tokens(cx);
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
        self.cancel_pointer_gesture(cx);
        self.mode = Mode::Select;
        self.link_source = None;
        self.exterior_source = None;
        self.rewire = None;
        cx.notify();
    }
    fn node_mode(&mut self, _: &NodeMode, _: &mut Window, cx: &mut Context<Self>) {
        self.mode = Mode::Node;
        cx.notify();
    }
    fn link_mode(&mut self, _: &LinkMode, _: &mut Window, cx: &mut Context<Self>) {
        self.mode = Mode::Link;
        self.link_source = None;
        self.exterior_source = None;
        self.rewire = None;
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
        self.status = "Choose an output path in the save dialog".into();
        let prompt = cx.prompt_for_new_path(
            &directory,
            Some(if kind == "archetype" {
                "archetype.orch"
            } else {
                "generated-project"
            }),
        );
        cx.spawn_in(window, async move |this, cx| {
            let chosen = match prompt.await {
                Ok(Ok(path)) => path,
                result => {
                    let message = match result {
                        Ok(Err(error)) => error.to_string(),
                        Err(error) => error.to_string(),
                        _ => String::new(),
                    };
                    let _ = this.update_in(cx, |this, _, cx| {
                        this.status = format!("Cannot open output dialog: {message}");
                        cx.notify();
                    });
                    return;
                }
            };
            if let Some(path) = chosen {
                let _ = this.update_in(cx, |this, _, _| {
                    if kind == "archetype" {
                        if let Some(id) = this.selected.first() {
                            this.client
                                .send("archetypes.save", json!({"nodeId":id,"path":path}));
                        }
                    } else {
                        this.client.send(
                            if kind == "project" {
                                "compile.project"
                            } else if kind == "compiler" {
                                "compiler.export"
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
    /// Place a Reference to another saved design; it must live in this design's directory.
    fn reference_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.snapshot.path.is_none() {
            self.status = "Save the design first: a reference path is relative to its file".into();
            cx.notify();
            return;
        }
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Reference a component design (.orch)".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = prompt.await
                && let Some(path) = paths.first()
            {
                let _ = this.update_in(cx, |this, _, _| {
                    let b = this.bounds.get();
                    let center = this.viewport.world([
                        f64::from(f32::from(b.size.width)) / 2.,
                        f64::from(f32::from(b.size.height)) / 2.,
                    ]);
                    this.client.send(
                        "components.reference",
                        json!({"path":path,"parentId":this.parent,"x":center[0],"y":center[1]}),
                    )
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
            self.status = "Apply the editor draft before revealing another entity".into();
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
/// Marks the current mode or an enabled toggle with the accent fill.
fn active(button: Stateful<Div>, on: bool) -> Stateful<Div> {
    // `button` already owns the hover style; GPUI panics if a second one is set.
    button.when(on, |b| {
        b.bg(crate::chrome::color(0x255452))
            .text_color(crate::chrome::color(0x70d5c6))
    })
}
fn button(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Stateful<Div> {
    let id: SharedString = id.into();
    let icon = icons::for_button(&id);
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
        .when(!icon.is_empty(), |b| b.child(icons::icon(icon)))
        .child(label.into())
}
impl Render for Wirecraft {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = format!(
            "{}{}",
            self.snapshot.document.project_name(),
            if self.snapshot.dirty { " •" } else { "" }
        );
        let file = self
            .snapshot
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map_or("Unsaved".into(), |f| f.to_string_lossy().into_owned());
        let version = env!("CARGO_PKG_VERSION");
        let window_title = format!("{title} — {file} — Wirecraft {version}");
        if window_title != self.window_title {
            window.set_window_title(&window_title);
            self.window_title = window_title;
        }
        let muted = crate::chrome::color(0x8fa6b5);
        let toolbar = div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .bg(crate::chrome::color(0x171d25))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap_1()
                    .child(
                        div()
                            .text_lg()
                            .text_color(crate::chrome::color(0x75d4d6))
                            .child("Wirecraft"),
                    )
                    .child(div().text_xs().text_color(muted).child(version)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(title)
                    .child(div().text_sm().text_color(muted).child(file)),
            )
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
                            .w_full()
                            .h(px(28.))
                            .pl(px(10. + depth as f32 * 14.))
                            .pr_2()
                            .overflow_hidden()
                            .when(this.selected.contains(&id), |b| {
                                b.bg(crate::chrome::color(0x253f45))
                            })
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                _cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                                    this.tree_pick(id.clone(), event.modifiers, window, cx);
                                }),
                            )
                            .on_drag(ticket, |ticket, _, _, cx| cx.new(|_| ticket.clone()))
                            .on_drop(_cx.listener(
                                move |this, ticket: &workbench::Ticket, _, cx| {
                                    this.reparent_from_tree(&ticket.id, &target, cx);
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
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    // Boxed +/− expander for composites, as in classic trees.
                                    .when(composite, |toggle| {
                                        toggle.child(
                                            div()
                                                .size(px(11.))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .border_1()
                                                .border_color(crate::chrome::color(0x8a9aab))
                                                .text_size(px(10.))
                                                .line_height(px(10.))
                                                .child(if collapsed { "+" } else { "−" }),
                                        )
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
            let preview = self.current_preview();
            let (scene, routed_delta) = self
                .drag_scene
                .as_ref()
                .map(|(s, d)| (s.clone(), *d))
                .or_else(|| preview.map(|p| (p.scene.clone(), [0., 0.])))
                .unwrap_or((self.scene.clone(), [0., 0.]));
            let bounds = self.bounds.clone();
            let view = self.viewport;
            let mut selected = self.selected.clone();
            // Entities and links the pending click would create are highlighted.
            if let Some(p) = preview {
                selected.extend(p.created.iter().cloned());
            }
            let drag = if self.resize_start.is_some() {
                [0., 0.]
            } else {
                [
                    self.drag_delta[0] - routed_delta[0],
                    self.drag_delta[1] - routed_delta[1],
                ]
            };
            let errors = self
                .compiler_issues()
                .iter()
                .filter(|d| d["severity"] == 1 || d["severity"] == 2)
                .filter_map(|d| d["source"]["nodeId"].as_str().map(str::to_string))
                .collect::<BTreeSet<_>>();
            let moving = self.drag_ids.clone();
            // Preview contour: the current box grown by each side's padding change.
            let resize = self.resize_start.as_ref().map(|(id, _, r, corner)| {
                let n = &self.snapshot.document.nodes[id];
                let new = interaction::padding_after_drag(n, *corner, self.drag_delta);
                let old = wirecraft_core::drawing::composite_padding(n);
                let (left, top) = (new.left - old.left, new.top - old.top);
                Rect {
                    x: r.x - left,
                    y: r.y - top,
                    width: r.width + left + new.right - old.right,
                    height: r.height + top + new.bottom - old.bottom,
                }
            });
            let selection = self.selection_start.zip(self.selection_end);
            let rubber = self.rubber_band();
            let link_ends = self.link_end_handles();
            let sheets = self.sheet_contours.clone();
            let sheet_blocks = self.sheet_blocks.clone();
            let parent_name = self.snapshot.document.qualified_name(&self.parent);
            content = content
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .p_2()
                        .child(
                            active(button("select", "Select"), self.mode == Mode::Select).on_click(
                                cx.listener(|this, _, w, cx| this.select_mode(&SelectMode, w, cx)),
                            ),
                        )
                        .child(
                            active(button("node", "Node"), self.mode == Mode::Node).on_click(
                                cx.listener(|this, _, w, cx| this.node_mode(&NodeMode, w, cx)),
                            ),
                        )
                        .child(
                            active(button("type", "Type"), self.mode == Mode::Type).on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.mode = Mode::Type;
                                    cx.notify();
                                }),
                            ),
                        )
                        .child(
                            active(button("link", "Link"), self.mode == Mode::Link).on_click(
                                cx.listener(|this, _, w, cx| this.link_mode(&LinkMode, w, cx)),
                            ),
                        )
                        .child(
                            active(button("port", "Port"), self.mode == Mode::Port).on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.mode = Mode::Port;
                                    this.status =
                                    "Click to place a port, then link it to an entity in Link mode"
                                        .into();
                                    cx.notify();
                                }),
                            ),
                        )
                        .child(
                            button("fit", "Fit")
                                .on_click(cx.listener(|this, _, w, cx| this.fit(&Fit, w, cx))),
                        )
                        .child(active(button("sheet", "Sheet"), self.show_sheets).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.show_sheets = !this.show_sheets;
                                this.refresh_sheets();
                                cx.notify();
                            }),
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
                        .on_scroll_wheel(cx.listener(Self::wheel))
                        .child(
                            canvas(
                                move |b, _, _| {
                                    bounds.set(b);
                                    b
                                },
                                move |b, _, window, cx| {
                                    scene.paint(
                                        b, view, &selected, &moving, drag, &errors, window, cx,
                                    );
                                    for end in &link_ends {
                                        scene::paint_link_end(*end, b.origin, view, window);
                                    }
                                    if let Some(band) = &rubber {
                                        scene::paint_rubber_band(band, b.origin, view, window);
                                    }
                                    for page in &sheets {
                                        let p = view.screen([page.x, page.y]);
                                        scene::dashed_outline(
                                            Bounds::new(
                                                b.origin + point(px(p[0] as f32), px(p[1] as f32)),
                                                size(
                                                    px((page.width * view.zoom) as f32),
                                                    px((page.height * view.zoom) as f32),
                                                ),
                                            ),
                                            1.,
                                            10.,
                                            6.,
                                            rgb(0x9bb0c3),
                                            window,
                                        );
                                    }
                                    for block in &sheet_blocks {
                                        scene.paint_sheet_block(block, b.origin, view, window, cx);
                                    }
                                    if let Some(r) = resize {
                                        let p = view.screen([r.x, r.y]);
                                        window.paint_quad(quad(
                                            Bounds::new(
                                                b.origin + point(px(p[0] as f32), px(p[1] as f32)),
                                                size(
                                                    px((r.width.max(0.) * view.zoom) as f32),
                                                    px((r.height.max(0.) * view.zoom) as f32),
                                                ),
                                            ),
                                            px(0.),
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
                                            if a[0] <= z[0] {
                                                BorderStyle::Solid
                                            } else {
                                                BorderStyle::Dashed
                                            },
                                        ));
                                    }
                                },
                            )
                            .size_full(),
                        ),
                );
        } else if self.tab == "Editor" {
            content = content.child(self.render_editor(window, cx));
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
              .child(button("analyze-c17","Analyze C17/Rust graph").on_click(cx.listener(|this,_,_,_|this.tool("editor.analyze_generated")))).child(button("stop-graph","Stop graph analyzer").on_click(cx.listener(|this,_,_,_|this.tool("editor.stop_generated")))).child(button("stop-lsp","Stop standalone").on_click(cx.listener(|this,_,_,_|this.tool("lsp.stop")))))
             .child(div().id("tool-output").flex_1().overflow_y_scroll().p_4()
                .children(self.generated_diagnostics.iter().enumerate().filter(|(_,d)|d["source"].is_object()).map(|(i,d)|{
                    let diagnostic=d.clone();let source=&d["source"];let label=format!("{}:{}:{} · {}",source["nodeId"].as_str().unwrap_or(""),source["line"],source["column"],d["message"].as_str().unwrap_or(""));
                    div().id(i).p_2().mb_2().bg(crate::chrome::color(0x3c2831)).cursor_pointer().on_click(cx.listener(move|this,_,window,cx|this.reveal_diagnostic(&diagnostic,window,cx))).child(label)
                })).child(self.output.clone()));
        } else {
            content = content.child(self.render_mcp(cx));
        }
        let draft_notice =
            (self.has_draft(cx) || self.editor_request_pending).then(|| {
                let name = self
                    .editor_entity
                    .as_ref()
                    .and_then(|id| self.snapshot.document.nodes.get(id))
                    .map(|n| n.name.as_str())
                    .unwrap_or("entity");
                div()
                    .id("editor-draft-notice")
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .bg(crate::chrome::color(0x443a22))
                    .child(div().flex_1().min_w_0().truncate().child(
                        if self.editor_request_pending {
                            format!("Applying edits to {name}…")
                        } else {
                            format!(
                                "Unapplied edits to {name} — apply or discard to change selection"
                            )
                        },
                    ))
                    .child(
                        button("apply-retained-draft", "Apply · Ctrl Enter")
                            .on_click(cx.listener(|this, _, w, cx| this.apply(&Apply, w, cx))),
                    )
                    .child(
                        button("discard-retained-draft", "Discard edits").on_click(cx.listener(
                            |this, _, w, cx| {
                                if !this.editor_request_pending {
                                    this.pending_draft = None;
                                    this.load_editor(w, cx);
                                    this.persist_ui(cx);
                                    this.status = "Editor edits discarded".into();
                                    cx.notify();
                                }
                            },
                        )),
                    )
                    .child(
                        button("show-retained-draft", "Show editor").on_click(cx.listener(
                            |this, _, _, cx| {
                                this.tab = "Editor".into();
                                this.persist_ui(cx);
                                cx.notify();
                            },
                        )),
                    )
            });
        div().capture_action(cx.listener(|this,_:&gpui_component::input::MoveUp,w,cx|{if !this.completion_action("up",w,cx){cx.propagate();}})).capture_action(cx.listener(|this,_:&gpui_component::input::MoveDown,w,cx|{if !this.completion_action("down",w,cx){cx.propagate();}})).capture_action(cx.listener(|this,action:&gpui_component::input::Enter,w,cx|{if action.secondary && this.menu.is_none(){cx.stop_propagation();this.completion=None;this.apply(&Apply,w,cx);}else if !this.completion_action("enter",w,cx){cx.propagate();}})).capture_action(cx.listener(|this,_:&gpui_component::input::Escape,w,cx|{if !this.completion_action("escape",w,cx){cx.propagate();}})).capture_action(cx.listener(|this,_:&gpui_component::input::IndentInline,w,cx|{if !this.completion_action("tab",w,cx){cx.propagate();}})).key_context("Wirecraft").track_focus(&self.focus).on_action(cx.listener(Self::open)).on_action(cx.listener(Self::save)).on_action(cx.listener(Self::save_as)).on_action(cx.listener(Self::new_project)).on_action(cx.listener(Self::undo)).on_action(cx.listener(Self::redo)).on_action(cx.listener(Self::delete)).on_action(cx.listener(Self::select_mode)).on_action(cx.listener(Self::node_mode)).on_action(cx.listener(Self::link_mode)).on_action(cx.listener(Self::fit)).on_action(cx.listener(Self::apply)).on_action(cx.listener(Self::copy)).on_action(cx.listener(Self::cut)).on_action(cx.listener(Self::paste)).on_action(cx.listener(Self::palette)).on_action(cx.listener(Self::show_export)).on_action(cx.listener(Self::show_options)).on_action(cx.listener(Self::request_completion)).on_action(cx.listener(Self::go_to_definition)).on_action(cx.listener(Self::find_references)).on_action(|_:&Quit,_,cx|cx.quit()).relative().size_full().flex().flex_col().bg(crate::chrome::color(0x10161e)).text_color(crate::chrome::color(0xe1e9ef)).text_sm().child(self.render_menubar(cx)).child(toolbar).child(tabs).child(div().flex().flex_1().min_h_0().when(["Designer","Editor","Tools"].contains(&self.tab.as_str()), |b| b.child(sidebar).child(self.panel_separator(false,cx))).child(content)).children(draft_notice).child(div().px_3().py_2().flex().items_center().gap_3().bg(crate::chrome::color(0x171d25)).child(div().flex_1().min_w_0().truncate().child(format!("{}   |   rev {}   |   {:.0}%   |   Wheel: zoom · Middle drag: pan · Shift: add · Ctrl: remove",self.status,self.snapshot.revision,self.viewport.zoom*100.))).child(button("compiler-issues",format!("{} · {} issue{}",if self.compiler_inflight {"Checking"}else{"Compiler"},self.compiler_issues().len(),if self.compiler_issues().len()==1 {""}else{"s"})).on_click(cx.listener(|this,_,_,cx|{this.menu=Some("Diagnostics".into());cx.notify();})))).children(self.render_completion(window,cx)).children(self.render_menu(window,cx)).child(self.pointer_capture(cx))
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
                println!("wirecraft-desktop [PROJECT.orch] [--state-dir DIRECTORY] [--version]");
                return Ok(());
            }
            // Release packaging checks installed commands with this line.
            "--version" | "-V" => {
                println!("Wirecraft {}", env!("CARGO_PKG_VERSION"));
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
    editor_tokens::register_languages();
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
                KeyBinding::new("f12", GoToDefinition, Some("Wirecraft")),
                KeyBinding::new("shift-f12", FindReferences, Some("Wirecraft")),
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
