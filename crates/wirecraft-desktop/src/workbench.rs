//! Screenshot-driven workbench views. Mutations use the shared application service.
use super::*;
use gpui_component::Sizable;
use std::collections::BTreeMap;
/// Menubar groups rendered as dropdowns anchored under their labels.
pub(super) const MENUBAR: [&str; 7] = [
    "File",
    "Edit",
    "Graph",
    "Generate",
    "MCP",
    "AI Support",
    "Help",
];

pub(super) fn property_changes(
    current: &str,
    baseline: &str,
) -> Result<serde_json::Map<String, Value>, String> {
    let current: Value = serde_json::from_str(current).map_err(|e| format!("Properties: {e}"))?;
    let baseline: Value =
        serde_json::from_str(baseline).map_err(|e| format!("Loaded properties: {e}"))?;
    let current = current
        .as_object()
        .ok_or("Properties must be a JSON object")?;
    let baseline = baseline
        .as_object()
        .ok_or("Loaded properties must be a JSON object")?;
    Ok(current
        .iter()
        .filter(|(key, value)| baseline.get(*key) != Some(*value))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect())
}
pub struct FormField {
    pub path: &'static str,
    pub(super) label: &'static str,
    pub input: Entity<InputState>,
    pub baseline: String,
}
pub fn make_fields(window: &mut Window, cx: &mut Context<Wirecraft>) -> Vec<FormField> {
    [
        ("/nameDetail", "Name detail"),
        ("/technology/languageId", "Language"),
        ("/technology/technologyId", "Technology"),
        ("/fileLayoutStrategyId", "File layout strategy"),
        ("/status", "Status"),
        ("/responsible", "Responsible"),
        ("/assignee", "Assignee"),
        ("/text/sourceLanguageId", "Declaration language"),
        ("/text/specificationLanguageId", "Specification language"),
        ("/text/aiInstructionsLanguageId", "Usage language"),
        ("/text/testsLanguageId", "Tests language"),
        ("/text/initializationLanguageId", "Instantiation language"),
        ("/link/sourcePortName", "Source port"),
        ("/link/targetPortName", "Target port"),
        ("/link/transportKind", "Transport"),
        ("/link/interactionKind", "Interaction"),
        ("/link/typeDefinitionId", "Type definition ID"),
        ("/link/typeName", "Payload type"),
        ("/masterRevision/name", "Master revision"),
        ("/masterRevision/date", "Master revision date"),
    ]
    .into_iter()
    .map(|(path, label)| FormField {
        path,
        label,
        input: cx.new(|cx| InputState::new(window, cx).placeholder("Inherited / none")),
        baseline: String::new(),
    })
    .collect()
}
const STATUSES: [&str; 6] = [
    "BUSINESS",
    "FUNCTIONAL",
    "IMPLEMENTATION",
    "TESTING",
    "DEPLOYED",
    "CANCELED",
];
#[derive(Clone)]
pub(super) struct Ticket {
    pub id: String,
    pub name: String,
}
impl Render for Ticket {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_3()
            .rounded_md()
            .bg(crate::chrome::color(0x255452))
            .text_color(crate::chrome::color(0xe1e9ef))
            .child(self.name.clone())
    }
}
#[derive(Clone, Copy)]
struct Command {
    id: &'static str,
    group: &'static str,
    label: &'static str,
    shortcut: &'static str,
    icon: &'static str,
}
const COMMANDS: &[Command] = &[
    Command {
        id: "new",
        group: "File",
        label: "New",
        shortcut: "Ctrl N",
        icon: "new",
    },
    Command {
        id: "open",
        group: "File",
        label: "Open…",
        shortcut: "Ctrl O",
        icon: "open",
    },
    Command {
        id: "save",
        group: "File",
        label: "Save",
        shortcut: "Ctrl S",
        icon: "save",
    },
    Command {
        id: "save-as",
        group: "File",
        label: "Save As…",
        shortcut: "Ctrl Shift S",
        icon: "save",
    },
    Command {
        id: "archetype",
        group: "File",
        label: "Save Model as Archetype…",
        shortcut: "",
        icon: "type",
    },
    Command {
        id: "export",
        group: "File",
        label: "Export…",
        shortcut: "Ctrl P",
        icon: "sheet",
    },
    Command {
        id: "options",
        group: "File",
        label: "Options…",
        shortcut: "Ctrl ,",
        icon: "settings",
    },
    Command {
        id: "quit",
        group: "File",
        label: "Quit",
        shortcut: "Ctrl Q",
        icon: "close",
    },
    Command {
        id: "undo",
        group: "Edit",
        label: "Undo",
        shortcut: "Ctrl Z",
        icon: "undo",
    },
    Command {
        id: "redo",
        group: "Edit",
        label: "Redo",
        shortcut: "Ctrl Y",
        icon: "redo",
    },
    Command {
        id: "cut",
        group: "Edit",
        label: "Cut",
        shortcut: "Ctrl X",
        icon: "cut",
    },
    Command {
        id: "copy",
        group: "Edit",
        label: "Copy",
        shortcut: "Ctrl C",
        icon: "copy",
    },
    Command {
        id: "paste",
        group: "Edit",
        label: "Paste",
        shortcut: "Ctrl V",
        icon: "paste",
    },
    Command {
        id: "left",
        group: "Edit",
        label: "Align Left",
        shortcut: "",
        icon: "node",
    },
    Command {
        id: "right",
        group: "Edit",
        label: "Align Right",
        shortcut: "",
        icon: "node",
    },
    Command {
        id: "top",
        group: "Edit",
        label: "Align Top",
        shortcut: "",
        icon: "node",
    },
    Command {
        id: "bottom",
        group: "Edit",
        label: "Align Bottom",
        shortcut: "",
        icon: "node",
    },
    Command {
        id: "distribute-horizontal",
        group: "Edit",
        label: "Distribute Evenly Horizontally",
        shortcut: "",
        icon: "node",
    },
    Command {
        id: "distribute-vertical",
        group: "Edit",
        label: "Distribute Evenly Vertically",
        shortcut: "",
        icon: "node",
    },
    Command {
        id: "fit",
        group: "Graph",
        label: "Zoom Extents",
        shortcut: "",
        icon: "fit",
    },
    Command {
        id: "sheet-preview",
        group: "Sheet",
        label: "Toggle Preview",
        shortcut: "",
        icon: "sheet",
    },
    Command {
        id: "select",
        group: "Graph",
        label: "Select Mode",
        shortcut: "Escape",
        icon: "select",
    },
    Command {
        id: "node",
        group: "Graph",
        label: "Node Mode",
        shortcut: "",
        icon: "node",
    },
    Command {
        id: "type",
        group: "Graph",
        label: "Type Mode",
        shortcut: "",
        icon: "type",
    },
    Command {
        id: "link",
        group: "Graph",
        label: "Link Mode",
        shortcut: "",
        icon: "link",
    },
    Command {
        id: "reference",
        group: "Graph",
        label: "Insert Component Reference…",
        shortcut: "",
        icon: "node",
    },
    Command {
        id: "update-reference",
        group: "Graph",
        label: "Update Component Reference",
        shortcut: "",
        icon: "node",
    },
    Command {
        id: "delete",
        group: "Graph",
        label: "Delete Selection",
        shortcut: "Delete",
        icon: "close",
    },
    Command {
        id: "palette",
        group: "Graph",
        label: "Open Palette",
        shortcut: "Ctrl Shift P",
        icon: "search",
    },
    Command {
        id: "generate",
        group: "Generate",
        label: "Generate Project (effective technology)",
        shortcut: "",
        icon: "code",
    },
    Command {
        id: "compiler-export",
        group: "Generate",
        label: "Export Compiler from Model Overrides…",
        shortcut: "",
        icon: "code",
    },
    Command {
        id: "files",
        group: "Generate",
        label: "Export Literal Files",
        shortcut: "",
        icon: "code",
    },
    Command {
        id: "mcp",
        group: "MCP",
        label: "Manage Server…",
        shortcut: "",
        icon: "server",
    },
    Command {
        id: "fiches",
        group: "AI Support",
        label: "Copy Selected Component Fiches as Markdown",
        shortcut: "",
        icon: "copy",
    },
    Command {
        id: "ai-source",
        group: "AI Support",
        label: "Generate Source Code · Copy Prompt",
        shortcut: "",
        icon: "spark",
    },
    Command {
        id: "ai-specification",
        group: "AI Support",
        label: "Tidy and Format Specifications · Copy Prompt",
        shortcut: "",
        icon: "spark",
    },
    Command {
        id: "ai-tests",
        group: "AI Support",
        label: "Generate Test Data · Copy Prompt",
        shortcut: "",
        icon: "spark",
    },
    Command {
        id: "about",
        group: "Help",
        label: "About",
        shortcut: "",
        icon: "type",
    },
    Command {
        id: "guide",
        group: "Help",
        label: "MCP Agent Guide…",
        shortcut: "",
        icon: "sheet",
    },
];
impl Wirecraft {
    pub(super) fn panel_separator(&self, right: bool, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id(if right {
                "inspector-divider"
            } else {
                "hierarchy-divider"
            })
            .w(px(5.))
            .flex_none()
            .bg(crate::chrome::color(0x29323d))
            .cursor_col_resize()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.panel_drag = Some((
                        right,
                        f32::from(event.position.x),
                        if right {
                            this.inspector_width
                        } else {
                            this.sidebar_width
                        },
                    ));
                    cx.stop_propagation();
                }),
            )
            .into_any_element()
    }
    pub(super) fn move_panel(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some((right, start, width)) = self.panel_drag {
            let delta = f32::from(event.position.x) - start;
            if right {
                self.inspector_width = (width - delta).clamp(280., 650.);
            } else {
                self.sidebar_width = (width + delta).clamp(180., 500.);
            }
            cx.notify();
        }
    }
    pub(super) fn end_panel(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.panel_drag.take().is_some() {
            self.persist_ui(cx);
            cx.notify();
        }
    }

    pub(super) fn merged_properties(&self, cx: &App) -> String {
        let raw = self.properties.read(cx).value().to_string();
        let Ok(mut value) = serde_json::from_str::<Value>(&raw) else {
            return raw;
        };
        let mut changed = false;
        for f in &self.inspector {
            let text = f.input.read(cx).value().to_string();
            if text == f.baseline {
                continue;
            }
            changed = true;
            let parts: Vec<_> = f.path.trim_start_matches('/').split('/').collect();
            let mut cursor = &mut value;
            for part in &parts[..parts.len() - 1] {
                cursor = &mut cursor[*part];
            }
            cursor[parts[parts.len() - 1]] =
                if ["/status", "/responsible", "/assignee"].contains(&f.path) && text.is_empty() {
                    Value::Null
                } else {
                    json!(text)
                };
        }
        let before = value.clone();
        let _ = self.tables.merge(&mut value, cx);
        if changed || value != before {
            serde_json::to_string_pretty(&value).unwrap_or(raw)
        } else {
            raw
        }
    }
    pub(super) fn palette(&mut self, _: &CommandPalette, w: &mut Window, cx: &mut Context<Self>) {
        self.menu = Some("Commands".into());
        self.palette_index = 0;
        self.palette_scroll.scroll_to_item(0);
        self.query.update(cx, |s, cx| {
            s.set_value("", w, cx);
            s.focus(w, cx);
        });
        cx.notify();
    }
    pub(super) fn show_export(&mut self, _: &ShowExport, _: &mut Window, cx: &mut Context<Self>) {
        self.tab = "Export".into();
        self.refresh_preview();
        self.tool("printers.list");
        self.menu = None;
        cx.notify();
    }
    pub(super) fn show_options(&mut self, _: &ShowOptions, _: &mut Window, cx: &mut Context<Self>) {
        self.tab = "Options".into();
        self.menu = None;
        cx.notify();
    }
    pub(super) fn submit_command(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let query = self.query.read(cx).value().to_lowercase();
        if let Some(command) = COMMANDS
            .iter()
            .filter(|c| {
                format!("{} {}", c.group, c.label)
                    .to_lowercase()
                    .contains(&query)
            })
            .nth(self.palette_index)
        {
            self.command(command.id, w, cx);
            self.focus.focus(w);
        }
    }
    fn command(&mut self, id: &str, w: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        match id {
            "new" => self.new_project(&New, w, cx),
            "open" => self.open(&Open, w, cx),
            "save" => self.save(&Save, w, cx),
            "save-as" => self.save_as(&SaveAs, w, cx),
            "undo" => self.undo(&Undo, w, cx),
            "redo" => self.redo(&Redo, w, cx),
            "copy" => self.copy(&CopyEntities, w, cx),
            "cut" => self.cut(&CutEntities, w, cx),
            "paste" => self.paste(&PasteEntities, w, cx),
            "fit" => self.fit(&Fit, w, cx),
            "sheet-preview" => {
                if self.tab == "Export" {
                    self.tab = "Designer".into();
                } else {
                    self.show_export(&ShowExport, w, cx);
                }
            }
            "select" => self.select_mode(&SelectMode, w, cx),
            "node" => self.node_mode(&NodeMode, w, cx),
            "link" => self.link_mode(&LinkMode, w, cx),
            "type" => self.mode = Mode::Type,
            "delete" => self.delete(&DeleteSelection, w, cx),
            "reference" => self.reference_dialog(w, cx),
            "update-reference" => {
                let references: Vec<String> = self
                    .selected
                    .iter()
                    .filter(|id| {
                        self.snapshot
                            .document
                            .nodes
                            .get(*id)
                            .is_some_and(wirecraft_core::components::is_reference)
                    })
                    .cloned()
                    .collect();
                if references.is_empty() {
                    self.status = "Select a component Reference to update".into();
                }
                for id in references {
                    self.client
                        .send("components.update_reference", json!({"nodeId":id}));
                }
            }
            "export" => self.show_export(&ShowExport, w, cx),
            "options" => self.show_options(&ShowOptions, w, cx),
            "palette" => self.palette(&CommandPalette, w, cx),
            "generate" => self.artifact_dialog("project", w, cx),
            "compiler-export" => self.artifact_dialog("compiler", w, cx),
            "files" => self.artifact_dialog("files", w, cx),
            "archetype" => self.artifact_dialog("archetype", w, cx),
            "mcp" => {
                self.tab = "MCP".into();
                self.tool("mcp.status");
            }
            "ai-source" | "ai-specification" | "ai-tests" => {
                if self.has_draft(cx) {
                    self.status = "Apply the editor draft before preparing assistance".into();
                } else {
                    self.client.send(
                        "ai.submit",
                        json!({"task":id.trim_start_matches("ai-"),"nodeIds":self.selected}),
                    );
                }
            }
            "fiches" => self
                .client
                .send("export.markdown", json!({"nodeIds":self.selected})),
            "about" => self.menu = Some("About".into()),
            "guide" => self.menu = Some("Guide".into()),
            "quit" => cx.quit(),
            "left"
            | "right"
            | "top"
            | "bottom"
            | "distribute-horizontal"
            | "distribute-vertical" => {
                if self.has_draft(cx) {
                    self.status = "Apply the editor draft before alignment".into();
                } else {
                    self.client.send(
                        "entities.align",
                        json!({"nodeIds":self.selected,"operation":id}),
                    );
                }
            }
            _ => {}
        }
        self.persist_ui(cx);
        cx.notify();
    }
    pub(super) fn render_menubar(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut bar = div()
            .h(px(32.))
            .flex()
            .items_center()
            .gap_1()
            .px_3()
            .bg(crate::chrome::color(0x10161e))
            .border_b_1()
            .border_color(crate::chrome::color(0x29323d));
        for label in MENUBAR {
            let anchors = self.menu_anchors.clone();
            let open = self.menu.as_deref() == Some(label);
            bar = bar.child(
                div()
                    .id(label)
                    .relative()
                    .px_3()
                    .py_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .when(open, |b| b.bg(crate::chrome::color(0x29323d)))
                    .hover(|s| s.bg(crate::chrome::color(0x29323d)))
                    // Menus open on press; pressing the open menu's label closes it.
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.menu = if this.menu.as_deref() == Some(label) {
                                None
                            } else {
                                Some(label.into())
                            };
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
                    // While a menubar dropdown is open, hovering another label switches to it.
                    .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        if *hovered
                            && this
                                .menu
                                .as_deref()
                                .is_some_and(|m| m != label && MENUBAR.contains(&m))
                        {
                            this.menu = Some(label.into());
                            cx.notify();
                        }
                    }))
                    .child(label)
                    .child(
                        canvas(
                            move |b, _, _| {
                                anchors.borrow_mut().insert(label, b);
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .inset_0(),
                    ),
            );
        }
        let bar = bar
            .child(div().flex_1())
            .child(
                div()
                    .id("command-search")
                    .flex()
                    .gap_2()
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, w, cx| this.palette(&CommandPalette, w, cx)))
                    .child(icons::icon("search"))
                    .child("Commands  Ctrl Shift P"),
            )
            .into_any_element();
        if cfg!(target_os = "linux") {
            gpui_component::TitleBar::new()
                .h(px(32.))
                .pl_0()
                .child(div().flex_1().min_w_0().child(bar))
                .into_any_element()
        } else {
            bar
        }
    }
    pub(super) fn render_menu(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let group = self.menu.as_deref()?;
        let context_menu = group.starts_with("Context");
        let dropdown = MENUBAR.contains(&group);
        let padding = gpui_component::window_paddings(window);
        let content_size = size(
            window.viewport_size().width - padding.left - padding.right,
            window.viewport_size().height - padding.top - padding.bottom,
        );
        let mut panel = div()
            .id("command-panel")
            .occlude()
            .overflow_hidden()
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .w(px(520.))
            .max_h(px(650.))
            .flex()
            .flex_col()
            .p_2()
            .gap_1()
            .rounded_lg()
            .border_1()
            .border_color(crate::chrome::color(0x40505c))
            .bg(crate::chrome::color(0x1c242e))
            .shadow_lg();
        panel = panel.capture_key_down(cx.listener(|this, event: &KeyDownEvent, w, cx| {
            if event.keystroke.key == "escape" {
                this.menu = None;
                this.focus.focus(w);
                cx.stop_propagation();
                cx.notify();
            } else if this.menu.as_deref() == Some("Commands")
                && ["up", "down"].contains(&event.keystroke.key.as_str())
            {
                let query = this.query.read(cx).value().to_lowercase();
                let count = COMMANDS
                    .iter()
                    .filter(|c| {
                        format!("{} {}", c.group, c.label)
                            .to_lowercase()
                            .contains(&query)
                    })
                    .count();
                this.palette_index = if event.keystroke.key == "up" {
                    this.palette_index.saturating_sub(1)
                } else {
                    (this.palette_index + 1).min(count.saturating_sub(1))
                };
                this.palette_scroll.scroll_to_item(this.palette_index);
                cx.stop_propagation();
                cx.notify();
            }
        }));
        panel = panel.when(dropdown, |p| p.w(px(320.)).p_1().rounded_md());
        panel = panel.when(!dropdown, |panel| {
            panel.child(
                div()
                    .flex()
                    .items_center()
                    .p_2()
                    .child(div().flex_1().text_lg().child(group.to_string()))
                    .child(button("close-menu", "Close").on_click(cx.listener(
                        |this, _, w, cx| {
                            this.menu = None;
                            this.focus.focus(w);
                            cx.notify();
                        },
                    ))),
            )
        });
        if group == "Type reference" {
            let mut choices = [
                ("string", "string"),
                ("boolean", "boolean"),
                ("number", "number"),
                ("date", "date"),
                ("int", "int"),
                ("float", "float"),
                ("double", "double"),
                ("char *", "char *"),
                ("void *", "void *"),
            ]
            .into_iter()
            .map(|(id, name)| (id.to_string(), name.to_string()))
            .collect::<Vec<_>>();
            choices.extend(
                self.snapshot
                    .document
                    .nodes
                    .values()
                    .filter(|n| n.kind == wirecraft_core::NodeKind::Type)
                    .map(|n| (n.id.clone(), self.snapshot.document.qualified_name(&n.id))),
            );
            panel = panel.child(
                div()
                    .id("type-options")
                    .max_h(px(480.))
                    .overflow_y_scroll()
                    .children(choices.into_iter().enumerate().map(|(i, (id, label))| {
                        div()
                            .id(i)
                            .flex()
                            .gap_2()
                            .p_2()
                            .cursor_pointer()
                            .hover(|s| s.bg(crate::chrome::color(0x293846)))
                            .child(icons::icon("type"))
                            .child(label)
                            .on_click(cx.listener(move |this, _, w, cx| {
                                let input = if let Some((index, key)) = this.type_choice {
                                    this.tables.types.get(index).map(|row| {
                                        if key {
                                            row.key_type.clone()
                                        } else {
                                            row.type_id.clone()
                                        }
                                    })
                                } else {
                                    this.inspector
                                        .iter()
                                        .find(|f| f.path == "/link/typeDefinitionId")
                                        .map(|f| f.input.clone())
                                };
                                if let Some(input) = input {
                                    input.update(cx, |s, cx| s.set_value(id.clone(), w, cx));
                                }
                                this.menu = None;
                                this.persist_ui(cx);
                                cx.notify();
                            }))
                    })),
            );
        } else if ["Designer font", "Code font"].contains(&group) {
            let path = if group == "Designer font" {
                "/appearance/designerFont"
            } else {
                "/appearance/editorFont"
            };
            panel = panel.child(
                uniform_list(
                    "font-choices",
                    self.font_choices.len(),
                    cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                        range
                            .map(|i| {
                                let name = this.font_choices[i].clone();
                                div()
                                    .id(i)
                                    .h(px(30.))
                                    .px_2()
                                    .cursor_pointer()
                                    .font_family(name.clone())
                                    .child(name.clone())
                                    .on_click(cx.listener(move |this, _, w, cx| {
                                        if let Some(field) =
                                            this.options.fields.iter().find(|f| f.path == path)
                                        {
                                            field.input.update(cx, |s, cx| {
                                                s.set_value(name.clone(), w, cx)
                                            });
                                        }
                                        this.menu = None;
                                        cx.notify();
                                    }))
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .h(px(480.)),
            );
        } else if ["Sheet format", "Document format", "Sheet scale"].contains(&group) {
            let choices: Vec<String> = if group == "Sheet scale" {
                [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000]
                    .into_iter()
                    .map(|n| n.to_string())
                    .collect()
            } else {
                wirecraft_core::sheets::FORMATS
                    .iter()
                    .filter(|f| {
                        group != "Document format" || (**f != "Auto" && !f.contains("roll"))
                    })
                    .map(|s| s.to_string())
                    .collect()
            };
            panel = panel.child(div().id("sheet-choices").overflow_y_scroll().children(
                choices.into_iter().enumerate().map(|(i, choice)| {
                    let group = group.to_string();
                    div()
                        .id(i)
                        .p_2()
                        .cursor_pointer()
                        .hover(|s| s.bg(crate::chrome::color(0x2d3b48)))
                        .child(choice.clone())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            match group.as_str() {
                                "Sheet scale" => {
                                    this.export_settings.sheet.scale = choice.parse().unwrap_or(1)
                                }
                                "Document format" => {
                                    this.export_settings.document.format = choice.clone()
                                }
                                _ => this.export_settings.sheet.format = choice.clone(),
                            };
                            this.menu = None;
                            this.export_page = 0;
                            this.refresh_preview();
                            this.persist_ui(cx);
                            cx.notify();
                        }))
                }),
            ));
        } else if group == "About" || group == "Guide" {
            let text = if group == "About" {
                format!(
                    "Wirecraft {}\nRust + GPUI\nMigrated from Threadworks 2.6.7\nSource: 7d392d3a0e30b86445e00e7c03032a5188b26593",
                    env!("CARGO_PKG_VERSION")
                )
            } else {
                wirecraft_runtime::mcp::GUIDE.into()
            };
            panel = panel.child(
                div()
                    .id("help-text")
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_4()
                    .child(text),
            );
            if group == "Guide" {
                panel = panel.child(button("copy-guide", "Copy agent guide").on_click(
                    |_, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(
                            wirecraft_runtime::mcp::GUIDE.into(),
                        ));
                    },
                ));
            }
        } else if group == "Diagnostics" {
            panel = panel.w(px(900.)).child(
                button("recheck-compiler", "Check compilation again")
                    .on_click(cx.listener(|this, _, _, cx| this.queue_compiler_check(cx))),
            );
            let issues = self.compiler_issues();
            let mut list = div()
                .id("compiler-diagnostics")
                .h(px(530.))
                .overflow_y_scroll();
            if issues.is_empty() {
                list = list.child(div().p_3().child(if self.compiler_inflight {
                    "Checking generated source…"
                } else {
                    "No compiler errors or warnings."
                }));
            }
            for (i, diagnostic) in issues.iter().enumerate() {
                let location = &diagnostic["source"];
                let name = location["nodeId"]
                    .as_str()
                    .map(|id| self.snapshot.document.qualified_name(id))
                    .unwrap_or_else(|| "Compiler service".into());
                let message = format!(
                    "{}: {}",
                    if diagnostic["severity"] == 1 {
                        "Error"
                    } else {
                        "Warning"
                    },
                    diagnostic["message"].as_str().unwrap_or("")
                );
                let detail = format!(
                    "{} · {} · {}:{}",
                    name,
                    location["section"].as_str().unwrap_or("source"),
                    location["line"].as_u64().unwrap_or(1),
                    location["column"].as_u64().unwrap_or(1)
                );
                let diagnostic = diagnostic.clone();
                list = list.child(
                    div()
                        .id(i)
                        .p_3()
                        .border_b_1()
                        .border_color(crate::chrome::color(0x40505c))
                        .cursor_pointer()
                        .hover(|s| s.bg(crate::chrome::color(0x293846)))
                        .child(
                            div()
                                .text_color(crate::chrome::color(0xeab779))
                                .child(message),
                        )
                        .child(div().text_xs().child(detail))
                        .on_click(cx.listener(move |this, _, w, cx| {
                            this.reveal_diagnostic(&diagnostic, w, cx);
                            if !this.has_draft(cx) {
                                this.menu = None;
                            }
                            cx.notify();
                        })),
                );
            }
            panel = panel.child(list);
        } else if let Some(category) = group.strip_prefix("Context:Archetypes") {
            panel = panel.child(button("context-back", "‹ Back").on_click(cx.listener(
                |this, _, _, cx| {
                    this.menu = Some(if this.menu.as_deref() == Some("Context:Archetypes") {
                        "Context".into()
                    } else {
                        "Context:Archetypes".into()
                    });
                    cx.notify();
                },
            )));
            let mut list = div()
                .id("context-archetypes")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll();
            if category.is_empty() {
                let groups = self
                    .archetypes
                    .iter()
                    .map(|a| a["group"].as_str().unwrap_or("Other").to_string())
                    .collect::<BTreeSet<_>>();
                for category in groups {
                    let text = format!("{category} ›");
                    list = list.child(
                        button(SharedString::from(text.clone()), text.clone()).on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.menu = Some(format!("Context:Archetypes:{category}"));
                                cx.notify();
                            }),
                        ),
                    );
                }
            } else {
                let category = category.trim_start_matches(':');
                for archetype in self
                    .archetypes
                    .iter()
                    .filter(|a| a["group"].as_str().unwrap_or("Other") == category)
                {
                    let id = archetype["id"].as_str().unwrap_or("").to_string();
                    let name = archetype["name"].as_str().unwrap_or(&id).to_string();
                    list = list.child(button(SharedString::from(id.clone()), name).on_click(
                        cx.listener(move |this, _, _, cx| {
                            if this.has_draft(cx) {
                                this.status =
                                    "Apply the editor draft before inserting an archetype".into();
                            } else {
                                this.client.send(
                                    "archetypes.import",
                                    json!({"id":id,"parentId":this.parent}),
                                );
                                this.menu = None;
                            }
                            cx.notify();
                        }),
                    ));
                }
            }
            panel = panel.child(list);
        } else {
            if group == "Commands" {
                panel = panel.child(Input::new(&self.query));
            }
            let query = if group == "Commands" {
                self.query.read(cx).value().to_lowercase()
            } else {
                String::new()
            };
            panel = panel.child(
                div()
                    .id("command-list")
                    .when(dropdown, |list| list.max_h(px(560.)))
                    .when(!dropdown, |list| {
                        list.h(px(if context_menu {
                            (f32::from(content_size.height) - 32.).min(650.) - 74.
                        } else {
                            550.
                        }))
                    })
                    .flex_shrink_0()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.palette_scroll)
                    .children(
                        COMMANDS
                            .iter()
                            .filter(|c| {
                                (["Commands", "Context"].contains(&group) || c.group == group)
                                    && format!("{} {}", c.group, c.label)
                                        .to_lowercase()
                                        .contains(&query)
                            })
                            .enumerate()
                            .map(|(index, c)| {
                                let id = c.id;
                                div()
                                    .id(id)
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .p_2()
                                    .rounded_sm()
                                    .cursor_pointer()
                                    .when(group == "Commands" && index == self.palette_index, |s| {
                                        s.bg(crate::chrome::color(0x255452))
                                    })
                                    .hover(|s| s.bg(crate::chrome::color(0x2d3b48)))
                                    .on_click(
                                        cx.listener(move |this, _, w, cx| this.command(id, w, cx)),
                                    )
                                    .child(icons::icon(c.icon))
                                    .child(div().flex_1().child(
                                        if ["Commands", "Context"].contains(&group) {
                                            format!("{}: {}", c.group, c.label)
                                        } else {
                                            c.label.into()
                                        },
                                    ))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(crate::chrome::color(0x8a9aab))
                                            .child(c.shortcut),
                                    )
                            }),
                    )
                    .when(group == "Context", |list| {
                        list.child(
                            button("context-archetype-catalog", "Archetypes ›").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.menu = Some("Context:Archetypes".into());
                                    this.client.send("archetypes.list", json!({}));
                                    cx.notify();
                                }),
                            ),
                        )
                    }),
            );
        }
        if dropdown {
            // Left-aligned under its label, with no backdrop: the rest of the window stays
            // interactive and a press outside closes the menu (label presses toggle it).
            // Anchors are window coordinates; this view is inset by the client-side border.
            let anchor = self
                .menu_anchors
                .borrow()
                .get(group)
                .copied()
                .unwrap_or_default();
            let bottom = f32::from(anchor.bottom() - padding.top);
            let height = f32::from(content_size.height) - bottom - 8.;
            let left = f32::from(anchor.left() - padding.left)
                .min(f32::from(content_size.width) - 328.)
                .max(4.);
            let anchors = self.menu_anchors.clone();
            return Some(
                panel
                    .absolute()
                    .left(px(left))
                    .top(px(bottom + 2.))
                    .max_h(px(height.clamp(120., 650.)))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down_out(cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                        if !anchors
                            .borrow()
                            .values()
                            .any(|b| b.contains(&event.position))
                        {
                            this.menu = None;
                            cx.notify();
                        }
                    }))
                    .into_any_element(),
            );
        }
        if context_menu {
            let origin = self
                .context_origin
                .map(|p| point(p.x - padding.left, p.y - padding.top))
                .unwrap_or(point(px(40.), px(100.)));
            let bounds = content_size;
            let height = (f32::from(bounds.height) - 32.).min(650.);
            let left = f32::from(origin.x)
                .min(f32::from(bounds.width) - 532.)
                .max(8.);
            let top = f32::from(origin.y)
                .min(f32::from(bounds.height) - height - 8.)
                .max(8.);
            return Some(
                div()
                    .absolute()
                    .inset_0()
                    .occlude()
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.menu = None;
                            cx.notify();
                        }),
                    )
                    .child(
                        panel
                            .h(px(height))
                            .absolute()
                            .left(px(left))
                            .top(px(top))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
                    )
                    .into_any_element(),
            );
        }
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .justify_center()
                .pt(px(64.))
                .bg(rgba(0x00000066))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.menu = None;
                        cx.notify();
                    }),
                )
                .child(panel.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()))
                .into_any_element(),
        )
    }
    pub(super) fn render_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut tabs = div().flex().gap_1().p_2();
        for (key, title) in [
            ("source", "Declaration"),
            ("specification", "Specification"),
            ("aiInstructions", "Usage Instructions"),
            ("tests", "Tests"),
            ("initialization", "Instantiation"),
        ] {
            tabs = tabs.child(
                button(key, title)
                    .when(self.section == key, |b| {
                        b.bg(crate::chrome::color(0x255452))
                    })
                    .on_click(cx.listener(move |this, _, w, cx| {
                        if this.has_draft(cx) || this.editor_request_pending {
                            this.status = "Apply the editor draft before changing section".into();
                        } else {
                            this.section = key.into();
                            this.load_editor(w, cx);
                            this.persist_ui(cx);
                        }
                        cx.notify();
                    })),
            );
        }
        let n = self
            .selected
            .first()
            .and_then(|id| self.snapshot.document.nodes.get(id));
        let language = n
            .map(|n| {
                self.snapshot
                    .document
                    .effective_text_language(&n.id, &self.section)
            })
            .unwrap_or_default();
        let editor = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .p_3()
                    .flex()
                    .gap_2()
                    .child(
                        div().flex_1().child(
                            n.map(|n| self.snapshot.document.qualified_name(&n.id))
                                .unwrap_or_default(),
                        ),
                    )
                    .child(
                        button("apply", "Apply · Ctrl Enter")
                            .on_click(cx.listener(|this, _, w, cx| this.apply(&Apply, w, cx))),
                    ),
            )
            .child(tabs)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .pb_2()
                    .text_xs()
                    .text_color(crate::chrome::color(0x8fa6b5))
                    .child("Language")
                    .when_some(
                        self.inspector
                            .iter()
                            .find(|f| f.path == format!("/text/{}LanguageId", self.section)),
                        |b, field| {
                            b.child(
                                div()
                                    .w(px(135.))
                                    .child(crate::inspector_widgets::language(&field.input, cx)),
                            )
                        },
                    )
                    .child(format!("Effective: {language}"))
                    .child(button("reload-entity", "Discard draft / Reload").on_click(
                        cx.listener(|this, _, w, cx| {
                            this.load_editor(w, cx);
                            this.persist_ui(cx);
                            cx.notify();
                        }),
                    )),
            )
            .when(!self.function_header.is_empty(), |b| {
                b.child(
                    div()
                        .id("generated-function-header")
                        .whitespace_nowrap()
                        .overflow_x_scroll()
                        .h(px(25.))
                        .px_3()
                        .font_family(self.options.saved.appearance.editor_font.clone())
                        .text_xs()
                        .text_color(crate::chrome::color(0x70d5c6))
                        .child(self.function_header.clone()),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, w, cx| {
                            this.editor.update(cx, |s, cx| s.focus(w, cx))
                        }),
                    )
                    .child({
                        let bounds = self.editor_bounds.clone();
                        canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {})
                            .absolute()
                            .size_full()
                    })
                    .font_family(self.options.saved.appearance.editor_font.clone())
                    .child(Input::new(&self.editor).h_full()),
            );
        let mut inspector = div()
            .id("inspector-scroll")
            .flex_1()
            .overflow_y_scroll()
            .p_2()
            .gap_1()
            .flex()
            .flex_col();
        inspector = inspector.child(heading("GENERAL")).child(field_row(
            "Name",
            Input::new(&self.name).small().into_any_element(),
        ));
        if let Some(n) = n {
            inspector = inspector.child(read_row(
                "Parent",
                n.parent_id
                    .as_ref()
                    .map(|id| self.snapshot.document.qualified_name(id))
                    .unwrap_or_else(|| "(none)".into()),
            ));
        }
        for f in &self.inspector {
            if f.path.starts_with("/masterRevision/")
                || f.path.starts_with("/text/")
                || ["/status", "/responsible", "/assignee"].contains(&f.path)
            {
                continue;
            }
            if f.path.starts_with("/link/") && !n.is_some_and(|n| n.is_link()) {
                continue;
            }
            if n.is_some_and(|n| n.is_link())
                && (f.path.starts_with("/technology/") || f.path == "/fileLayoutStrategyId")
            {
                continue;
            }
            if f.path == "/link/sourcePortName" {
                inspector = inspector.child(heading("LINK"));
            }
            inspector = inspector.child(field_row(f.label, self.inspector_control(f, cx)));
        }
        if let Some(n) = n {
            inspector = inspector.child(heading("EFFECTIVE"));
            for (label, key) in [
                ("Language", "languageId"),
                ("Technology", "technologyId"),
                ("FS layout", "fileLayoutStrategyId"),
                ("Responsible", "responsible"),
            ] {
                inspector = inspector.child(read_row(
                    label,
                    self.snapshot.document.effective(&n.id, key),
                ));
            }
            inspector = inspector.child(heading("MANAGEMENT"));
            for f in self
                .inspector
                .iter()
                .filter(|f| ["/status", "/responsible", "/assignee"].contains(&f.path))
            {
                inspector = inspector.child(field_row(f.label, self.inspector_control(f, cx)));
            }
            let revision = self.snapshot.document.effective_revision(&n.id);
            inspector = inspector
                .child(heading("TRACKING"))
                .child(read_row(
                    "Revision",
                    revision
                        .and_then(|v| v["name"].as_str())
                        .unwrap_or("")
                        .into(),
                ))
                .child(read_row(
                    "Revision date",
                    revision
                        .and_then(|v| v["date"].as_str())
                        .unwrap_or("")
                        .into(),
                ))
                .child(read_row(
                    "Modified by",
                    n.modified["user"].as_str().unwrap_or("").into(),
                ))
                .child(read_row(
                    "Modified date",
                    n.modified["date"].as_str().unwrap_or("").into(),
                ));
            if n.id == self.snapshot.document.root_node_id {
                for field in self
                    .inspector
                    .iter()
                    .filter(|f| f.path.starts_with("/masterRevision/"))
                {
                    inspector = inspector.child(field_row(
                        field.label,
                        Input::new(&field.input).into_any_element(),
                    ));
                }
            }
            if !n.status_changes.is_empty() {
                inspector = inspector.child(crate::inspector::history(&n.status_changes));
            }
        }
        if let Some(n) = n
            && !n.diagnostics.is_empty()
        {
            inspector = inspector.child(heading("STORED DIAGNOSTICS"));
            for (i, d) in n.diagnostics.iter().enumerate() {
                let diagnostic = json!({"source":{"nodeId":n.id,"section":d["textSection"],"line":d["line"],"column":d["column"]},"message":d["message"]});
                let mapped = d["textSection"].is_string() && d["line"].is_number();
                inspector = inspector.child(
                    div()
                        .id(("stored-diagnostic", i))
                        .p_2()
                        .rounded_sm()
                        .border_1()
                        .border_color(crate::chrome::color(0x54604d))
                        .text_xs()
                        .child(format!(
                            "{} · {}",
                            d["severity"].as_str().unwrap_or(""),
                            d["message"].as_str().unwrap_or("")
                        ))
                        .when(mapped, |b| {
                            b.cursor_pointer()
                                .on_click(cx.listener(move |this, _, w, cx| {
                                    this.reveal_diagnostic(&diagnostic, w, cx)
                                }))
                        }),
                );
            }
        }
        inspector = inspector.child(self.render_tables(cx));
        inspector = inspector.child(
            button(
                "advanced-properties",
                if self.advanced {
                    "Hide advanced properties"
                } else {
                    "Advanced properties · JSON"
                },
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.advanced = !this.advanced;
                cx.notify();
            })),
        );
        if self.advanced {
            inspector = inspector.child(
                div()
                    .h(px(420.))
                    .child(Input::new(&self.properties).h_full()),
            );
        }
        div()
            .flex()
            .flex_1()
            .min_h_0()
            .child(editor)
            .child(self.panel_separator(true, cx))
            .child(
                div()
                    .w(px(self.inspector_width))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .border_l_1()
                    .border_color(crate::chrome::color(0x29323d))
                    .bg(crate::chrome::color(0x191f28))
                    .child(
                        div()
                            .p_3()
                            .text_color(crate::chrome::color(0x70d5c6))
                            .child("Inspector"),
                    )
                    .child(inspector),
            )
            .into_any_element()
    }
    pub(super) fn render_management(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut body = div().flex().flex_1().min_h_0().gap_3().p_3();
        let doc = &self.snapshot.document;
        let mut nodes: Vec<_> = doc
            .nodes
            .values()
            .filter(|n| !n.is_link() && n.id != doc.root_node_id)
            .collect();
        nodes.sort_by_key(|n| doc.qualified_name(&n.id).to_lowercase());
        let lanes: Vec<Option<&str>> = if self.management_backlog {
            vec![None]
        } else {
            STATUSES.iter().map(|s| Some(*s)).collect()
        };
        for (i, status) in lanes.into_iter().enumerate() {
            let tickets: Vec<_> = nodes
                .iter()
                .filter(|n| n.status.as_deref() == status)
                .collect();
            let client = self.client.clone();
            let target = status.map(str::to_string);
            let draft = self.has_draft(cx);
            let mut lane = div()
                .id(("lane", i))
                .flex_1()
                .min_w(px(225.))
                .flex()
                .flex_col()
                .rounded_md()
                .border_1()
                .border_color(crate::chrome::color(0x303c49))
                .bg(crate::chrome::color(0x171f28))
                .child(
                    div()
                        .p_3()
                        .text_color(crate::chrome::color(0x70d5c6))
                        .child(format!(
                            "{}  {}",
                            status.unwrap_or("BACKLOG"),
                            tickets.len()
                        )),
                )
                .on_drop(move |t: &Ticket, _, _| {
                    if !draft {
                        client.send(
                            "threadwork.update_entity",
                            json!({"nodeId":t.id,"patch":{"status":target}}),
                        );
                    }
                });
            let mut list = div()
                .id(("tickets", i))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .p_2()
                .flex()
                .flex_col()
                .gap_2();
            for n in tickets {
                let id = n.id.clone();
                let open = id.clone();
                let remove = id.clone();
                let next = if status.is_none() {
                    Some("BUSINESS")
                } else {
                    None
                };
                list = list.child(
                    div()
                        .id(SharedString::from(id.clone()))
                        .p_3()
                        .rounded_md()
                        .bg(crate::chrome::color(
                            if n.kind == wirecraft_core::NodeKind::Type {
                                0x173b3a
                            } else {
                                0x252f3c
                            },
                        ))
                        .border_1()
                        .border_color(crate::chrome::color(if self.selected.contains(&id) {
                            0x70d5c6
                        } else {
                            0x43505f
                        }))
                        .cursor_pointer()
                        .on_click(
                            cx.listener(move |this, _, w, cx| this.select(open.clone(), w, cx)),
                        )
                        .on_drag(
                            Ticket {
                                id: id.clone(),
                                name: n.name.clone(),
                            },
                            |ticket, _, _, cx| cx.new(|_| ticket.clone()),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .child(
                                    div()
                                        .flex_1()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(n.name.clone()),
                                )
                                .child(
                                    button(
                                        "board-toggle",
                                        if status.is_none() { "+" } else { "×" },
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            cx.stop_propagation();
                                            if !this.has_draft(cx) {
                                                this.client.send(
                                                "threadwork.update_entity",
                                                json!({"nodeId":remove,"patch":{"status":next}}),
                                            );
                                            }
                                        },
                                    )),
                                ),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(crate::chrome::color(0x96a9bb))
                                .child(
                                    n.parent_id
                                        .as_ref()
                                        .map(|id| doc.qualified_name(id))
                                        .unwrap_or_default(),
                                ),
                        )
                        .child(div().text_xs().mt_2().child(format!(
                            "Responsible: {}",
                            n.responsible.as_deref().unwrap_or("(none)")
                        )))
                        .child(div().text_xs().child(format!(
                            "Assignee: {}",
                            n.assignee.as_deref().unwrap_or("(none)")
                        ))),
                );
            }
            lane = lane.child(list);
            let target = status.map(str::to_string);
            lane = lane.child(
                button("move-ticket", "Move selected here")
                    .text_xs()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.has_draft(cx) {
                            let changes = this
                                .selected
                                .iter()
                                .filter(|id| *id != &this.snapshot.document.root_node_id)
                                .map(|id| json!({"nodeId":id,"patch":{"status":target}}))
                                .collect::<Vec<_>>();
                            this.client
                                .send("entities.patch", json!({"changes":changes}));
                        }
                    })),
            );
            body = body.child(lane);
        }
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_3()
                    .child(
                        button("kanban", "Kanban board")
                            .when(!self.management_backlog, |b| {
                                b.bg(crate::chrome::color(0x255452))
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.management_backlog = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        button("backlog", "Backlog")
                            .when(self.management_backlog, |b| {
                                b.bg(crate::chrome::color(0x255452))
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.management_backlog = true;
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        button("open-ticket", "Edit selected ticket").on_click(cx.listener(
                            |this, _, _, cx| {
                                this.tab = "Editor".into();
                                cx.notify();
                            },
                        )),
                    ),
            )
            .child(
                div()
                    .id("board-scroll")
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_x_scroll()
                    .child(body),
            )
            .into_any_element()
    }
    pub(super) fn render_archetypes(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut groups: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
        for a in &self.archetypes {
            groups
                .entry(a["group"].as_str().unwrap_or("Other").into())
                .or_default()
                .push(a);
        }
        let mut catalog = div().id("archetype-list").flex_1().overflow_y_scroll();
        for (technology, items) in groups {
            catalog = catalog.child(heading(&technology));
            for a in items {
                let id = a["id"].as_str().unwrap_or("").to_string();
                let selected = self.archetype_selected.as_ref() == Some(&id);
                catalog = catalog.child(
                    div()
                        .id(SharedString::from(id.clone()))
                        .p_3()
                        .border_b_1()
                        .border_color(crate::chrome::color(0x2d3947))
                        .cursor_pointer()
                        .when(selected, |b| b.bg(crate::chrome::color(0x255452)))
                        .hover(|s| s.bg(crate::chrome::color(0x293846)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.archetype_selected = Some(id.clone());
                            this.archetype_preview = None;
                            this.client.send("archetypes.preview", json!({"id":id}));
                            cx.notify();
                        }))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(a["name"].as_str().unwrap_or("").to_string()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(crate::chrome::color(0x96a9bb))
                                .child(
                                    a["description"]
                                        .as_str()
                                        .unwrap_or("")
                                        .chars()
                                        .take(180)
                                        .collect::<String>(),
                                ),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(crate::chrome::color(0x9bb0c3))
                                .child(format!(
                                    "{} entities · {}",
                                    a["nodes"],
                                    a["id"].as_str().unwrap_or("")
                                )),
                        ),
                );
            }
        }
        let mut preview = div().flex_1().min_w_0().flex().flex_col().child(
            div()
                .p_3()
                .child("Archetype preview · select a reusable workflow"),
        );
        if let Some(scene) = &self.archetype_preview {
            let scene = scene.clone();
            preview = preview.child(
                canvas(
                    |bounds, _, _| bounds,
                    move |bounds, _, w, cx| {
                        let view = scene.fitted([
                            f64::from(f32::from(bounds.size.width)),
                            f64::from(f32::from(bounds.size.height)),
                        ]);
                        scene.paint(
                            bounds,
                            view,
                            &BTreeSet::new(),
                            &BTreeSet::new(),
                            [0., 0.],
                            &BTreeSet::new(),
                            w,
                            cx,
                        );
                    },
                )
                .flex_1(),
            );
        }
        div()
            .flex()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .w(px(300.))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .bg(crate::chrome::color(0x191f28))
                    .child(
                        div()
                            .p_3()
                            .flex()
                            .gap_2()
                            .child(button("refresh-archetypes", "Refresh").on_click(
                                cx.listener(|this, _, _, _| this.tool("archetypes.list")),
                            ))
                            .child(
                                button("import-archetype", "Import .orch").on_click(
                                    cx.listener(|this, _, w, cx| this.import_dialog(w, cx)),
                                ),
                            ),
                    )
                    .child(catalog)
                    .child(
                        button("insert-archetype", "Insert Archetype into Design").on_click(
                            cx.listener(|this, _, _, cx| {
                                if this.has_draft(cx) {
                                    this.status =
                                        "Apply the editor draft before inserting an archetype"
                                            .into();
                                } else if let Some(id) = &this.archetype_selected {
                                    this.client.send(
                                        "archetypes.import",
                                        json!({"id":id,"parentId":this.parent}),
                                    );
                                }
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .child(preview)
            .into_any_element()
    }
    pub(super) fn render_analysis(&self, cx: &mut Context<Self>) -> AnyElement {
        let report = &self.report;
        let mut metrics = div().flex().gap_3().p_3();
        for key in ["principal", "error", "dependency", "structural"] {
            let g = &report["graphs"][key];
            let cycles = g["cycles"].as_array().map(Vec::len).unwrap_or(0);
            metrics = metrics.child(
                div()
                    .flex_1()
                    .p_3()
                    .rounded_md()
                    .bg(crate::chrome::color(0x1b2631))
                    .child(heading(key))
                    .child(format!(
                        "{} relationships · {cycles} cycles",
                        g["edges"].as_u64().unwrap_or(0)
                    )),
            );
        }
        let graph = self.report["graphs"][&self.analysis_layer].clone();
        let ids: Vec<String> = report["nodeOrder"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|id| id.as_str().map(str::to_string))
            .filter(|id| self.snapshot.document.nodes.contains_key(id))
            .collect();
        let matrix_ids = Arc::new(ids);
        let ids_for_rows = matrix_ids.clone();
        let column_start = self.analysis_column.min(matrix_ids.len().saturating_sub(1));
        let column_end = (column_start + 32).min(matrix_ids.len());
        let cells = uniform_list(
            "matrix-rows",
            matrix_ids.len(),
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                range
                    .map(|i| {
                        let source = ids_for_rows[i].clone();
                        let mut row = div().id(i).flex().h(px(28.)).child(
                            div()
                                .w(px(220.))
                                .flex_none()
                                .px_2()
                                .overflow_hidden()
                                .text_xs()
                                .child(format!(
                                    "{}. {}",
                                    i + 1,
                                    this.snapshot.document.qualified_name(&source)
                                )),
                        );
                        let adjacent = graph["adjacency"][&source].as_array();
                        for (j, target) in ids_for_rows
                            .iter()
                            .enumerate()
                            .take(column_end)
                            .skip(column_start)
                        {
                            let has = adjacent.is_some_and(|a| {
                                a.iter().any(|id| id.as_str() == Some(target.as_str()))
                            });
                            let a = source.clone();
                            let b = target.clone();
                            row = row.child(
                                div()
                                    .id(("cell", j))
                                    .w(px(45.))
                                    .flex_none()
                                    .text_center()
                                    .border_1()
                                    .border_color(crate::chrome::color(0x303f4b))
                                    .bg(crate::chrome::color(if has { 0x257e81 } else { 0x19242c }))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, w, cx| {
                                        if has && !this.has_draft(cx) {
                                            this.selected = BTreeSet::from([a.clone(), b.clone()]);
                                            this.parent = this.snapshot.document.nodes[&a]
                                                .parent_id
                                                .clone()
                                                .unwrap_or_else(|| {
                                                    this.snapshot.document.root_node_id.clone()
                                                });
                                            this.rebuild();
                                            this.tab = "Designer".into();
                                            this.fit(&Fit, w, cx);
                                            this.load_editor(w, cx);
                                            cx.notify();
                                        }
                                    }))
                                    .child(if has { "1" } else { "0" }),
                            );
                        }
                        row
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .h(px(260.))
        .w(px(220. + (column_end - column_start) as f32 * 45.));
        let mut header = div()
            .flex()
            .h(px(36.))
            .child(div().w(px(220.)).flex_none().child("Node / target →"));
        for (i, id) in matrix_ids
            .iter()
            .enumerate()
            .take(column_end)
            .skip(column_start)
        {
            header = header.child(
                div()
                    .w(px(45.))
                    .flex_none()
                    .text_xs()
                    .overflow_hidden()
                    .child(format!(
                        "{} {}",
                        i + 1,
                        self.snapshot.document.nodes[id].name
                    )),
            );
        }
        let mut findings = div().flex().flex_wrap().gap_3().p_3();
        for category in [
            "overview",
            "principal",
            "error",
            "dependency",
            "workflows",
            "cross-layer",
            "quality",
        ] {
            let mut column = div()
                .flex_1()
                .min_w(px(300.))
                .max_w(px(600.))
                .flex()
                .flex_col()
                .gap_2()
                .child(heading(&format!("{category} findings")));
            for (i, metric) in report["indicators"][category]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
            {
                let ids = metric["affectedNodes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|id| id.as_str().map(str::to_string))
                    .collect::<BTreeSet<_>>();
                column = column.child(
                    div()
                        .id(("metric", i))
                        .flex()
                        .items_center()
                        .gap_2()
                        .p_1()
                        .cursor_pointer()
                        .hover(|s| s.bg(crate::chrome::color(0x293846)))
                        .child(
                            div()
                                .flex_1()
                                .text_xs()
                                .child(metric["label"].as_str().unwrap_or("").to_string()),
                        )
                        .child(metric["value"].to_string())
                        .on_click(cx.listener(move |this, _, w, cx| {
                            if !this.has_draft(cx) && !ids.is_empty() {
                                this.selected = ids.clone();
                                this.parent = this.snapshot.document.root_node_id.clone();
                                this.rebuild();
                                this.tab = "Designer".into();
                                this.fit(&Fit, w, cx);
                                this.load_editor(w, cx);
                                this.persist_ui(cx);
                                cx.notify();
                            }
                        })),
                );
            }
            for (i, finding) in report["findings"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|f| f["category"] == category)
                .enumerate()
            {
                let ids: BTreeSet<String> = finding["affectedNodes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect();
                column =
                    column.child(
                        div()
                            .id(("finding", i))
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(crate::chrome::color(0x54604d))
                            .bg(crate::chrome::color(0x252b2a))
                            .child(format!(
                                "{} · {}",
                                finding["severity"].as_str().unwrap_or(""),
                                finding["title"].as_str().unwrap_or("")
                            ))
                            .child(div().py_2().text_xs().child(
                                finding["recommendation"].as_str().unwrap_or("").to_string(),
                            ))
                            .child(button("affected", "Select affected entities").on_click(
                                cx.listener(move |this, _, w, cx| {
                                    if !this.has_draft(cx) {
                                        this.selected = ids.clone();
                                        this.parent = this.snapshot.document.root_node_id.clone();
                                        this.rebuild();
                                        this.tab = "Designer".into();
                                        this.fit(&Fit, w, cx);
                                        this.load_editor(w, cx);
                                        cx.notify();
                                    }
                                }),
                            )),
                    );
            }
            if report["findings"]
                .as_array()
                .is_some_and(|a| !a.iter().any(|f| f["category"] == category))
            {
                column = column.child(
                    div()
                        .text_color(crate::chrome::color(0x92a8b8))
                        .child("No findings at the current thresholds."),
                );
            }
            findings = findings.child(column);
        }
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .p_3()
                    .child(div().flex_1().text_lg().child("Network Analysis"))
                    .child(
                        button("report-full", "Refresh")
                            .on_click(cx.listener(|this, _, _, _| this.tool("analysis.report"))),
                    )
                    .child(button("validate", "Validate design").on_click(
                        cx.listener(|this, _, _, _| this.tool("threadwork.validate_design")),
                    )),
            )
            .child(metrics)
            .child(
                div()
                    .flex()
                    .gap_2()
                    .px_3()
                    .items_center()
                    .child(
                        button("matrix-prev", "Previous columns").on_click(cx.listener(
                            |this, _, _, cx| {
                                this.analysis_column = this.analysis_column.saturating_sub(32);
                                cx.notify();
                            },
                        )),
                    )
                    .child(format!(
                        "Columns {}–{} of {} · rows are virtualized",
                        column_start + 1,
                        column_end,
                        matrix_ids.len()
                    ))
                    .child(button("matrix-next", "Next columns").on_click(cx.listener(
                        |this, _, _, cx| {
                            if this.analysis_column + 32
                                < this
                                    .snapshot
                                    .document
                                    .nodes
                                    .values()
                                    .filter(|n| !n.is_link())
                                    .count()
                            {
                                this.analysis_column += 32;
                            }
                            cx.notify();
                        },
                    ))),
            )
            .child(
                div().flex().gap_2().px_3().children(
                    [
                        ("principal", "Principal data"),
                        ("error", "Error data"),
                        ("dependency", "Dependencies"),
                        ("structural", "Structural union"),
                    ]
                    .into_iter()
                    .map(|(key, label)| {
                        button(key, label)
                            .when(self.analysis_layer == key, |b| {
                                b.bg(crate::chrome::color(0x255452))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.analysis_layer = key.into();
                                cx.notify();
                            }))
                    }),
                ),
            )
            .child(
                div()
                    .id("matrix-scroll")
                    .mx_3()
                    .overflow_x_scroll()
                    .child(div().flex().flex_col().child(header).child(cells)),
            )
            .child(
                div()
                    .id("findings-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(findings)
                    .child(
                        div().p_3().flex().flex_wrap().gap_2().children(
                            report["actions"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .enumerate()
                                .map(|(i, action)| {
                                    let ids = action["affectedNodes"]
                                        .as_array()
                                        .into_iter()
                                        .flatten()
                                        .filter_map(|id| id.as_str().map(str::to_string))
                                        .collect::<BTreeSet<_>>();
                                    div()
                                        .id(("inspection-action", i))
                                        .p_2()
                                        .rounded_sm()
                                        .bg(crate::chrome::color(0x242c36))
                                        .cursor_pointer()
                                        .child(action["title"].as_str().unwrap_or("").to_string())
                                        .on_click(cx.listener(move |this, _, w, cx| {
                                            if !this.has_draft(cx) && !ids.is_empty() {
                                                this.selected = ids.clone();
                                                this.parent =
                                                    this.snapshot.document.root_node_id.clone();
                                                this.rebuild();
                                                this.tab = "Designer".into();
                                                this.fit(&Fit, w, cx);
                                                this.load_editor(w, cx);
                                                this.persist_ui(cx);
                                                cx.notify();
                                            }
                                        }))
                                }),
                        ),
                    )
                    .child(div().p_3().child(format!(
                            "Isolated entities: {} · Connected components: {}",
                            report["isolatedNodes"]
                                .as_array()
                                .map(Vec::len)
                                .unwrap_or(0),
                            report["connectedComponents"]
                                .as_array()
                                .map(Vec::len)
                                .unwrap_or(0)
                        ))),
            )
            .into_any_element()
    }
}
fn heading(text: &str) -> AnyElement {
    div()
        .pt_2()
        .pb_1()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(crate::chrome::color(0x70d5c6))
        .child(text.to_uppercase())
        .into_any_element()
}
fn field_row(label: &str, input: AnyElement) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .min_h(px(27.))
        .child(
            div()
                .w(px(112.))
                .flex_none()
                .text_xs()
                .text_color(crate::chrome::color(0xa0b0c0))
                .child(label.to_string()),
        )
        .child(div().flex_1().min_w_0().child(input))
        .into_any_element()
}
fn read_row(label: &str, value: String) -> AnyElement {
    let tooltip = value.clone();
    div()
        .id(SharedString::from(format!("read-{label}")))
        .tooltip(move |w, cx| gpui_component::tooltip::Tooltip::new(tooltip.clone()).build(w, cx))
        .flex()
        .items_center()
        .gap_2()
        .min_h(px(22.))
        .child(
            div()
                .w(px(112.))
                .flex_none()
                .text_xs()
                .text_color(crate::chrome::color(0x8496a5))
                .child(label.to_string()),
        )
        .child(div().flex_1().min_w_0().text_xs().truncate().child(value))
        .into_any_element()
}
