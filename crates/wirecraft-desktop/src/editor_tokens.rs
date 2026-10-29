//! Immediate language syntax plus versioned compiler token overlays for authored bodies.
use super::*;
use std::time::Duration;
pub(super) struct Ticket {
    node: String,
    section: String,
    text: String,
    revision: u64,
    generation: u64,
}
pub(super) fn register_languages() {
    use gpui_component::highlighter::{LanguageConfig, LanguageRegistry};
    let c = LanguageConfig::new(
        "c",
        tree_sitter_c::LANGUAGE.into(),
        vec![],
        tree_sitter_c::HIGHLIGHT_QUERY,
        "",
        "",
    );
    LanguageRegistry::singleton().register("c", &c);
    let cpp = LanguageConfig::new(
        "cpp",
        tree_sitter_cpp::LANGUAGE.into(),
        vec![],
        &format!(
            "{}\n{}",
            tree_sitter_c::HIGHLIGHT_QUERY,
            tree_sitter_cpp::HIGHLIGHT_QUERY
        ),
        "",
        "",
    );
    LanguageRegistry::singleton().register("cpp", &cpp);
    let rust = LanguageConfig::new(
        "rust",
        tree_sitter_rust::LANGUAGE.into(),
        vec![],
        tree_sitter_rust::HIGHLIGHTS_QUERY,
        "",
        "",
    );
    LanguageRegistry::singleton().register("rust", &rust);
}
impl Wirecraft {
    pub(super) fn queue_tokens(&mut self, cx: &mut Context<Self>) {
        self.token_dirty = true;
        self.token_timer = Some(cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(350)).await;
            let _ = this.update(cx, |this, cx| this.start_tokens(cx));
        }));
    }
    fn start_tokens(&mut self, cx: &mut Context<Self>) {
        if self.token_inflight || !self.token_dirty || !self.connected_document {
            return;
        }
        self.token_dirty = false;
        let Some(node) = self.editor_entity.clone() else {
            return;
        };
        if self.tab != "Editor"
            || wirecraft_core::editor::compiler_language(
                &self.snapshot.document,
                &node,
                &self.section,
            )
            .is_none()
        {
            return;
        }
        let text = self.editor.read(cx).value().to_string();
        if self.token_ticket.as_ref().is_some_and(|t| {
            t.node == node
                && t.section == self.section
                && t.text == text
                && t.revision == self.snapshot.revision
        }) {
            return;
        }
        self.token_generation += 1;
        self.token_ticket = Some(Ticket {
            node: node.clone(),
            section: self.section.clone(),
            text: text.clone(),
            revision: self.snapshot.revision,
            generation: self.token_generation,
        });
        let mut request = Request::new(
            "editor.tokens",
            json!({"nodeId":node,"section":self.section,"text":text,"generation":self.token_generation}),
        );
        request.expected_revision = Some(self.snapshot.revision);
        self.token_inflight = true;
        let _ = self.client.sender.send(Message::Execute(request));
    }
    pub(super) fn accept_tokens(&mut self, value: Value, cx: &mut Context<Self>) {
        self.token_inflight = false;
        if let Some(t) = &self.token_ticket
            && value["generation"].as_u64() == Some(t.generation)
            && value["revision"].as_u64() == Some(self.snapshot.revision)
            && self.editor_entity.as_deref() == Some(&t.node)
            && self.section == t.section
            && self.editor.read(cx).value().as_ref() == t.text
        {
            if value["diagnosticsCurrent"] == true {
                self.generated_revision = self.snapshot.revision;
                self.generated_diagnostics =
                    value["diagnostics"].as_array().cloned().unwrap_or_default();
                for diagnostic in &mut self.generated_diagnostics {
                    diagnostic["positionEncoding"] = value["serverPositionEncoding"].clone();
                }
            }
            let light = self.options.saved.appearance.theme == "light";
            let spans = value["tokens"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| {
                    let range = v["startByte"].as_u64()? as usize..v["endByte"].as_u64()? as usize;
                    let color = match v["kind"].as_str()? {
                        // The last four are rust-analyzer's.
                        "type" | "class" | "struct" | "enum" | "interface" | "typeParameter"
                        | "builtinType" | "typeAlias" | "selfTypeKeyword" | "union" => {
                            if light {
                                0x006d8f
                            } else {
                                0x72d4eb
                            }
                        }
                        "function" | "method" => {
                            if light {
                                0x9439a4
                            } else {
                                0xdf83e6
                            }
                        }
                        "property" => {
                            if light {
                                0x367957
                            } else {
                                0x9cdbb4
                            }
                        }
                        "parameter" => {
                            if light {
                                0x296980
                            } else {
                                0x96cddd
                            }
                        }
                        "macro" | "enumMember" => {
                            if light {
                                0x916418
                            } else {
                                0xe3c879
                            }
                        }
                        "namespace" => {
                            if light {
                                0x3f6ea0
                            } else {
                                0x9fc9ef
                            }
                        }
                        _ => return None,
                    };
                    Some((
                        range,
                        HighlightStyle {
                            color: Some(rgb(color).into()),
                            ..Default::default()
                        },
                    ))
                })
                .collect();
            let text = t.text.clone();
            self.editor.update(cx, |state, cx| {
                state.set_semantic_highlights(&text, spans, cx)
            });
        }
        if self.token_dirty {
            self.queue_tokens(cx);
        }
        cx.notify();
    }
    pub(super) fn tokens_failed(&mut self, message: String, cx: &mut Context<Self>) {
        self.token_inflight = false;
        self.status = format!("Compiler coloring unavailable: {message}");
        if self.token_dirty {
            self.queue_tokens(cx);
        }
        cx.notify();
    }
}
