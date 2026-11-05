//! Linux has no native prompt. GPUI's fallback lets clicks reach what lies behind it: a press
//! on its button also pressed a home screen row, which asked again instead of proceeding.
use gpui::{prelude::*, *};

fn is_cancel(action: &PromptButton) -> bool {
    matches!(action, PromptButton::Cancel(_))
}
struct ThemedPrompt {
    message: String,
    detail: Option<String>,
    actions: Vec<PromptButton>,
    focus: FocusHandle,
}
impl EventEmitter<PromptResponse> for ThemedPrompt {}
impl Focusable for ThemedPrompt {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Render for ThemedPrompt {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = crate::chrome::color(0x8fa6b5);
        let dialog = div()
            .id("prompt")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                let chosen = match event.keystroke.key.as_str() {
                    "enter" => Some(0),
                    "escape" => this.actions.iter().position(is_cancel),
                    _ => None,
                };
                if let Some(index) = chosen {
                    cx.emit(PromptResponse(index));
                }
            }))
            .w(px(380.))
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_lg()
            .border_1()
            .border_color(crate::chrome::color(0x40505c))
            .bg(crate::chrome::color(0x1c242e))
            .text_color(crate::chrome::color(0xe1e9ef))
            .shadow_lg()
            .child(self.message.clone())
            .children(
                self.detail
                    .clone()
                    .map(|detail| div().text_sm().text_color(muted).child(detail)),
            )
            .child(div().flex().justify_end().gap_2().children(
                self.actions.iter().enumerate().map(|(index, action)| {
                    div()
                        .id(index)
                        .px_3()
                        .py_1()
                        .rounded_sm()
                        .cursor_pointer()
                        .text_sm()
                        .bg(crate::chrome::color(if is_cancel(action) {
                            0x242c36
                        } else {
                            0x255452
                        }))
                        .hover(|s| s.bg(crate::chrome::color(0x34414d)))
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.emit(PromptResponse(index));
                        }))
                        .child(action.label().clone())
                }),
            ));
        div()
            .id("prompt-backdrop")
            .size_full()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x00000080))
            .child(dialog)
    }
}
pub fn build(
    _: PromptLevel,
    message: &str,
    detail: Option<&str>,
    actions: &[PromptButton],
    handle: PromptHandle,
    window: &mut Window,
    cx: &mut App,
) -> RenderablePromptHandle {
    let view = cx.new(|cx| ThemedPrompt {
        message: message.into(),
        detail: detail.map(Into::into),
        actions: actions.to_vec(),
        focus: cx.focus_handle(),
    });
    handle.with_view(view, window, cx)
}
