//! Go to definition (F12, Ctrl-click) and Find references (Shift-F12) in compiler
//! bodies. The backend maps every server location to the model; this module reveals it.
use super::*;
pub(super) struct Ticket {
    node: String,
    section: String,
    text: String,
    generation: u64,
}
pub(super) struct References {
    identifier: String,
    locations: Vec<Value>,
}
impl Wirecraft {
    pub(super) fn go_to_definition(
        &mut self,
        _: &GoToDefinition,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request_navigation("editor.definition", cx);
    }
    pub(super) fn find_references(
        &mut self,
        _: &FindReferences,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request_navigation("editor.references", cx);
    }
    pub(super) fn request_navigation(&mut self, command: &str, cx: &mut Context<Self>) {
        if self.tab != "Editor" {
            return;
        }
        let Some(node) = self.editor_entity.clone() else {
            return;
        };
        if wirecraft_core::editor::compiler_language(&self.snapshot.document, &node, &self.section)
            .is_none()
        {
            self.status =
                "Definitions and references need a C17 or Rust Declaration or Instantiation body"
                    .into();
            cx.notify();
            return;
        }
        let state = self.editor.read(cx);
        let text = state.value().to_string();
        let offset = state.cursor();
        self.navigation_generation += 1;
        self.navigation = Some(Ticket {
            node: node.clone(),
            section: self.section.clone(),
            text: text.clone(),
            generation: self.navigation_generation,
        });
        let mut request = Request::new(
            command,
            json!({"nodeId":node,"section":self.section,"text":text,"offset":offset,"generation":self.navigation_generation}),
        );
        request.expected_revision = Some(self.snapshot.revision);
        let _ = self.client.sender.send(Message::Execute(request));
        self.status = if command == "editor.definition" {
            "Looking up the definition…".into()
        } else {
            "Finding references…".into()
        };
        cx.notify();
    }
    pub(super) fn accept_navigation(
        &mut self,
        command: &str,
        value: &Value,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ticket) = self.navigation.take() else {
            return;
        };
        // A reply for an older request, another body or edited text is stale.
        if value["generation"].as_u64() != Some(ticket.generation)
            || value["revision"].as_u64() != Some(self.snapshot.revision)
            || self.editor_entity.as_deref() != Some(ticket.node.as_str())
            || self.section != ticket.section
            || self.editor.read(cx).value().as_ref() != ticket.text
        {
            self.status = "The body changed; look it up again".into();
            return;
        }
        let identifier = value["identifier"].as_str().unwrap_or("").to_string();
        let locations = value["locations"].as_array().cloned().unwrap_or_default();
        if command == "editor.references" {
            self.status = format!(
                "{} reference{} to {identifier}",
                locations.len(),
                if locations.len() == 1 { "" } else { "s" }
            );
            self.references = Some(References {
                identifier,
                locations,
            });
            self.menu = Some("References".into());
        } else if let Some(location) = locations.first() {
            self.reveal_location(location, w, cx);
        } else {
            self.status = format!("No definition found for {identifier}");
        }
        cx.notify();
    }
    /// Moves the cursor within the edited body, or reveals another entity's body (which
    /// needs the draft applied first); generated and external code are only described.
    /// Returns whether the editor navigated.
    pub(super) fn reveal_location(
        &mut self,
        location: &Value,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let source = &location["source"];
        let navigated = match location["kind"].as_str().unwrap_or("") {
            "body"
                if source["nodeId"].as_str() == self.editor_entity.as_deref()
                    && source["section"] == self.section =>
            {
                let text = self.editor.read(cx).value().to_string();
                let row = (source["line"].as_u64().unwrap_or(1).saturating_sub(1) as usize)
                    .min(text.split('\n').count().saturating_sub(1));
                let length = text.split('\n').nth(row).map_or(0, |l| l.chars().count());
                let column =
                    (source["column"].as_u64().unwrap_or(1).saturating_sub(1) as usize).min(length);
                self.editor.update(cx, |editor, cx| {
                    editor.set_cursor_position(
                        lsp_types::Position::new(row as u32, column as u32),
                        w,
                        cx,
                    );
                    editor.focus(w, cx);
                });
                self.status = format!("{}:{}", row + 1, column + 1);
                true
            }
            "body" | "entity" => {
                let blocked = self.has_draft(cx) || self.editor_request_pending;
                self.reveal_diagnostic(location, w, cx);
                !blocked
            }
            "generated" => {
                self.status = format!(
                    "Generated code, line {}: {}",
                    location["line"],
                    location["text"].as_str().unwrap_or("")
                );
                false
            }
            _ => {
                self.status = format!(
                    "Outside the design: {}:{}",
                    location["path"].as_str().unwrap_or(""),
                    location["line"]
                );
                false
            }
        };
        cx.notify();
        navigated
    }
    pub(super) fn render_references(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut list = div()
            .id("editor-references")
            .h(px(530.))
            .overflow_y_scroll();
        let Some(references) = &self.references else {
            return list.into_any_element();
        };
        list = list.child(
            div()
                .p_2()
                .text_color(crate::chrome::color(0x9eb7c4))
                .child(format!("References to {}", references.identifier)),
        );
        if references.locations.is_empty() {
            list = list.child(div().p_3().child("No references found."));
        }
        for (i, location) in references.locations.iter().enumerate() {
            let source = &location["source"];
            let name = source["nodeId"]
                .as_str()
                .map(|id| self.snapshot.document.qualified_name(id));
            let title = match (location["kind"].as_str().unwrap_or(""), name) {
                ("body", Some(name)) => format!(
                    "{name} · {}:{}:{}",
                    source["section"].as_str().unwrap_or("source"),
                    source["line"],
                    source["column"]
                ),
                ("entity", Some(name)) => format!("{name} · entity"),
                ("generated", _) => format!(
                    "Generated {} · line {}",
                    location["path"].as_str().unwrap_or(""),
                    location["line"]
                ),
                _ => format!(
                    "{}:{}",
                    location["path"].as_str().unwrap_or(""),
                    location["line"]
                ),
            };
            let navigable = ["body", "entity"].contains(&location["kind"].as_str().unwrap_or(""));
            let location = location.clone();
            list = list.child(
                div()
                    .id(i)
                    .p_3()
                    .border_b_1()
                    .border_color(crate::chrome::color(0x40505c))
                    .when(navigable, |row| {
                        row.cursor_pointer()
                            .hover(|s| s.bg(crate::chrome::color(0x293846)))
                    })
                    .when(!navigable, |row| {
                        row.text_color(crate::chrome::color(0x8a98a3))
                    })
                    .child(div().child(title))
                    .child(
                        div()
                            .text_xs()
                            .truncate()
                            .font_family(self.options.saved.appearance.editor_font.clone())
                            .child(location["text"].as_str().unwrap_or("").to_string()),
                    )
                    .on_click(cx.listener(move |this, _, w, cx| {
                        if this.reveal_location(&location, w, cx) {
                            this.menu = None;
                        }
                        cx.notify();
                    })),
            );
        }
        list.into_any_element()
    }
}
