use gpui::*;
use std::collections::{BTreeMap, BTreeSet};
use wirecraft_core::{
    Document, NodeKind,
    viewport::{Rect, RouteIndex, SceneIndex, Viewport},
};
pub struct Scene {
    appearance: wirecraft_runtime::config::Appearance,
    pub drawing: wirecraft_core::drawing::Drawing,
    index: SceneIndex,
    routes: RouteIndex,
    pub nodes: BTreeMap<String, (String, NodeKind, Rect)>,
    pub links: BTreeMap<String, (String, Vec<[f64; 2]>)>,
}
impl Scene {
    pub fn new(d: &Document, parent: &str) -> Self {
        let mut s = Self {
            appearance: wirecraft_runtime::config::Appearance::default(),
            drawing: wirecraft_core::drawing::Drawing::new(d, "__empty__"),
            index: SceneIndex::default(),
            routes: RouteIndex::default(),
            nodes: BTreeMap::new(),
            links: BTreeMap::new(),
        };
        let drawing = wirecraft_core::drawing::Drawing::new(d, parent);
        for (id, shape) in &drawing.shapes {
            s.index.insert(id.clone(), shape.rect);
            s.nodes
                .insert(id.clone(), (shape.name.clone(), shape.kind, shape.rect));
        }
        for (id, edge) in &drawing.edges {
            s.links
                .insert(id.clone(), (edge.name.clone(), edge.points.clone()));
        }
        s.routes = RouteIndex::new(
            drawing
                .edges
                .iter()
                .map(|(id, edge)| (id.as_str(), edge.points.as_slice())),
        );
        for (id, annotation) in &drawing.annotations {
            s.index.insert(
                id.clone(),
                Rect {
                    x: annotation.rect.x - 22.,
                    width: annotation.rect.width + 22.,
                    ..annotation.rect
                },
            );
        }
        s.drawing = drawing;
        s
    }
    pub fn query(&self, rect: Rect) -> Vec<String> {
        let mut ids = self.index.query(rect);
        ids.extend(self.routes.query(rect));
        ids
    }
    pub fn with_appearance(mut self, appearance: wirecraft_runtime::config::Appearance) -> Self {
        self.appearance = appearance;
        self
    }
    fn color(&self, value: u32) -> u32 {
        let key = match value {
            0x10161e => "canvasBackground",
            0x202a38 => "gridMinor",
            0x34404d => "gridMajor",
            0x4c3b17 => "compilerFill",
            0xf0ad36 => "compilerStroke",
            0x202c38 => "nodeFill",
            0x8196a9 => "nodeStroke",
            0x173b3a => "typeFill",
            0x45baa8 => "typeStroke",
            0x58c6a4 => "typeText",
            0x292e43 => "libraryFill",
            0x8893e4 => "libraryStroke",
            0x432a2a => "errorFill",
            0xe27b7b => "errorStroke",
            0x273d2c => "testFill",
            0x6ac284 => "testStroke",
            0xe6edf3 => "textPrimary",
            0x9db0c3 => "textSecondary",
            0x829bad => "textMuted",
            0x70d5c6 | 0xf6c86c => "selection",
            0x9cb5c8 => "linkDefault",
            0xe3b664 => "linkDependency",
            _ => return value,
        };
        self.appearance.color(key, value)
    }
    pub fn fitted(&self, size: [f64; 2]) -> Viewport {
        if self.nodes.is_empty() {
            return Viewport::default();
        }
        let rects = self
            .nodes
            .values()
            .map(|n| n.2)
            .chain(self.drawing.annotations.values().map(|a| a.rect))
            .collect::<Vec<_>>();
        let min_x = rects.iter().map(|r| r.x).fold(f64::INFINITY, f64::min);
        let min_y = rects.iter().map(|r| r.y).fold(f64::INFINITY, f64::min);
        let max_x = rects
            .iter()
            .map(|r| r.x + r.width)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = rects
            .iter()
            .map(|r| r.y + r.height)
            .fold(f64::NEG_INFINITY, f64::max);
        let zoom = ((size[0] - 80.) / (max_x - min_x).max(1.))
            .min((size[1] - 80.) / (max_y - min_y).max(1.))
            .clamp(0.05, 2.);
        Viewport {
            x: 40. - min_x * zoom,
            y: 40. - min_y * zoom,
            zoom,
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn paint(
        &self,
        bounds: Bounds<Pixels>,
        view: Viewport,
        selected: &BTreeSet<String>,
        moving: &BTreeSet<String>,
        drag: [f64; 2],
        window: &mut Window,
        cx: &mut App,
    ) {
        window.paint_quad(fill(bounds, rgb(self.color(0x10161e))));
        let width = f64::from(f32::from(bounds.size.width));
        let height = f64::from(f32::from(bounds.size.height));
        let origin = view.world([0., 0.]);
        let visible = self.query(Rect {
            x: origin[0] - 200.,
            y: origin[1] - 100.,
            width: width / view.zoom + 400.,
            height: height / view.zoom + 200.,
        });
        let screen = |p: [f64; 2]| {
            let p = view.screen(p);
            point(
                bounds.origin.x + px(p[0] as f32),
                bounds.origin.y + px(p[1] as f32),
            )
        };
        let mut grid = PathBuilder::stroke(px(1.));
        let spacing = if view.zoom < 0.4 { 100. } else { 25. };
        let mut x = (origin[0] / spacing).floor() * spacing;
        while x < origin[0] + width / view.zoom {
            grid.move_to(screen([x, origin[1]]));
            grid.line_to(screen([x, origin[1] + height / view.zoom]));
            x += spacing;
        }
        let mut y = (origin[1] / spacing).floor() * spacing;
        while y < origin[1] + height / view.zoom {
            grid.move_to(screen([origin[0], y]));
            grid.line_to(screen([origin[0] + width / view.zoom, y]));
            y += spacing;
        }
        if let Ok(path) = grid.build() {
            window.paint_path(path, rgb(self.color(0x202a38)));
        }
        let mut major = PathBuilder::stroke(px(1.));
        let step = spacing * 5.;
        let mut x = (origin[0] / step).floor() * step;
        while x < origin[0] + width / view.zoom {
            major.move_to(screen([x, origin[1]]));
            major.line_to(screen([x, origin[1] + height / view.zoom]));
            x += step;
        }
        let mut y = (origin[1] / step).floor() * step;
        while y < origin[1] + height / view.zoom {
            major.move_to(screen([origin[0], y]));
            major.line_to(screen([origin[0] + width / view.zoom, y]));
            y += step;
        }
        if let Ok(path) = major.build() {
            window.paint_path(path, rgb(self.color(0x34404d)));
        }
        let mut ordered = visible
            .iter()
            .filter_map(|id| self.drawing.shapes.get(id).map(|shape| (id, shape)))
            .collect::<Vec<_>>();
        ordered.sort_by_key(|(_, shape)| shape.depth);
        for (id, shape) in &ordered {
            if shape.composite && shape.expanded {
                self.paint_shape(id, shape, view, selected, moving, drag, &screen, window, cx);
            }
        }
        for id in &visible {
            if let Some((name, points)) = self.links.get(id) {
                let edge = &self.drawing.edges[id];
                let mut shifted = points.clone();
                if moving.contains(&edge.source) && moving.contains(&edge.target) {
                    for p in &mut shifted {
                        p[0] += drag[0];
                        p[1] += drag[1];
                    }
                } else if moving.contains(&edge.source) || moving.contains(&edge.target) {
                    let mut a = shifted[0];
                    let mut b = *shifted.last().unwrap();
                    if moving.contains(&edge.source) {
                        a[0] += drag[0];
                        a[1] += drag[1];
                    }
                    if moving.contains(&edge.target) {
                        b[0] += drag[0];
                        b[1] += drag[1];
                    }
                    let mid = (a[0] + b[0]) / 2.;
                    shifted = vec![a, [mid, a[1]], [mid, b[1]], b];
                }
                let points = &shifted;
                let mut builder =
                    PathBuilder::stroke(px(if selected.contains(id) { 3. } else { 1.5 }));
                if edge.dashed {
                    for segment in points.windows(2) {
                        let a = screen(segment[0]);
                        let b = screen(segment[1]);
                        let delta = b - a;
                        let length =
                            (f32::from(delta.x).powi(2) + f32::from(delta.y).powi(2)).sqrt();
                        if length < 0.1 {
                            continue;
                        }
                        let mut start = 0.;
                        while start < length {
                            builder.move_to(a + delta * (start / length));
                            builder.line_to(a + delta * ((start + 6.).min(length) / length));
                            start += 10.;
                        }
                    }
                } else {
                    for (i, p) in points.iter().enumerate() {
                        if i == 0 {
                            builder.move_to(screen(*p));
                        } else {
                            builder.line_to(screen(*p));
                        }
                    }
                }
                if let Some(end) = points.last() {
                    builder.move_to(screen([end[0] - 8., end[1] - 4.]));
                    builder.line_to(screen(*end));
                    builder.line_to(screen([end[0] - 8., end[1] + 4.]));
                }
                if let Ok(path) = builder.build() {
                    window.paint_path(
                        path,
                        rgb(if selected.contains(id) {
                            self.appearance.color("selection", 0x70d5c6)
                        } else {
                            self.appearance.color(edge.color_key, 0x9cb5c8)
                        }),
                    );
                }
                if view.zoom >= 0.35 {
                    let edge = &self.drawing.edges[id];
                    for (p, label_text) in [
                        (points.first(), &edge.source_label),
                        (points.last(), &edge.target_label),
                    ] {
                        if let Some(p) = p {
                            let pos = screen(*p);
                            window.paint_quad(quad(
                                Bounds::new(pos - point(px(3.), px(3.)), size(px(6.), px(6.))),
                                px(3.),
                                rgb(self.color(0x70d5c6)),
                                px(0.),
                                rgb(self.color(0x70d5c6)),
                                BorderStyle::Solid,
                            ));
                            self.label(
                                label_text,
                                pos + point(px(6.), px(-13.)),
                                (9. * view.zoom as f32).max(5.),
                                0x70bdb3,
                                window,
                                cx,
                            );
                        }
                    }
                }
                if let Some(p) = points.get(1) {
                    self.label(
                        name,
                        screen([p[0] + 4., p[1] - 20.]),
                        (10. * view.zoom as f32).max(5.),
                        0x8fabbf,
                        window,
                        cx,
                    );
                }
            }
        }
        for (id, shape) in &ordered {
            if !shape.composite || !shape.expanded {
                self.paint_shape(id, shape, view, selected, moving, drag, &screen, window, cx);
            }
        }
        for id in &visible {
            if let Some(annotation) = self.drawing.annotations.get(id) {
                let mut r = annotation.rect;
                let offset = if moving.contains(&annotation.owner) {
                    drag
                } else {
                    [0., 0.]
                };
                r.x += offset[0];
                r.y += offset[1];
                let selected = selected.contains(&annotation.target);
                let rect = Bounds::new(
                    screen([r.x, r.y]),
                    size(
                        px((r.width * view.zoom) as f32),
                        px((r.height * view.zoom) as f32),
                    ),
                );
                let mut owner = self.drawing.shapes[&annotation.owner].rect;
                owner.x += offset[0];
                owner.y += offset[1];
                let mut connector = PathBuilder::stroke(px(1.));
                connector.move_to(screen([owner.x + owner.width, r.y + r.height / 2.]));
                connector.line_to(screen([r.x, r.y + r.height / 2.]));
                if let Ok(path) = connector.build() {
                    window.paint_path(path, rgb(self.color(0x45baa8)));
                }
                window.paint_quad(quad(
                    rect,
                    px(1.),
                    rgb(self.color(0x173b3a)),
                    px(if selected { 2. } else { 1. }),
                    rgb(self.color(if selected { 0x70d5c6 } else { 0x45baa8 })),
                    BorderStyle::Solid,
                ));
                if view.zoom >= 0.3 {
                    self.label(
                        &annotation.label,
                        rect.origin + point(px(4. * view.zoom as f32), px(2. * view.zoom as f32)),
                        (10. * view.zoom as f32).max(5.),
                        0x58c6a4,
                        window,
                        cx,
                    );
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn paint_shape(
        &self,
        id: &str,
        shape: &wirecraft_core::drawing::Shape,
        view: Viewport,
        selected: &BTreeSet<String>,
        moving: &BTreeSet<String>,
        drag: [f64; 2],
        screen: &impl Fn([f64; 2]) -> Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let r = shape.rect;
        let active = selected.contains(id);
        let offset = if moving.contains(id) { drag } else { [0., 0.] };
        let rect = Bounds::new(
            screen([r.x + offset[0], r.y + offset[1]]),
            size(
                px((r.width * view.zoom) as f32),
                px((r.height * view.zoom) as f32),
            ),
        );
        let (fill_color, stroke) = match shape.role.as_str() {
            "Type" => (0x173b3a, 0x45baa8),
            "ServiceLibrary" => (0x292e43, 0x8893e4),
            "CompilerTemplate" => (0x4c3b17, 0xf0ad36),
            "ErrorHandler" | "CompositeErrorHandler" => (0x432a2a, 0xe27b7b),
            "Test" | "TestSuite" => (0x273d2c, 0x6ac284),
            _ => (0x202c38, 0x8196a9),
        };
        window.paint_quad(quad(
            rect,
            px(4.),
            rgb(self.color(fill_color)),
            px(if active { 2. } else { 1. }),
            rgb(self.color(if active { 0x70d5c6 } else { stroke })),
            if shape.composite && shape.expanded {
                BorderStyle::Dashed
            } else {
                BorderStyle::Solid
            },
        ));
        if view.zoom >= 0.2 {
            let mut y = 8.;
            for (text, size, color) in [
                (
                    format!(
                        "{}{} {}",
                        if shape.composite {
                            if shape.expanded { "⊟ " } else { "⊞ " }
                        } else {
                            ""
                        },
                        shape.name,
                        shape.detail
                    ),
                    14.,
                    0xe6edf3,
                ),
                (shape.role.clone(), 11., 0x9db0c3),
                (shape.technology.clone(), 10., 0x829bad),
            ] {
                self.label(
                    &text,
                    rect.origin + point(px(9. * view.zoom as f32), px(y * view.zoom as f32)),
                    if shape.composite && shape.expanded && size == 14. {
                        let viewport = window.viewport_size();
                        let scale = (f64::from(f32::from(viewport.width))
                            / self.appearance.reference_width)
                            .min(
                                f64::from(f32::from(viewport.height))
                                    / self.appearance.reference_height,
                            )
                            .clamp(0.4, 2.);
                        self.appearance.composite_title_size * scale as f32
                    } else {
                        (size * view.zoom as f32).max(7.)
                    },
                    color,
                    window,
                    cx,
                );
                y += if shape.composite && shape.expanded {
                    (self.appearance.composite_title_size + 3.) / view.zoom as f32
                } else {
                    19.
                };
            }
            for field in &shape.fields {
                if y + 14. > r.height as f32 {
                    break;
                }
                self.label(
                    field,
                    rect.origin + point(px(9. * view.zoom as f32), px(y * view.zoom as f32)),
                    (10. * view.zoom as f32).max(7.),
                    0x58c6a4,
                    window,
                    cx,
                );
                y += 15.;
            }
        }
        if active {
            window.paint_quad(fill(
                Bounds::new(
                    rect.bottom_right() - point(px(6.), px(6.)),
                    size(px(6.), px(6.)),
                ),
                rgb(self.color(0x70d5c6)),
            ));
        }
        if shape.diagnostics > 0 {
            window.paint_quad(quad(
                Bounds::new(
                    rect.top_right() - point(px(12.), px(-4.)),
                    size(px(8.), px(8.)),
                ),
                px(4.),
                rgb(0xe37272),
                px(0.),
                rgb(0xe37272),
                BorderStyle::Solid,
            ));
        }
    }
    fn label(
        &self,
        text: &str,
        origin: Point<Pixels>,
        font_size: f32,
        color: u32,
        window: &mut Window,
        cx: &mut App,
    ) {
        let clean = text.replace(['\n', '\r'], " ");
        let run = TextRun {
            len: clean.len(),
            font: font(self.appearance.designer_font.clone()),
            color: rgb(self.color(color)).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = window
            .text_system()
            .shape_line(clean.into(), px(font_size), &[run], None);
        let _ = line.paint(origin, px(font_size + 5.), window, cx);
    }
}
