// From:
// https://github.com/zed-industries/zed/blob/a8afc63a91f6b75528540dcffe73dc8ce0c92ad8/crates/gpui/examples/window_shadow.rs
use gpui::{
    AnyElement, App, Bounds, CursorStyle, Decorations, Edges, HitboxBehavior, Hsla,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce,
    ResizeEdge, Size, Styled as _, Window, canvas, div, point, prelude::FluentBuilder as _, px,
};

use crate::ActiveTheme;

#[cfg(not(target_os = "linux"))]
const SHADOW_SIZE: Pixels = px(0.0);
#[cfg(target_os = "linux")]
const SHADOW_SIZE: Pixels = px(12.0);
const BORDER_SIZE: Pixels = px(1.0);
pub(crate) const BORDER_RADIUS: Pixels = px(0.0);

/// Create a new window border.
pub fn window_border() -> WindowBorder {
    WindowBorder::new()
}

/// Window border use to render a custom window border and shadow for Linux.
#[derive(IntoElement, Default)]
pub struct WindowBorder {
    children: Vec<AnyElement>,
}

impl WindowBorder {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

/// Get the window paddings.
pub fn window_paddings(window: &Window) -> Edges<Pixels> {
    match window.window_decorations() {
        Decorations::Server => Edges::all(px(0.0)),
        Decorations::Client { tiling } => {
            let mut paddings = Edges::all(SHADOW_SIZE);
            if tiling.top {
                paddings.top = px(0.0);
            }
            if tiling.bottom {
                paddings.bottom = px(0.0);
            }
            if tiling.left {
                paddings.left = px(0.0);
            }
            if tiling.right {
                paddings.right = px(0.0);
            }
            paddings
        }
    }
}

impl ParentElement for WindowBorder {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for WindowBorder {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let decorations = window.window_decorations();
        window.set_client_inset(SHADOW_SIZE);

        div()
            .id("window-backdrop")
            .bg(gpui::transparent_black())
            .map(|div| match decorations {
                Decorations::Server => div,
                Decorations::Client { tiling, .. } => div
                    .bg(gpui::transparent_black())
                    .child(
                        canvas(
                            move |_bounds, window, _| {
                                let size = window.viewport_size();
                                let gutter = SHADOW_SIZE;
                                let (w, h) = (size.width, size.height);
                                let specs = [
                                    (
                                        point(px(0.), px(0.)),
                                        gpui::size(gutter, gutter),
                                        ResizeEdge::TopLeft,
                                        !tiling.top && !tiling.left,
                                    ),
                                    (
                                        point(gutter, px(0.)),
                                        gpui::size((w - gutter * 2.).max(px(0.)), gutter),
                                        ResizeEdge::Top,
                                        !tiling.top,
                                    ),
                                    (
                                        point(w - gutter, px(0.)),
                                        gpui::size(gutter, gutter),
                                        ResizeEdge::TopRight,
                                        !tiling.top && !tiling.right,
                                    ),
                                    (
                                        point(px(0.), gutter),
                                        gpui::size(gutter, (h - gutter * 2.).max(px(0.))),
                                        ResizeEdge::Left,
                                        !tiling.left,
                                    ),
                                    (
                                        point(w - gutter, gutter),
                                        gpui::size(gutter, (h - gutter * 2.).max(px(0.))),
                                        ResizeEdge::Right,
                                        !tiling.right,
                                    ),
                                    (
                                        point(px(0.), h - gutter),
                                        gpui::size(gutter, gutter),
                                        ResizeEdge::BottomLeft,
                                        !tiling.bottom && !tiling.left,
                                    ),
                                    (
                                        point(gutter, h - gutter),
                                        gpui::size((w - gutter * 2.).max(px(0.)), gutter),
                                        ResizeEdge::Bottom,
                                        !tiling.bottom,
                                    ),
                                    (
                                        point(w - gutter, h - gutter),
                                        gpui::size(gutter, gutter),
                                        ResizeEdge::BottomRight,
                                        !tiling.bottom && !tiling.right,
                                    ),
                                ];
                                specs
                                    .into_iter()
                                    .filter(|(_, _, _, enabled)| *enabled)
                                    .map(|(origin, size, edge, _)| {
                                        (
                                            window.insert_hitbox(
                                                Bounds::new(origin, size),
                                                HitboxBehavior::Normal,
                                            ),
                                            edge,
                                        )
                                    })
                                    .collect::<Vec<_>>()
                            },
                            move |_bounds, hitboxes, window, _| {
                                for (hitbox, edge) in hitboxes {
                                    let style = match edge {
                                        ResizeEdge::Top | ResizeEdge::Bottom => {
                                            CursorStyle::ResizeUpDown
                                        }
                                        ResizeEdge::Left | ResizeEdge::Right => {
                                            CursorStyle::ResizeLeftRight
                                        }
                                        ResizeEdge::TopLeft | ResizeEdge::BottomRight => {
                                            CursorStyle::ResizeUpLeftDownRight
                                        }
                                        ResizeEdge::TopRight | ResizeEdge::BottomLeft => {
                                            CursorStyle::ResizeUpRightDownLeft
                                        }
                                    };
                                    window.set_cursor_style(style, &hitbox);
                                }
                            },
                        )
                        .size_full()
                        .absolute(),
                    )
                    .when(!(tiling.top || tiling.right), |div| {
                        div.rounded_tr(BORDER_RADIUS)
                    })
                    .when(!(tiling.top || tiling.left), |div| {
                        div.rounded_tl(BORDER_RADIUS)
                    })
                    .when(!tiling.top, |div| div.pt(SHADOW_SIZE))
                    .when(!tiling.bottom, |div| div.pb(SHADOW_SIZE))
                    .when(!tiling.left, |div| div.pl(SHADOW_SIZE))
                    .when(!tiling.right, |div| div.pr(SHADOW_SIZE))
                    .on_mouse_down(MouseButton::Left, move |_, window, _| {
                        let size = window.viewport_size();
                        let pos = window.mouse_position();

                        match resize_edge(pos, SHADOW_SIZE, size) {
                            Some(edge) => {
                                let enabled = match edge {
                                    ResizeEdge::Top => !tiling.top,
                                    ResizeEdge::Bottom => !tiling.bottom,
                                    ResizeEdge::Left => !tiling.left,
                                    ResizeEdge::Right => !tiling.right,
                                    ResizeEdge::TopLeft => !tiling.top && !tiling.left,
                                    ResizeEdge::TopRight => !tiling.top && !tiling.right,
                                    ResizeEdge::BottomLeft => !tiling.bottom && !tiling.left,
                                    ResizeEdge::BottomRight => !tiling.bottom && !tiling.right,
                                };
                                if enabled {
                                    window.start_window_resize(edge);
                                }
                            }
                            None => {}
                        };
                    }),
            })
            .size_full()
            .child(
                div()
                    .map(|div| match decorations {
                        Decorations::Server => div,
                        Decorations::Client { tiling } => div
                            .when(!(tiling.top || tiling.right), |div| {
                                div.rounded_tr(BORDER_RADIUS)
                            })
                            .when(!(tiling.top || tiling.left), |div| {
                                div.rounded_tl(BORDER_RADIUS)
                            })
                            .border_color(cx.theme().window_border)
                            .when(!tiling.top, |div| div.border_t(BORDER_SIZE))
                            .when(!tiling.bottom, |div| div.border_b(BORDER_SIZE))
                            .when(!tiling.left, |div| div.border_l(BORDER_SIZE))
                            .when(!tiling.right, |div| div.border_r(BORDER_SIZE))
                            .when(!tiling.is_tiled(), |div| {
                                div.shadow(vec![gpui::BoxShadow {
                                    color: Hsla {
                                        h: 0.,
                                        s: 0.,
                                        l: 0.,
                                        a: 0.3,
                                    },
                                    blur_radius: SHADOW_SIZE / 2.,
                                    spread_radius: px(0.),
                                    offset: point(px(0.0), px(0.0)),
                                }])
                            }),
                    })
                    .on_mouse_move(|_e, _, cx| {
                        cx.stop_propagation();
                    })
                    .bg(gpui::transparent_black())
                    .size_full()
                    .children(self.children),
            )
    }
}

fn resize_edge(pos: Point<Pixels>, shadow_size: Pixels, size: Size<Pixels>) -> Option<ResizeEdge> {
    let edge = if pos.y < shadow_size && pos.x < shadow_size {
        ResizeEdge::TopLeft
    } else if pos.y < shadow_size && pos.x > size.width - shadow_size {
        ResizeEdge::TopRight
    } else if pos.y < shadow_size {
        ResizeEdge::Top
    } else if pos.y > size.height - shadow_size && pos.x < shadow_size {
        ResizeEdge::BottomLeft
    } else if pos.y > size.height - shadow_size && pos.x > size.width - shadow_size {
        ResizeEdge::BottomRight
    } else if pos.y > size.height - shadow_size {
        ResizeEdge::Bottom
    } else if pos.x < shadow_size {
        ResizeEdge::Left
    } else if pos.x > size.width - shadow_size {
        ResizeEdge::Right
    } else {
        return None;
    };
    Some(edge)
}
