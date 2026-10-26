//! Explicit Ctrl-Space completion. The backend owns compiler context and clangd.
use super::*;
pub(super) struct Pending {
    node: String,
    section: String,
    text: String,
    offset: usize,
    generation: u64,
    origin: Point<Pixels>,
    items: Option<Vec<Value>>,
    selected: usize,
    scroll: ScrollHandle,
}
impl Wirecraft {
    pub(super) fn request_completion(
        &mut self,
        _: &Complete,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tab != "Editor" {
            return;
        }
        let Some(node) = self.editor_entity.clone() else {
            return;
        };
        let state = self.editor.read(cx);
        let text = state.value().to_string();
        let offset = state.cursor();
        let utf16 = text[..offset].encode_utf16().count();
        let bounds = self.editor.update(cx, |state, cx| {
            state.bounds_for_range(utf16..utf16, self.editor_bounds.get(), w, cx)
        });
        let origin = bounds
            .map(|b| point(b.left(), b.bottom() + px(4.)))
            .unwrap_or(point(px(320.), px(310.)));
        self.completion_generation += 1;
        self.completion = Some(Pending {
            node: node.clone(),
            section: self.section.clone(),
            text: text.clone(),
            offset,
            generation: self.completion_generation,
            origin,
            items: None,
            selected: 0,
            scroll: ScrollHandle::new(),
        });
        let mut request = Request::new(
            "editor.complete",
            json!({"nodeId":node,"section":self.section,"text":text,"offset":offset,"generation":self.completion_generation}),
        );
        request.expected_revision = Some(self.snapshot.revision);
        let _ = self.client.sender.send(Message::Execute(request));
        self.status = "Loading compiler-context suggestions…".into();
        cx.notify();
    }
    pub(super) fn accept_completions(&mut self, value: &Value, cx: &mut Context<Self>) {
        let Some(pending) = self.completion.as_mut() else {
            return;
        };
        if value["generation"].as_u64() != Some(pending.generation)
            || value["revision"].as_u64() != Some(self.snapshot.revision)
            || self.editor_entity.as_deref() != Some(pending.node.as_str())
            || self.section != pending.section
            || self.tab != "Editor"
            || self.editor.read(cx).value().as_ref() != pending.text
            || self.editor.read(cx).cursor() != pending.offset
        {
            self.completion = None;
            return;
        }
        pending.items = Some(value["items"].as_array().cloned().unwrap_or_default());
        self.status = format!(
            "{} compiler-context suggestions",
            pending.items.as_ref().unwrap().len()
        );
        cx.notify();
    }
    pub(super) fn completion_action(
        &mut self,
        key: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.completion.is_none() {
            return false;
        }
        cx.stop_propagation();
        match key {
            "escape" => self.completion = None,
            "enter" | "tab" => self.insert_completion(w, cx),
            _ => {
                let p = self.completion.as_mut().unwrap();
                let count = p.items.as_ref().map_or(0, Vec::len);
                p.selected = if key == "up" {
                    p.selected.saturating_sub(1)
                } else {
                    (p.selected + 1).min(count.saturating_sub(1))
                };
                p.scroll.scroll_to_item(p.selected);
            }
        }
        cx.notify();
        true
    }
    fn insert_completion(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(p) = self.completion.take() else {
            return;
        };
        let Some(item) = p.items.as_ref().and_then(|items| items.get(p.selected)) else {
            return;
        };
        if self.editor.read(cx).value().as_ref() != p.text
            || self.editor.read(cx).cursor() != p.offset
        {
            return;
        }
        let range = &item["textEdit"]["range"];
        let offset = |position: &Value| {
            wirecraft_runtime::lsp::offset(
                &p.text,
                position["line"].as_u64().unwrap_or(u64::MAX) as usize,
                position["character"].as_u64().unwrap_or(u64::MAX) as usize,
                "utf-32",
            )
        };
        let (Ok(start), Ok(end), Some(text)) = (
            offset(&range["start"]),
            offset(&range["end"]),
            item["textEdit"]["newText"].as_str(),
        ) else {
            return;
        };
        if end < start {
            return;
        }
        let utf16_start = p.text[..start].encode_utf16().count();
        let utf16_end = p.text[..end].encode_utf16().count();
        self.editor.update(cx, |state, cx| {
            state.replace_text_in_range(Some(utf16_start..utf16_end), text, w, cx);
            state.focus(w, cx);
        });
        self.persist_ui(cx);
        cx.notify();
    }
    pub(super) fn render_completion(
        &self,
        w: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let pending = self.completion.as_ref()?;
        if self.tab != "Editor" || self.menu.is_some() {
            return None;
        }
        let width = (f32::from(w.viewport_size().width) - 24.).min(760.);
        let height = pending
            .items
            .as_ref()
            .map_or(36., |items| (items.len().max(1) as f32 * 30.).min(270.));
        let x = f32::from(pending.origin.x)
            .min(f32::from(w.viewport_size().width) - width - 12.)
            .max(12.);
        let mut y = f32::from(pending.origin.y);
        if y + height > f32::from(w.viewport_size().height) - 48. {
            y = (y - height - 46.).max(12.);
        }
        let mut menu = div()
            .id("editor-completions")
            .w(px(width))
            .max_h(px(270.))
            .border_1()
            .border_color(crate::chrome::color(0x40505c))
            .rounded_md()
            .bg(crate::chrome::color(0x1c242e))
            .shadow_lg()
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.completion = None;
                cx.notify();
            }))
            .on_mouse_down(MouseButton::Left, |_, w, cx| {
                w.prevent_default();
                cx.stop_propagation();
            });
        if let Some(items) = &pending.items {
            if items.is_empty() {
                menu = menu.child(div().p_2().child("No suggestions for this context"));
            } else {
                menu = menu.child(
                    div()
                        .id("completion-items")
                        .w_full()
                        .max_h(px(270.))
                        .overflow_y_scroll()
                        .track_scroll(&pending.scroll)
                        .children(items.iter().enumerate().map(|(i, item)| {
                            let label = item["label"].as_str().unwrap_or("").to_string();
                            let detail = item["detail"].as_str().unwrap_or("").to_string();
                            div()
                                .id(i)
                                .w_full()
                                .h(px(30.))
                                .px_2()
                                .flex()
                                .items_center()
                                .gap_3()
                                .cursor_pointer()
                                .when(i == pending.selected, |s| {
                                    s.bg(crate::chrome::color(0x255452))
                                })
                                .hover(|s| s.bg(crate::chrome::color(0x293846)))
                                .child(
                                    div()
                                        .w(px(width * 0.46))
                                        .truncate()
                                        .font_family(
                                            self.options.saved.appearance.editor_font.clone(),
                                        )
                                        .child(label),
                                )
                                .child(
                                    div()
                                        .w(px(width * 0.50 - 24.))
                                        .min_w_0()
                                        .truncate()
                                        .text_xs()
                                        .text_color(crate::chrome::color(0x9eb7c4))
                                        .child(detail),
                                )
                                .on_click(cx.listener(move |this, _, w, cx| {
                                    if let Some(p) = this.completion.as_mut() {
                                        p.selected = i;
                                    }
                                    this.insert_completion(w, cx);
                                }))
                        })),
                );
            }
        } else {
            menu = menu.child(div().p_2().child("Loading compiler-context suggestions…"));
        }
        Some(deferred(anchored().position(point(px(x), px(y))).child(menu)).into_any_element())
    }
}
