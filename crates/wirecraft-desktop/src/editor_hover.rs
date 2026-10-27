//! Hover uses the same generated body service as completion and coloring.
use super::*;
use gpui_component::input::{HoverProvider, Rope};
use std::rc::Rc;
struct Provider {
    owner: WeakEntity<Wirecraft>,
}
impl HoverProvider for Provider {
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<anyhow::Result<Option<lsp_types::Hover>>> {
        let text = text.to_string();
        let owner = self.owner.clone();
        cx.spawn(async move |cx| {
            Timer::after(std::time::Duration::from_millis(300)).await;
            let Some((request, sender, node, section, revision)) = owner
                .update(cx, |this, cx| {
                    let node = this.editor_entity.clone()?;
                    if this.tab != "Editor"
                        || this.editor.read(cx).value().as_ref() != text
                        || this
                            .snapshot
                            .document
                            .effective_text_language(&node, &this.section)
                            != "c"
                    {
                        return None;
                    }
                    let mut request = Request::new(
                        "editor.hover",
                        json!({"nodeId":node,"section":this.section,"text":text,"offset":offset}),
                    );
                    request.expected_revision = Some(this.snapshot.revision);
                    Some((
                        request,
                        this.client.sender.clone(),
                        node,
                        this.section.clone(),
                        this.snapshot.revision,
                    ))
                })
                .ok()
                .flatten()
            else {
                return Ok(None);
            };
            let (reply, result) = async_channel::bounded(1);
            sender
                .send(Message::Query { request, reply })
                .map_err(|_| anyhow::anyhow!("Backend disconnected"))?;
            let value = result.recv().await?.map_err(anyhow::Error::msg)?;
            let current = owner
                .update(cx, |this, cx| {
                    this.editor_entity.as_ref() == Some(&node)
                        && this.section == section
                        && this.snapshot.revision == revision
                        && this.editor.read(cx).value().as_ref() == text
                })
                .unwrap_or(false);
            if !current || value["hover"].is_null() {
                return Ok(None);
            }
            Ok(Some(serde_json::from_value(value["hover"].clone())?))
        })
    }
}
impl Wirecraft {
    pub(super) fn attach_hover(&self, cx: &mut Context<Self>) {
        let provider = Rc::new(Provider {
            owner: cx.entity().downgrade(),
        });
        self.editor
            .update(cx, |state, _| state.lsp.hover_provider = Some(provider));
    }
}
