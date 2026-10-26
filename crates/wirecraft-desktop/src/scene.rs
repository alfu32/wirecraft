use gpui::*;
use std::collections::{BTreeMap, BTreeSet};
use wirecraft_core::{
    Document, NodeKind,
    viewport::{Rect, SceneIndex, Viewport, route},
};
pub struct Scene {
    pub index: SceneIndex,
    pub nodes: BTreeMap<String, (String, NodeKind, Rect)>,
    pub links: BTreeMap<String, (String, Vec<[f64; 2]>)>,
}
impl Scene {
    pub fn new(d: &Document, parent: &str) -> Self {
        let mut s = Self {
            index: SceneIndex::default(),
            nodes: BTreeMap::new(),
            links: BTreeMap::new(),
        };
        if let Ok(p) = d.node(parent) {
            for id in &p.children {
                if let Ok(n) = d.node(id)
                    && !n.is_link()
                {
                    let l = &n.layout;
                    let rect = Rect {
                        x: l.x,
                        y: l.y,
                        width: l.width,
                        height: l.height,
                    };
                    s.index.insert(id.clone(), rect);
                    s.nodes.insert(id.clone(), (n.name.clone(), n.kind, rect));
                }
            }
            for id in &p.children {
                if let Ok(n) = d.node(id)
                    && let Some(link) = &n.link
                    && let (Some(a), Some(b)) = (
                        s.nodes.get(&link.source_node_id),
                        s.nodes.get(&link.target_node_id),
                    )
                {
                    let points = route(a.2, b.2);
                    let x = points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
                    let y = points.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
                    let max_x = points
                        .iter()
                        .map(|p| p[0])
                        .fold(f64::NEG_INFINITY, f64::max);
                    let max_y = points
                        .iter()
                        .map(|p| p[1])
                        .fold(f64::NEG_INFINITY, f64::max);
                    s.index.insert(
                        id.clone(),
                        Rect {
                            x: x - 5.,
                            y: y - 20.,
                            width: max_x - x + 10.,
                            height: max_y - y + 40.,
                        },
                    );
                    s.links.insert(id.clone(), (n.name.clone(), points));
                }
            }
        }
        s
    }
    pub fn paint(
        &self,
        bounds: Bounds<Pixels>,
        view: Viewport,
        selected: &BTreeSet<String>,
        drag: [f64; 2],
        window: &mut Window,
        cx: &mut App,
    ) {
        let width = f64::from(f32::from(bounds.size.width));
        let height = f64::from(f32::from(bounds.size.height));
        let origin = view.world([0., 0.]);
        let visible = self.index.query(Rect {
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
            window.paint_path(path, rgb(0x202a38));
        }
        for id in &visible {
            if let Some((name, points)) = self.links.get(id) {
                let mut builder =
                    PathBuilder::stroke(px(if selected.contains(id) { 3. } else { 1.5 }));
                for (i, p) in points.iter().enumerate() {
                    if i == 0 {
                        builder.move_to(screen(*p));
                    } else {
                        builder.line_to(screen(*p));
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
                            0xf6c86c
                        } else {
                            0x66b9cb
                        }),
                    );
                }
                if let Some(p) = points.get(1) {
                    label(
                        name,
                        screen([p[0] + 4., p[1] - 20.]),
                        12.,
                        0x8fabbf,
                        window,
                        cx,
                    );
                }
            }
        }
        for id in &visible {
            if let Some((name, kind, r)) = self.nodes.get(id) {
                let active = selected.contains(id);
                let offset = if active { drag } else { [0., 0.] };
                let rect = Bounds::new(
                    screen([r.x + offset[0], r.y + offset[1]]),
                    size(
                        px((r.width * view.zoom) as f32),
                        px((r.height * view.zoom) as f32),
                    ),
                );
                window.paint_quad(quad(
                    rect,
                    px(4.),
                    rgb(if *kind == NodeKind::Type {
                        0x263445
                    } else {
                        0x162438
                    }),
                    px(if active { 2. } else { 1. }),
                    rgb(if active { 0xf6c86c } else { 0x47718c }),
                    BorderStyle::Solid,
                ));
                if view.zoom >= 0.3 {
                    label(
                        &format!("{kind:?}"),
                        rect.origin + point(px(9.), px(6.)),
                        (10. * view.zoom as f32).max(8.),
                        0x78b9c9,
                        window,
                        cx,
                    );
                    label(
                        name,
                        rect.origin + point(px(9.), px((28. * view.zoom) as f32)),
                        (14. * view.zoom as f32).max(9.),
                        0xe6edf3,
                        window,
                        cx,
                    );
                }
            }
        }
    }
}
fn label(
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
        font: font("DejaVu Sans"),
        color: rgb(color).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let line = window
        .text_system()
        .shape_line(clean.into(), px(font_size), &[run], None);
    let _ = line.paint(origin, px(font_size + 5.), window, cx);
}
