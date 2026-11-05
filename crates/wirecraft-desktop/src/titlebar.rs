//! The title bar: application, file tools, project, command search and window controls.
use super::*;
use gpui_component::{Sizable, tooltip::Tooltip};

const COMMAND_LIST_WIDTH: f32 = 580.;
/// The search box gives way to the project name, down to a usable minimum.
fn search_width(window_width: f32) -> f32 {
    (window_width * 0.2).clamp(170., 340.)
}
fn tool(id: &'static str, icon: &'static str, tip: &'static str, enabled: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .size(px(26.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .tooltip(move |window, cx| Tooltip::new(tip).build(window, cx))
        .when(enabled, |b| {
            b.cursor_pointer()
                .hover(|s| s.bg(crate::chrome::color(0x2d3b48)))
        })
        .when(!enabled, |b| b.opacity(0.35))
        .child(icons::icon(icon))
}
impl Wirecraft {
    pub(super) fn render_titlebar(
        &self,
        title: String,
        file: String,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = crate::chrome::color(0x8fa6b5);
        let brand = div()
            .flex_none()
            .flex()
            .items_baseline()
            .gap_1()
            .child(
                div()
                    .text_color(crate::chrome::color(0x75d4d6))
                    .child("Wirecraft"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(env!("CARGO_PKG_VERSION")),
            );
        // Occluding and stopping the press keeps these from moving or zooming the window.
        let tools = div()
            .id("title-tools")
            .flex_none()
            .flex()
            .items_center()
            .gap(px(2.))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                tool("title-home", "home", "Home", true)
                    .when(self.home.open, |b| b.bg(crate::chrome::color(0x255452)))
                    .on_click(cx.listener(|this, _, w, cx| this.toggle_home(w, cx))),
            )
            .child(
                tool("title-new", "new", "New · Ctrl+N", true)
                    .on_click(cx.listener(|this, _, w, cx| this.new_project(&New, w, cx))),
            )
            .child(
                tool("title-open", "open", "Open · Ctrl+O", true)
                    .on_click(cx.listener(|this, _, w, cx| this.open(&Open, w, cx))),
            )
            .child(
                tool("title-save", "save", "Save · Ctrl+S", true)
                    .on_click(cx.listener(|this, _, w, cx| this.save(&Save, w, cx))),
            )
            .child(
                tool("title-save-as", "save-as", "Save As · Ctrl+Shift+S", true)
                    .on_click(cx.listener(|this, _, w, cx| this.save_as(&SaveAs, w, cx))),
            )
            .child(
                tool(
                    "title-undo",
                    "undo",
                    "Undo · Ctrl+Z",
                    self.snapshot.can_undo,
                )
                .when(self.snapshot.can_undo, |b| {
                    b.on_click(cx.listener(|this, _, w, cx| this.undo(&Undo, w, cx)))
                }),
            )
            .child(
                tool(
                    "title-redo",
                    "redo",
                    "Redo · Ctrl+Y",
                    self.snapshot.can_redo,
                )
                .when(self.snapshot.can_redo, |b| {
                    b.on_click(cx.listener(|this, _, w, cx| this.redo(&Redo, w, cx)))
                }),
            );
        let project = div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_baseline()
            .gap_2()
            .overflow_hidden()
            .whitespace_nowrap()
            .child(title)
            .child(div().text_xs().text_color(muted).child(file));
        let open = self.menu.as_deref() == Some("Commands");
        let anchors = self.menu_anchors.clone();
        let search = div()
            .id("command-search")
            .relative()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(2.))
            .ml(px(6.))
            .w(px(search_width(f32::from(window.viewport_size().width))))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Input::new(&self.query).xsmall().w_full()),
            )
            .child(
                tool("command-run", "search", "Search commands", true)
                    .on_click(cx.listener(|this, _, w, cx| this.palette(&CommandPalette, w, cx))),
            )
            .child(
                tool(
                    "command-toggle",
                    if open { "chevron-up" } else { "chevron-down" },
                    if open {
                        "Hide command list"
                    } else {
                        "Show command list · Ctrl+Shift+P"
                    },
                    true,
                )
                .when(open, |b| b.bg(crate::chrome::color(0x255452)))
                .on_click(cx.listener(|this, _, w, cx| {
                    if this.menu.as_deref() == Some("Commands") {
                        this.close_command_list(w, cx);
                    } else {
                        this.palette(&CommandPalette, w, cx);
                    }
                })),
            )
            .child(
                canvas(
                    move |b, _, _| {
                        anchors.borrow_mut().insert("Commands", b);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            );
        gpui_component::TitleBar::new()
            .bg(crate::chrome::color(0x10161e))
            .border_color(crate::chrome::color(0x29323d))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap_3()
                    .pr_2()
                    .child(brand)
                    .child(tools)
                    .child(project)
                    .child(search),
            )
            .into_any_element()
    }
    pub(super) fn close_command_list(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        self.query.update(cx, |s, cx| s.set_value("", w, cx));
        self.focus.focus(w);
        cx.notify();
    }
    /// Keys the open command list takes from the search box. Returns whether it took it.
    pub(super) fn command_list_key(
        &mut self,
        key: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.menu.as_deref() != Some("Commands") {
            return false;
        }
        // The same key must not also reach the search box or leave the home screen.
        if ["up", "down", "escape"].contains(&key) {
            cx.stop_propagation();
        }
        let count = workbench::matching_commands(&self.query.read(cx).value()).count();
        match key {
            "up" => self.palette_index = self.palette_index.saturating_sub(1),
            "down" => self.palette_index = (self.palette_index + 1).min(count.saturating_sub(1)),
            "escape" => {
                self.close_command_list(w, cx);
                return true;
            }
            _ => return false,
        }
        self.palette_scroll.scroll_to_item(self.palette_index);
        cx.notify();
        true
    }
    /// Under the search box, with no backdrop; a press outside it closes it.
    pub(super) fn render_command_list(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let padding = gpui_component::window_paddings(window);
        let anchor = self
            .menu_anchors
            .borrow()
            .get("Commands")
            .copied()
            .unwrap_or_default();
        let viewport = window.viewport_size();
        let width = f32::from(viewport.width - padding.left - padding.right);
        let top = f32::from(anchor.bottom() - padding.top) + 3.;
        let left = (f32::from(anchor.right() - padding.left) - COMMAND_LIST_WIDTH)
            .min(width - COMMAND_LIST_WIDTH - 4.)
            .max(4.);
        let height = (f32::from(viewport.height - padding.top - padding.bottom) - top - 8.)
            .clamp(120., 620.);
        let query = self.query.read(cx).value().to_string();
        let matches = workbench::matching_commands(&query).collect::<Vec<_>>();
        let muted = crate::chrome::color(0x8a9aab);
        let anchors = self.menu_anchors.clone();
        div()
            .id("command-panel")
            .occlude()
            .absolute()
            .left(px(left))
            .top(px(top))
            .w(px(COMMAND_LIST_WIDTH))
            .max_h(px(height))
            .flex()
            .flex_col()
            .p_2()
            .gap_1()
            .rounded_md()
            .border_1()
            .border_color(crate::chrome::color(0x40505c))
            .bg(crate::chrome::color(0x1c242e))
            .shadow_lg()
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down_out(cx.listener(move |this, event: &MouseDownEvent, w, cx| {
                if !anchors
                    .borrow()
                    .values()
                    .any(|b| b.contains(&event.position))
                {
                    this.close_command_list(w, cx);
                }
            }))
            .child(div().flex_none().px_1().py_1().child(format!(
                "Commands · {} match{}",
                matches.len(),
                if matches.len() == 1 { "" } else { "es" }
            )))
            .child(
                div()
                    .id("command-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.palette_scroll)
                    .children(matches.into_iter().enumerate().map(|(index, c)| {
                        let id = c.id;
                        div()
                            .id(id)
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_2()
                            .py(px(5.))
                            .rounded_sm()
                            .cursor_pointer()
                            .when(index == self.palette_index, |s| {
                                s.bg(crate::chrome::color(0x255452))
                            })
                            .hover(|s| s.bg(crate::chrome::color(0x2d3b48)))
                            .on_click(
                                cx.listener(move |this, _, w, cx| {
                                    this.run_listed_command(id, w, cx)
                                }),
                            )
                            .child(icons::icon(c.icon))
                            .child(div().flex_1().child(format!("{} > {}", c.group, c.label)))
                            .child(div().text_xs().text_color(muted).child(c.shortcut))
                    })),
            )
            .child(
                div()
                    .flex_none()
                    .px_1()
                    .pt_1()
                    .text_xs()
                    .text_color(muted)
                    .child("↑/↓: select · Enter: execute selected · Escape: close"),
            )
            .into_any_element()
    }
}
