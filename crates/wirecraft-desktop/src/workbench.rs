//! Screenshot-driven workbench views. Mutations use the shared application service.
use super::*;
use std::collections::BTreeMap;

pub struct FormField {
    pub path: &'static str,
    label: &'static str,
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
struct Ticket {
    id: String,
    name: String,
}
impl Render for Ticket {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_3()
            .rounded_md()
            .bg(rgb(0x255452))
            .text_color(rgb(0xe1e9ef))
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
        label: "Generate C17 Project",
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
        if changed {
            serde_json::to_string_pretty(&value).unwrap_or(raw)
        } else {
            raw
        }
    }
    pub(super) fn palette(&mut self, _: &CommandPalette, w: &mut Window, cx: &mut Context<Self>) {
        self.menu = Some("Commands".into());
        self.query.update(cx, |s, cx| {
            s.set_value("", w, cx);
            s.focus(w, cx);
        });
        cx.notify();
    }
    pub(super) fn show_export(&mut self, _: &ShowExport, _: &mut Window, cx: &mut Context<Self>) {
        self.tab = "Export".into();
        self.menu = None;
        cx.notify();
    }
    pub(super) fn show_options(&mut self, _: &ShowOptions, _: &mut Window, cx: &mut Context<Self>) {
        self.tab = "Options".into();
        self.menu = None;
        cx.notify();
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
            "select" => self.select_mode(&SelectMode, w, cx),
            "node" => self.node_mode(&NodeMode, w, cx),
            "link" => self.link_mode(&LinkMode, w, cx),
            "type" => self.mode = Mode::Type,
            "delete" => self.delete(&DeleteSelection, w, cx),
            "export" => self.show_export(&ShowExport, w, cx),
            "options" => self.show_options(&ShowOptions, w, cx),
            "palette" => self.palette(&CommandPalette, w, cx),
            "generate" => self.artifact_dialog("c17", w, cx),
            "files" => self.artifact_dialog("files", w, cx),
            "archetype" => self.artifact_dialog("archetype", w, cx),
            "mcp" => self.tab = "MCP".into(),
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
            .bg(rgb(0x10161e))
            .border_b_1()
            .border_color(rgb(0x29323d));
        for label in [
            "File",
            "Edit",
            "Graph",
            "Generate",
            "MCP",
            "AI Support",
            "Help",
        ] {
            bar = bar.child(
                div()
                    .id(label)
                    .px_3()
                    .py_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0x29323d)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.menu = if this.menu.as_deref() == Some(label) {
                            None
                        } else {
                            Some(label.into())
                        };
                        cx.notify();
                    }))
                    .child(label),
            );
        }
        bar.child(div().flex_1())
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
            .into_any_element()
    }
    pub(super) fn render_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let group = self.menu.as_deref()?;
        let mut panel = div()
            .id("command-panel")
            .w(px(520.))
            .max_h(px(650.))
            .flex()
            .flex_col()
            .p_2()
            .gap_1()
            .rounded_lg()
            .border_1()
            .border_color(rgb(0x40505c))
            .bg(rgb(0x1c242e))
            .shadow_lg();
        panel = panel.child(
            div()
                .flex()
                .items_center()
                .p_2()
                .child(div().flex_1().text_lg().child(group.to_string()))
                .child(
                    button("close-menu", "Close").on_click(cx.listener(|this, _, w, cx| {
                        this.menu = None;
                        this.focus.focus(w);
                        cx.notify();
                    })),
                ),
        );
        if group == "About" || group == "Guide" {
            panel=panel.child(div().p_4().child(if group=="About" {format!("Wirecraft {}\nRust + GPUI\nMigrated from Threadworks 2.6.7\nSource: 7d392d3a0e30b86445e00e7c03032a5188b26593",env!("CARGO_PKG_VERSION"))}else{"Connect an MCP client using MCP → Copy configuration. The desktop and MCP share one design and undo history. Use threadwork.get_design, get_entity and validate_design to inspect it. Use expectedRevision with HTTP API mutations to prevent stale edits. Files keep the .orch format. Closing this desktop stops its owned backend and language servers. See README.md and docs/migration for the full interface, compiler capabilities and remaining gaps.".into()}));
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
                div().id("command-list").overflow_y_scroll().children(
                    COMMANDS
                        .iter()
                        .filter(|c| {
                            (group == "Commands" || c.group == group)
                                && format!("{} {}", c.group, c.label)
                                    .to_lowercase()
                                    .contains(&query)
                        })
                        .map(|c| {
                            let id = c.id;
                            div()
                                .id(id)
                                .flex()
                                .items_center()
                                .gap_3()
                                .p_2()
                                .rounded_sm()
                                .cursor_pointer()
                                .hover(|s| s.bg(rgb(0x2d3b48)))
                                .on_click(
                                    cx.listener(move |this, _, w, cx| this.command(id, w, cx)),
                                )
                                .child(icons::icon(c.icon))
                                .child(div().flex_1().child(if group == "Commands" {
                                    format!("{}: {}", c.group, c.label)
                                } else {
                                    c.label.into()
                                }))
                                .child(div().text_xs().text_color(rgb(0x8a9aab)).child(c.shortcut))
                        }),
                ),
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
                    .when(self.section == key, |b| b.bg(rgb(0x255452)))
                    .on_click(cx.listener(move |this, _, w, cx| {
                        if this.has_draft(cx) {
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
                    .text_color(rgb(0x8fa6b5))
                    .child(format!("Effective language: {language}"))
                    .child(button("reload-entity", "Discard draft / Reload").on_click(
                        cx.listener(|this, _, w, cx| {
                            this.load_editor(w, cx);
                            this.persist_ui(cx);
                            cx.notify();
                        }),
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .font_family(self.options.saved.appearance.editor_font.clone())
                    .child(Input::new(&self.editor).h_full()),
            );
        let mut inspector = div()
            .id("inspector-scroll")
            .flex_1()
            .overflow_y_scroll()
            .p_3()
            .gap_2()
            .flex()
            .flex_col();
        inspector = inspector
            .child(heading("GENERAL"))
            .child(field_row("Name", Input::new(&self.name).into_any_element()));
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
            if f.path.starts_with("/link/") && !n.is_some_and(|n| n.is_link()) {
                continue;
            }
            if n.is_some_and(|n| n.is_link())
                && (f.path.starts_with("/technology/") || f.path == "/fileLayoutStrategyId")
            {
                continue;
            }
            if f.path == "/status" {
                inspector = inspector.child(heading("MANAGEMENT"));
            }
            if f.path == "/text/sourceLanguageId" {
                inspector = inspector.child(heading("SECTION LANGUAGES"));
            }
            if f.path == "/link/sourcePortName" {
                inspector = inspector.child(heading("LINK"));
            }
            if f.path == "/status" {
                let status = f.input.read(cx).value().to_string();
                inspector = inspector.child(field_row(
                    f.label,
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_1()
                        .children(std::iter::once("").chain(STATUSES).map(|s| {
                            let input = f.input.clone();
                            button(
                                if s.is_empty() { "no-status" } else { s },
                                if s.is_empty() { "None" } else { s },
                            )
                            .text_xs()
                            .when(status == s, |b| b.bg(rgb(0x255452)))
                            .on_click(move |_, w, cx| {
                                input.update(cx, |state, cx| state.set_value(s, w, cx))
                            })
                        }))
                        .into_any_element(),
                ));
            } else {
                inspector =
                    inspector.child(field_row(f.label, Input::new(&f.input).into_any_element()));
            }
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
            inspector = inspector
                .child(heading("TRACKING"))
                .child(read_row("Modified", n.modified.to_string()))
                .child(read_row(
                    "Revision",
                    n.revision
                        .as_ref()
                        .map(Value::to_string)
                        .unwrap_or_default(),
                ));
            for (i, event) in n.status_changes.iter().rev().enumerate() {
                inspector = inspector.child(
                    div()
                        .id(("transition", i))
                        .text_xs()
                        .p_2()
                        .bg(rgb(0x202b35))
                        .child(format!(
                            "{}  {} → {} · {}",
                            event["changedDate"].as_str().unwrap_or(""),
                            event["initStatus"].as_str().unwrap_or("none"),
                            event["endStatus"].as_str().unwrap_or("none"),
                            event["userId"].as_str().unwrap_or("")
                        )),
                );
            }
        }
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
            .child(
                div()
                    .w(px(360.))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .border_l_1()
                    .border_color(rgb(0x29323d))
                    .bg(rgb(0x191f28))
                    .child(div().p_3().text_color(rgb(0x70d5c6)).child("Inspector"))
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
                .border_color(rgb(0x303c49))
                .bg(rgb(0x171f28))
                .child(div().p_3().text_color(rgb(0x70d5c6)).child(format!(
                    "{}  {}",
                    status.unwrap_or("BACKLOG"),
                    tickets.len()
                )))
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
                        .bg(rgb(if n.kind == wirecraft_core::NodeKind::Type {
                            0x173b3a
                        } else {
                            0x252f3c
                        }))
                        .border_1()
                        .border_color(rgb(if self.selected.contains(&id) {
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
                            div().text_xs().text_color(rgb(0x96a9bb)).child(
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
                            .when(!self.management_backlog, |b| b.bg(rgb(0x255452)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.management_backlog = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        button("backlog", "Backlog")
                            .when(self.management_backlog, |b| b.bg(rgb(0x255452)))
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
                .entry(a["technologyId"].as_str().unwrap_or("Other").into())
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
                        .border_color(rgb(0x2d3947))
                        .cursor_pointer()
                        .when(selected, |b| b.bg(rgb(0x255452)))
                        .hover(|s| s.bg(rgb(0x293846)))
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
                        .child(div().text_xs().text_color(rgb(0x9bb0c3)).child(format!(
                            "{} entities · {}",
                            a["nodes"],
                            a["id"].as_str().unwrap_or("")
                        ))),
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
                        scene.paint(bounds, view, &BTreeSet::new(), [0., 0.], w, cx);
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
                    .bg(rgb(0x191f28))
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
                    .bg(rgb(0x1b2631))
                    .child(heading(key))
                    .child(format!(
                        "{} relationships · {cycles} cycles",
                        g["edges"].as_u64().unwrap_or(0)
                    )),
            );
        }
        let graph = self.report["graphs"][&self.analysis_layer].clone();
        let ids: Vec<_> = self
            .snapshot
            .document
            .nodes
            .values()
            .filter(|n| !n.is_link())
            .map(|n| n.id.clone())
            .collect();
        let matrix_ids = Arc::new(ids);
        let ids_for_rows = matrix_ids.clone();
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
                        for (j, target) in ids_for_rows.iter().enumerate() {
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
                                    .border_color(rgb(0x303f4b))
                                    .bg(rgb(if has { 0x257e81 } else { 0x19242c }))
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
        .w(px(220. + matrix_ids.len() as f32 * 45.));
        let mut header = div()
            .flex()
            .h(px(36.))
            .child(div().w(px(220.)).flex_none().child("Node / target →"));
        for (i, id) in matrix_ids.iter().enumerate() {
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
        let mut findings = div().flex().gap_3().p_3();
        for category in ["principal", "error", "dependency"] {
            let mut column = div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_2()
                .child(heading(&format!("{category} findings")));
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
                            .border_color(rgb(0x54604d))
                            .bg(rgb(0x252b2a))
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
                        .text_color(rgb(0x92a8b8))
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
                            .when(self.analysis_layer == key, |b| b.bg(rgb(0x255452)))
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
        .py_2()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0x70d5c6))
        .child(text.to_uppercase())
        .into_any_element()
}
fn field_row(label: &str, input: AnyElement) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .mb_2()
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xa0b0c0))
                .child(label.to_string()),
        )
        .child(input)
        .into_any_element()
}
fn read_row(label: &str, value: String) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .mb_2()
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x8496a5))
                .child(label.to_string()),
        )
        .child(div().text_xs().child(value))
        .into_any_element()
}
