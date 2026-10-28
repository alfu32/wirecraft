use gpui::*;
use std::collections::{BTreeMap, BTreeSet};
use wirecraft_core::{
    Document, NodeKind,
    viewport::{Rect, RouteIndex, SceneIndex, Viewport},
};
/// Contour and badge color of entities the user marked as doing IO.
const IO_COLOR: u32 = 0xe8a33d;
/// Strokes a rectangle with dashes that continue around its corners.
fn dashed_outline(
    rect: Bounds<Pixels>,
    width: f32,
    dash: f32,
    gap: f32,
    color: Rgba,
    window: &mut Window,
) {
    let corners = [
        rect.origin,
        rect.top_right(),
        rect.bottom_right(),
        rect.bottom_left(),
        rect.origin,
    ];
    let mut path = PathBuilder::stroke(px(width));
    // Distance into the current dash/gap period, carried across corners.
    let mut phase = 0.;
    let period = dash + gap;
    for side in corners.windows(2) {
        let (a, b) = (side[0], side[1]);
        let delta = b - a;
        let length = (f32::from(delta.x).powi(2) + f32::from(delta.y).powi(2)).sqrt();
        if length < 0.1 {
            continue;
        }
        let at = |t: f32| a + delta * (t / length);
        let mut t = 0.;
        while t < length {
            let span = if phase < dash {
                dash - phase
            } else {
                period - phase
            };
            let end = (t + span).min(length);
            if phase < dash {
                path.move_to(at(t));
                path.line_to(at(end));
            }
            phase = (phase + end - t) % period;
            t = end;
        }
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}
pub struct Scene {
    parent: String,
    appearance: wirecraft_runtime::config::Appearance,
    pub drawing: wirecraft_core::drawing::Drawing,
    index: SceneIndex,
    routes: RouteIndex,
    incident: BTreeMap<String, Vec<String>>,
    pub nodes: BTreeMap<String, (String, NodeKind, Rect)>,
    pub links: BTreeMap<String, (String, Vec<[f64; 2]>)>,
}
impl Scene {
    pub fn new(d: &Document, parent: &str) -> Self {
        let mut s = Self {
            parent: parent.into(),
            appearance: wirecraft_runtime::config::Appearance::default(),
            drawing: wirecraft_core::drawing::Drawing::new(d, "__empty__"),
            index: SceneIndex::default(),
            routes: RouteIndex::default(),
            incident: BTreeMap::new(),
            nodes: BTreeMap::new(),
            links: BTreeMap::new(),
        };
        let drawing = wirecraft_core::drawing::Drawing::new(d, parent);
        for (id, shape) in &drawing.shapes {
            s.index.insert(id.clone(), shape.rect);
            s.nodes
                .insert(id.clone(), (shape.name.clone(), shape.kind, shape.rect));
        }
        for (id, edge) in drawing.edges.iter().filter(|(_, e)| !e.annotation_only) {
            for endpoint in [&edge.source, &edge.target] {
                s.incident
                    .entry(endpoint.clone())
                    .or_default()
                    .push(id.clone());
            }
            s.links
                .insert(id.clone(), (edge.name.clone(), edge.points.clone()));
        }
        let mut indexed_paths = Vec::new();
        for (id, edge) in drawing.edges.iter().filter(|(_, e)| !e.annotation_only) {
            indexed_paths.push((id.as_str(), edge.points.clone()));
            let (mut output, mut input) = wirecraft_core::drawing::port_glyphs(&edge.points);
            if let Some(first) = output.first().copied() {
                output.push(first);
                indexed_paths.push((id.as_str(), output));
            }
            if let Some(first) = input.first().copied() {
                input.push(first);
                indexed_paths.push((id.as_str(), input));
            }
        }
        s.routes = RouteIndex::new(indexed_paths.iter().map(|(id, p)| (*id, p.as_slice())));
        for (id, annotation) in &drawing.annotations {
            s.index.insert(
                id.clone(),
                Rect {
                    x: annotation.rect.x - 24.,
                    width: annotation.rect.width + 24.,
                    height: if annotation.flag {
                        drawing.shapes[&annotation.owner].rect.y - annotation.rect.y
                    } else {
                        annotation.rect.height
                    },
                    ..annotation.rect
                },
            );
        }
        s.drawing = drawing;
        s
    }
    pub fn contains_entity(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
            || self.links.contains_key(id)
            || self.drawing.annotations.values().any(|a| a.target == id)
    }
    pub fn query(&self, rect: Rect) -> Vec<String> {
        let mut ids = self.index.query(rect);
        ids.extend(self.routes.query(rect));
        ids.sort();
        ids.dedup();
        ids
    }
    pub fn pick(&self, world: [f64; 2], tolerance: f64, include_links: bool) -> Option<String> {
        let hits = self.query(Rect {
            x: world[0] - tolerance,
            y: world[1] - tolerance,
            width: 2. * tolerance,
            height: 2. * tolerance,
        });
        let shape = hits
            .iter()
            .filter_map(|id| {
                self.drawing
                    .shapes
                    .get(id)
                    .filter(|s| s.rect.contains(world))
                    .map(|s| (id, s))
            })
            .max_by_key(|(_, s)| s.depth);
        // Ports and routes inside a composite must remain pickable through its background.
        if let Some((id, s)) = shape
            && (!s.composite || !s.expanded || world[1] < s.rect.y + 55.)
        {
            return Some(id.clone());
        }
        if let Some(target) = hits.iter().find_map(|id| {
            self.drawing
                .annotations
                .get(id)
                .filter(|a| a.rect.contains(world))
                .map(|a| a.target.clone())
        }) {
            return Some(target);
        }
        if include_links
            && let Some(id) = hits.iter().find(|id| {
                self.links.get(*id).is_some_and(|(_, p)| {
                    p.windows(2).any(|s| {
                        wirecraft_core::viewport::distance_to_segment(world, s[0], s[1])
                            <= tolerance
                    }) || {
                        let (output, input) = wirecraft_core::drawing::port_glyphs(p);
                        [output, input].iter().any(|glyph| {
                            glyph
                                .iter()
                                .map(|p| Rect::from_points(*p, *p))
                                .reduce(Rect::union)
                                .is_some_and(|r| r.contains(world))
                        })
                    }
                })
            })
        {
            return Some(id.clone());
        }
        shape.map(|(id, _)| id.clone())
    }
    pub fn window_selection(&self, start: [f64; 2], end: [f64; 2]) -> BTreeSet<String> {
        let r = Rect::from_points(start, end);
        let crossing = end[0] < start[0];
        let matches = |other: Rect| {
            if crossing {
                r.intersects(other)
            } else {
                r.encloses(other)
            }
        };
        self.query(r)
            .into_iter()
            .filter_map(|id| {
                if let Some(n) = self.nodes.get(&id) {
                    return matches(n.2).then_some(id);
                }
                if let Some(a) = self.drawing.annotations.get(&id) {
                    return matches(a.rect).then(|| a.target.clone());
                }
                self.links
                    .get(&id)
                    .filter(|(_, p)| {
                        if crossing {
                            p.windows(2).any(|s| r.intersects_segment(s[0], s[1]))
                        } else {
                            p.iter().all(|p| r.contains(*p))
                        }
                    })
                    .map(|_| id)
            })
            .collect()
    }
    fn project_selection(&self, selected: &BTreeSet<String>) -> BTreeSet<String> {
        selected
            .iter()
            .map(|id| self.drawing.visible_ancestor.get(id).unwrap_or(id).clone())
            .collect()
    }
    pub fn selection_bounds(
        &self,
        selected: &BTreeSet<String>,
        moving: &BTreeSet<String>,
        delta: [f64; 2],
    ) -> Option<Rect> {
        let projected = self.project_selection(selected);
        let selected = &projected;
        let mut rects = Vec::new();
        if selected.contains(&self.parent) {
            rects.extend(self.nodes.values().map(|n| n.2));
        }
        for id in selected {
            if let Some(n) = self.nodes.get(id) {
                let mut r = n.2;
                if moving.contains(id) {
                    r.x += delta[0];
                    r.y += delta[1];
                }
                rects.push(r);
            }
            if let Some(edge) = self.drawing.edges.get(id) {
                let points = self.preview_route(edge, moving, delta);
                let (output, input) = wirecraft_core::drawing::port_glyphs(&points);
                rects.extend(points.into_iter().chain(output).chain(input).map(|p| Rect {
                    x: p[0],
                    y: p[1],
                    width: 0.,
                    height: 0.,
                }));
            }
        }
        for annotation in self
            .drawing
            .annotations
            .values()
            .filter(|a| selected.contains(&a.target))
        {
            let mut r = annotation.rect;
            if moving.contains(&annotation.owner) {
                r.x += delta[0];
                r.y += delta[1];
            }
            rects.push(r);
        }
        rects.into_iter().reduce(Rect::union)
    }
    fn preview_route(
        &self,
        edge: &wirecraft_core::drawing::Edge,
        moving: &BTreeSet<String>,
        delta: [f64; 2],
    ) -> Vec<[f64; 2]> {
        let mut points = edge.points.clone();
        if points.len() < 2 {
            return points;
        }
        // The cached scene is already routed at its own drag offset. Preserve that
        // obstacle-aware path when no newer pointer delta needs a cheap preview.
        if delta == [0., 0.] {
            return points;
        }
        if moving.contains(&edge.source) && moving.contains(&edge.target) {
            for p in &mut points {
                p[0] += delta[0];
                p[1] += delta[1];
            }
        } else if moving.contains(&edge.source) || moving.contains(&edge.target) {
            let mut a = points[0];
            let mut b = *points.last().unwrap();
            if moving.contains(&edge.source) {
                a[0] += delta[0];
                a[1] += delta[1];
            }
            if moving.contains(&edge.target) {
                b[0] += delta[0];
                b[1] += delta[1];
            }
            let mid = (a[0] + b[0]) / 2.;
            points = vec![a, [mid, a[1]], [mid, b[1]], b];
        }
        points
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
    fn visible_for(
        &self,
        viewport: Rect,
        moving: &BTreeSet<String>,
        delta: [f64; 2],
    ) -> BTreeSet<String> {
        let mut visible = self.query(viewport).into_iter().collect::<BTreeSet<_>>();
        let mut edges = BTreeSet::new();
        for id in moving {
            if let Some(n) = self.nodes.get(id) {
                let translated = Rect {
                    x: n.2.x + delta[0],
                    y: n.2.y + delta[1],
                    ..n.2
                };
                if viewport.intersects(translated) {
                    visible.insert(id.clone());
                } else {
                    visible.remove(id);
                }
            }
            if let Some(incident) = self.incident.get(id) {
                edges.extend(incident.iter());
            }
        }
        for id in edges {
            if self
                .preview_route(&self.drawing.edges[id], moving, delta)
                .windows(2)
                .any(|p| viewport.intersects_segment(p[0], p[1]))
            {
                visible.insert(id.clone());
            } else {
                visible.remove(id);
            }
        }
        if !moving.is_empty() {
            for (id, a) in &self.drawing.annotations {
                if moving.contains(&a.owner) {
                    let r = Rect {
                        x: a.rect.x + delta[0],
                        y: a.rect.y + delta[1],
                        ..a.rect
                    };
                    if viewport.intersects(r) {
                        visible.insert(id.clone());
                    } else {
                        visible.remove(id);
                    }
                }
            }
        }
        visible
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
        errors: &BTreeSet<String>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let projected = self.project_selection(selected);
        let selected = &projected;
        window.paint_quad(fill(bounds, rgb(self.color(0x10161e))));
        let width = f64::from(f32::from(bounds.size.width));
        let height = f64::from(f32::from(bounds.size.height));
        let origin = view.world([0., 0.]);
        let viewport_rect = Rect {
            x: origin[0] - 200.,
            y: origin[1] - 100.,
            width: width / view.zoom + 400.,
            height: height / view.zoom + 200.,
        };
        let visible = self.visible_for(viewport_rect, moving, drag);
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
                self.paint_shape(
                    id,
                    shape,
                    view,
                    selected,
                    moving,
                    drag,
                    errors.contains(*id),
                    &screen,
                    window,
                    cx,
                );
            }
        }
        let traces = visible
            .iter()
            .filter(|id| self.links.contains_key(*id))
            .map(|id| {
                let edge = &self.drawing.edges[id];
                let points = self.preview_route(edge, moving, drag);
                let trace = wirecraft_core::drawing::trace_geometry(&points, &edge.crossings);
                (id.clone(), (points, trace))
            })
            .collect::<BTreeMap<_, _>>();
        // Backside connectors precede every foreground trace: the crossing
        // conductor stays continuous, without a background-colored erase mask.
        for (_, trace) in traces.values() {
            let mut path = PathBuilder::stroke(px(1.2));
            for via in &trace.vias {
                for (i, p) in via.backside.iter().enumerate() {
                    if i == 0 {
                        path.move_to(screen(*p));
                    } else {
                        path.line_to(screen(*p));
                    }
                }
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, rgba(0x7b858d88));
            }
        }
        for id in &visible {
            if self.links.contains_key(id) {
                let edge = &self.drawing.edges[id];
                let (points, trace) = &traces[id];
                let mut builder =
                    PathBuilder::stroke(px(if selected.contains(id) { 3. } else { 1.5 }));
                for fragment in &trace.fragments {
                    if edge.dashed {
                        for segment in fragment.windows(2) {
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
                        for (i, p) in fragment.iter().enumerate() {
                            if i == 0 {
                                builder.move_to(screen(*p));
                            } else {
                                builder.line_to(screen(*p));
                            }
                        }
                    }
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
                let color = if selected.contains(id) {
                    self.appearance.color("selection", 0x70d5c6)
                } else {
                    self.appearance.color(edge.color_key, 0xd0d5db)
                };
                for via in &trace.vias {
                    for p in [via.before, via.after] {
                        let p = screen(p);
                        let radius = px((via.radius * view.zoom) as f32);
                        window.paint_quad(quad(
                            Bounds::new(p - point(radius, radius), size(radius * 2., radius * 2.)),
                            radius,
                            rgba(0x00000000),
                            px(1.2),
                            rgb(color),
                            BorderStyle::Solid,
                        ));
                    }
                }
                let arrow = wirecraft_core::drawing::flow_arrow(&trace.fragments);
                let mut polygon = PathBuilder::fill();
                for (i, p) in arrow.iter().enumerate() {
                    if i == 0 {
                        polygon.move_to(screen(*p));
                    } else {
                        polygon.line_to(screen(*p));
                    }
                }
                if !arrow.is_empty() {
                    polygon.close();
                    if let Ok(path) = polygon.build() {
                        window.paint_path(path, rgb(color));
                    }
                }
                let (output, input) = wirecraft_core::drawing::port_glyphs(points);
                let mut symbol = PathBuilder::stroke(px(1.2));
                for (i, p) in output.iter().enumerate() {
                    if i == 0 {
                        symbol.move_to(screen(*p));
                    } else {
                        symbol.line_to(screen(*p));
                    }
                }
                if !output.is_empty() {
                    symbol.close();
                }
                if let Ok(path) = symbol.build() {
                    window.paint_path(path, rgb(color));
                }
                let mut symbol = PathBuilder::fill();
                for (i, p) in input.iter().enumerate() {
                    if i == 0 {
                        symbol.move_to(screen(*p));
                    } else {
                        symbol.line_to(screen(*p));
                    }
                }
                if !input.is_empty() {
                    symbol.close();
                }
                if let Ok(path) = symbol.build() {
                    window.paint_path(path, rgb(color));
                }
                if view.zoom >= 0.35 && points.len() >= 2 {
                    for (index, adjacent, label) in [
                        (0, 1, &edge.source_label),
                        (points.len() - 1, points.len() - 2, &edge.target_label),
                    ] {
                        let p = points[index];
                        let q = points[adjacent];
                        self.port_label(
                            label,
                            screen(p),
                            q[0] >= p[0],
                            (9. * view.zoom as f32).max(5.),
                            window,
                            cx,
                        );
                    }
                }
            }
        }
        for (id, shape) in &ordered {
            if !shape.composite || !shape.expanded {
                self.paint_shape(
                    id,
                    shape,
                    view,
                    selected,
                    moving,
                    drag,
                    errors.contains(*id),
                    &screen,
                    window,
                    cx,
                );
            }
        }
        for port in &self.drawing.boundary_ports {
            if !visible.contains(&port.owner) && !visible.contains(&port.edge) {
                continue;
            }
            let offset = if moving.contains(&port.owner) {
                drag
            } else {
                [0., 0.]
            };
            let p = screen([port.point[0] + offset[0], port.point[1] + offset[1]]);
            let radius = px((5. * view.zoom) as f32);
            window.paint_quad(quad(
                Bounds::new(p - point(radius, radius), size(radius * 2., radius * 2.)),
                radius,
                rgb(self.color(0x70bdb3)),
                px(0.),
                rgb(self.color(0x70bdb3)),
                BorderStyle::Solid,
            ));
            if view.zoom >= 0.35 {
                let owner = self.drawing.shapes[&port.owner].rect;
                self.port_label(
                    &port.label,
                    p,
                    port.point[0] > owner.x + owner.width / 2.,
                    (9. * view.zoom as f32).max(5.),
                    window,
                    cx,
                );
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
                let stroke = if selected {
                    self.color(0x70d5c6)
                } else {
                    self.appearance.color(
                        annotation.color_key,
                        if annotation.color_key == "typeStroke" {
                            0x45baa8
                        } else {
                            0xe3b664
                        },
                    )
                };
                let mut connector = PathBuilder::stroke(px(if selected { 2. } else { 1. }));
                let anchor = if annotation.flag {
                    [r.x, r.y + 18.]
                } else {
                    [owner.x + owner.width, r.y + r.height / 2.]
                };
                if annotation.flag {
                    connector.move_to(screen([r.x, owner.y]));
                    connector.line_to(screen(anchor));
                    connector.line_to(screen([r.x + r.width, anchor[1]]));
                } else {
                    connector.move_to(screen(anchor));
                    connector.line_to(screen([r.x, anchor[1]]));
                    window.paint_quad(quad(
                        rect,
                        px(1.),
                        rgb(self.color(if annotation.color_key == "typeStroke" {
                            0x173b3a
                        } else {
                            0x202c38
                        })),
                        px(if selected { 2. } else { 1. }),
                        rgb(stroke),
                        BorderStyle::Solid,
                    ));
                }
                if let Ok(path) = connector.build() {
                    window.paint_path(path, rgb(stroke));
                }
                let dot = screen(anchor);
                window.paint_quad(quad(
                    Bounds::new(dot - point(px(3.), px(3.)), size(px(6.), px(6.))),
                    px(3.),
                    rgb(stroke),
                    px(0.),
                    rgb(stroke),
                    BorderStyle::Solid,
                ));
                if view.zoom >= 0.3 {
                    self.label(
                        &annotation.label,
                        rect.origin + point(px(4. * view.zoom as f32), px(2. * view.zoom as f32)),
                        (10. * view.zoom as f32).max(5.),
                        stroke,
                        window,
                        cx,
                    );
                }
            }
        }
        if let Some(r) = self.selection_bounds(selected, moving, drag) {
            let origin = screen([r.x, r.y]) - point(px(5.), px(5.));
            dashed_outline(
                Bounds::new(
                    origin,
                    size(
                        px((r.width * view.zoom) as f32 + 10.),
                        px((r.height * view.zoom) as f32 + 10.),
                    ),
                ),
                1.,
                6.,
                5.,
                rgb(self.color(0x70d5c6)),
                window,
            );
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
        code_error: bool,
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
        if let Some(exterior) = &shape.exterior {
            self.paint_exterior(exterior, shape, rect, active, view, window, cx);
            return;
        }
        let (fill_color, stroke) = match shape.role.as_str() {
            "Reference" => (0x2c2541, 0xb08fe8),
            "Type" => (0x173b3a, 0x45baa8),
            "ServiceLibrary" => (0x292e43, 0x8893e4),
            "CompilerTemplate" => (0x4c3b17, 0xf0ad36),
            "ErrorHandler" | "CompositeErrorHandler" => (0x432a2a, 0xe27b7b),
            "Test" | "TestSuite" => (0x273d2c, 0x6ac284),
            _ => (0x202c38, 0x8196a9),
        };
        let outline = rgb(self.color(if active {
            0x70d5c6
        } else if shape.io.is_some() {
            IO_COLOR
        } else {
            stroke
        }));
        let width: f32 = if active { 2. } else { 1. };
        // GPUI's dashed border scales its dashes with the border width, which at 1px
        // reads as a solid line; dashed contours are stroked with explicit lengths.
        let dashes = if shape.io.is_some() {
            Some((width.max(2.), 14., 6.))
        } else if shape.composite && shape.expanded {
            Some((width, 9., 7.))
        } else {
            None
        };
        window.paint_quad(quad(
            rect,
            px(0.),
            rgb(self.color(fill_color)),
            px(if dashes.is_some() { 0. } else { width }),
            outline,
            BorderStyle::Solid,
        ));
        if let Some((width, dash, gap)) = dashes {
            dashed_outline(rect, width, dash, gap, outline, window);
        }
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
        // Only expanded composites keep handles: one per corner, editing its two sides.
        if active && shape.composite && shape.expanded {
            for corner in [
                rect.origin,
                rect.top_right() - point(px(6.), px(0.)),
                rect.bottom_right() - point(px(6.), px(6.)),
                rect.bottom_left() - point(px(0.), px(6.)),
            ] {
                window.paint_quad(fill(
                    Bounds::new(corner, size(px(6.), px(6.))),
                    rgb(self.color(0x70d5c6)),
                ));
            }
        }
        if let Some(badge) = shape.io {
            self.io_badge(badge, rect, view, window, cx);
        }
        if shape.diagnostics > 0 || code_error {
            let diameter = (14. * view.zoom as f32).max(8.);
            let origin = rect.top_right() - point(px(diameter + 4.), px(-4.));
            window.paint_quad(quad(
                Bounds::new(origin, size(px(diameter), px(diameter))),
                px(diameter / 2.),
                rgb(0xd94f54),
                px(0.),
                rgb(0xd94f54),
                BorderStyle::Solid,
            ));
            self.label(
                "!",
                origin + point(px(diameter * 0.36), px(-1.)),
                diameter * 0.8,
                0xffffff,
                window,
                cx,
            );
        }
    }
    /// An Exterior: a heavy circle with a chevron along the flow and its derived name
    /// beside it, away from the link. The link's port cap sits on the rim.
    #[allow(clippy::too_many_arguments)]
    fn paint_exterior(
        &self,
        exterior: &wirecraft_core::drawing::ExteriorView,
        shape: &wirecraft_core::drawing::Shape,
        rect: Bounds<Pixels>,
        active: bool,
        view: Viewport,
        window: &mut Window,
        cx: &mut App,
    ) {
        let diameter = rect.size.height;
        let zoom = view.zoom as f32;
        window.paint_quad(quad(
            rect,
            diameter / 2.,
            rgb(self.color(0x16303f)),
            px(if active { 4. } else { 2.5 } * zoom.clamp(0.5, 1.5)),
            rgb(self.color(if active { 0x70d5c6 } else { 0x4fb3d9 })),
            BorderStyle::Solid,
        ));
        // Output: packets arrive from the link side; input: they leave towards it.
        let towards_right = exterior.output != exterior.link_right;
        let sign = if towards_right { 1. } else { -1. };
        let center = rect.center();
        let arm = diameter * 0.16;
        let mut chevron = PathBuilder::stroke(px((3. * zoom).max(1.5)));
        chevron.move_to(center + point(-arm * 0.5 * sign, -arm));
        chevron.line_to(center + point(arm * 0.5 * sign, px(0.)));
        chevron.line_to(center + point(-arm * 0.5 * sign, arm));
        if let Ok(path) = chevron.build() {
            window.paint_path(path, rgb(self.color(0x9fd8f0)));
        }
        if view.zoom >= 0.2 {
            let font_size = (13. * zoom).max(7.);
            let label = if exterior.type_name.is_empty() {
                shape.name.clone()
            } else {
                format!("{}: {}", shape.name, exterior.type_name)
            };
            let side = if exterior.link_right {
                rect.left()
            } else {
                rect.right()
            };
            self.port_label(
                &label,
                point(side, center.y + px((font_size + 5.) / 2.)),
                !exterior.link_right,
                font_size,
                window,
                cx,
            );
        }
        if shape.diagnostics > 0 {
            let badge = px((14. * zoom).max(8.));
            window.paint_quad(quad(
                Bounds::new(rect.top_right() - point(badge, px(0.)), size(badge, badge)),
                badge / 2.,
                rgb(0xd94f54),
                px(0.),
                rgb(0xd94f54),
                BorderStyle::Solid,
            ));
        }
    }
    /// A pill straddling the top edge, left of the title, naming the IO role.
    fn io_badge(
        &self,
        badge: &str,
        rect: Bounds<Pixels>,
        view: Viewport,
        window: &mut Window,
        cx: &mut App,
    ) {
        if view.zoom < 0.2 {
            return;
        }
        let font_size = (10. * view.zoom as f32).max(7.);
        let height = font_size + 6.;
        let run = TextRun {
            len: badge.len(),
            font: font(self.appearance.designer_font.clone()).bold(),
            color: rgb(0x1f1606).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line =
            window
                .text_system()
                .shape_line(badge.to_string().into(), px(font_size), &[run], None);
        let origin = rect.origin + point(px(10. * view.zoom as f32), px(-height / 2.));
        window.paint_quad(quad(
            Bounds::new(origin, size(line.width + px(10.), px(height))),
            px(height / 2.),
            rgb(self.color(IO_COLOR)),
            px(0.),
            rgb(self.color(IO_COLOR)),
            BorderStyle::Solid,
        ));
        let _ = line.paint(origin + point(px(5.), px(0.)), px(height), window, cx);
    }
    fn port_label(
        &self,
        text: &str,
        anchor: Point<Pixels>,
        right: bool,
        font_size: f32,
        window: &mut Window,
        cx: &mut App,
    ) {
        let split = text.rfind(':').unwrap_or(text.len());
        let name_len = if split < text.len() { split + 1 } else { split };
        let run = |len, color| TextRun {
            len,
            font: font(self.appearance.designer_font.clone()),
            color: rgb(self.color(color)).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = [
            run(name_len, 0xd0d5db),
            run(text.len() - name_len, 0x58c6a4),
        ];
        let line =
            window
                .text_system()
                .shape_line(text.to_string().into(), px(font_size), &runs, None);
        let x = if right { px(9.) } else { -line.width - px(9.) };
        let _ = line.paint(
            anchor + point(x, px(-font_size - 5.)),
            px(font_size + 5.),
            window,
            cx,
        );
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

#[cfg(test)]
mod tests {
    use super::Scene;
    use serde_json::json;
    use std::collections::BTreeSet;
    use wirecraft_core::{Document, NodeKind};
    fn fixture() -> (Scene, Document, String, String, String) {
        let mut d = Document::new("Canvas");
        let a = d.create("root", "A", NodeKind::Processor).unwrap();
        let b = d.create("root", "B", NodeKind::Processor).unwrap();
        d.patch(
            &a,
            &json!({"layout":{"x":100,"y":100,"width":100,"height":80}}),
        )
        .unwrap();
        d.patch(
            &b,
            &json!({"layout":{"x":400,"y":100,"width":100,"height":80}}),
        )
        .unwrap();
        let wire = d
            .create_link(
                "wire",
                wirecraft_core::Link {
                    source_node_id: a.clone(),
                    target_node_id: b.clone(),
                    ..Default::default()
                },
            )
            .unwrap();
        (Scene::new(&d, "root"), d, a, b, wire)
    }
    #[test]
    fn directional_windows_select_whole_objects_or_crossed_nodes_and_routes() {
        let (s, _, a, _, wire) = fixture();
        assert_eq!(
            s.window_selection([90., 90.], [190., 190.]),
            BTreeSet::new()
        );
        assert_eq!(
            s.window_selection([190., 190.], [90., 90.]),
            BTreeSet::from([a.clone()])
        );
        assert_eq!(
            s.window_selection([90., 90.], [230., 190.]),
            BTreeSet::from([a])
        );
        assert_eq!(
            s.window_selection([290., 130.], [310., 150.]),
            BTreeSet::new()
        );
        assert_eq!(
            s.window_selection([310., 150.], [290., 130.]),
            BTreeSet::from([wire])
        );
    }
    #[test]
    fn pick_and_selection_bounds_follow_all_moving_nodes_and_selected_links() {
        let (s, _, a, b, wire) = fixture();
        assert_eq!(s.pick([110., 120.], 5., true), Some(a.clone()));
        assert_eq!(s.pick([300., 140.], 5., true), Some(wire.clone()));
        let selection = BTreeSet::from([a, b]);
        let r = s
            .selection_bounds(&selection, &selection, [50., 25.])
            .unwrap();
        assert_eq!([r.x, r.y, r.width, r.height], [150., 125., 420., 73.]);
        let r = s
            .selection_bounds(&BTreeSet::from([wire.clone()]), &selection, [50., 25.])
            .unwrap();
        for (actual, expected) in [r.x, r.y, r.width, r.height]
            .into_iter()
            .zip([270.2, 156.7, 179.8, 9.6])
        {
            assert!((actual - expected).abs() < 1e-6);
        }
        assert_eq!(s.pick([230., 136.5], 1., true), Some(wire));
    }
    #[test]
    fn links_inside_composite_remain_pickable() {
        let (_, mut d, a, b, wire) = fixture();
        let group = d.create("root", "Group", NodeKind::Group).unwrap();
        d.patch(
            &group,
            &json!({"layout":{"x":50,"y":50,"width":600,"height":300,"isExpanded":true}}),
        )
        .unwrap();
        d.move_node(&a, Some(&group)).unwrap();
        d.move_node(&b, Some(&group)).unwrap();
        let s = Scene::new(&d, "root");
        assert_eq!(s.pick([300., 140.], 5., true), Some(wire));
        assert_eq!(s.pick([60., 60.], 5., true), Some(group));
    }
    #[test]
    fn hidden_selected_descendant_projects_to_visible_composite_contour() {
        let (_, mut d, a, _, _) = fixture();
        let group = d.create("root", "Group", NodeKind::Group).unwrap();
        d.patch(&group,&json!({"layout":{"x":60,"y":220,"closedWidth":180,"closedHeight":70,"isExpanded":false}})).unwrap();
        d.move_node(&a, Some(&group)).unwrap();
        let s = Scene::new(&d, "root");
        let selection = BTreeSet::from([a]);
        assert_eq!(
            s.project_selection(&selection),
            BTreeSet::from([group.clone()])
        );
        let r = s
            .selection_bounds(&selection, &BTreeSet::new(), [0., 0.])
            .unwrap();
        let contour = s.drawing.shapes[&group].rect;
        assert_eq!([r.x, r.y], [60., 220.]);
        assert_eq!([r.width, r.height], [contour.width, contour.height]);
    }
    #[test]
    fn moving_primitives_are_culled_at_their_preview_positions() {
        let (s, _, a, b, wire) = fixture();
        let moving = BTreeSet::from([a.clone(), b]);
        let old = wirecraft_core::viewport::Rect::from_points([90., 90.], [240., 190.]);
        assert!(s.visible_for(old, &moving, [10000., 0.]).is_empty());
        let translated = wirecraft_core::viewport::Rect::from_points([10090., 90.], [10240., 190.]);
        assert_eq!(
            s.visible_for(translated, &moving, [10000., 0.]),
            BTreeSet::from([a, wire])
        );
        let empty = wirecraft_core::viewport::Rect::from_points([10250., 90.], [10300., 120.]);
        assert!(s.visible_for(empty, &moving, [10000., 0.]).is_empty());
    }
    #[test]
    fn completed_reroute_retains_its_obstacle_detour() {
        let (_, mut d, a, b, wire) = fixture();
        // The auto-sized obstacle is 120 wide; leave a detour corridor before B.
        d.patch(&b, &json!({"layout":{"x":600}})).unwrap();
        let obstacle = d.create("root", "Obstacle", NodeKind::Processor).unwrap();
        d.patch(&obstacle, &json!({"layout":{"x":320,"y":90}}))
            .unwrap();
        let s = Scene::new(&d, "root");
        let edge = &s.drawing.edges[&wire];
        let obstacle = s.drawing.shapes[&obstacle].rect;
        assert!(
            edge.points
                .windows(2)
                .all(|p| !obstacle.intersects_segment(p[0], p[1])),
            "fixture must route around the obstacle"
        );
        assert_eq!(
            s.preview_route(edge, &BTreeSet::from([a]), [0., 0.]),
            edge.points
        );
    }
}
