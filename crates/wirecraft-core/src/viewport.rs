//! Canvas geometry, independent from a window or rendering API.
use crate::Document;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}
impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 40.,
            y: 40.,
            zoom: 1.,
        }
    }
}
impl Viewport {
    pub fn screen(&self, p: [f64; 2]) -> [f64; 2] {
        [p[0] * self.zoom + self.x, p[1] * self.zoom + self.y]
    }
    pub fn world(&self, p: [f64; 2]) -> [f64; 2] {
        [(p[0] - self.x) / self.zoom, (p[1] - self.y) / self.zoom]
    }
    pub fn zoom_at(&mut self, p: [f64; 2], factor: f64) {
        let w = self.world(p);
        self.zoom = (self.zoom * factor).clamp(0.1, 4.);
        self.x = p[0] - w[0] * self.zoom;
        self.y = p[1] - w[1] * self.zoom;
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
/// Selection modifiers are idempotent: adding an existing hit never toggles it off.
#[derive(Clone, Copy, Debug, Default)]
pub enum SelectionOperation {
    #[default]
    Replace,
    Add,
    Remove,
}
impl SelectionOperation {
    pub fn apply(self, base: &BTreeSet<String>, hits: BTreeSet<String>) -> BTreeSet<String> {
        match self {
            Self::Replace => hits,
            Self::Add => base.union(&hits).cloned().collect(),
            Self::Remove => base.difference(&hits).cloned().collect(),
        }
    }
}
impl Rect {
    pub fn from_points(a: [f64; 2], b: [f64; 2]) -> Self {
        Self {
            x: a[0].min(b[0]),
            y: a[1].min(b[1]),
            width: (a[0] - b[0]).abs(),
            height: (a[1] - b[1]).abs(),
        }
    }
    pub fn encloses(self, other: Self) -> bool {
        self.contains([other.x, other.y])
            && self.contains([other.x + other.width, other.y + other.height])
    }
    pub fn union(self, other: Self) -> Self {
        Self::from_points(
            [self.x.min(other.x), self.y.min(other.y)],
            [
                (self.x + self.width).max(other.x + other.width),
                (self.y + self.height).max(other.y + other.height),
            ],
        )
    }
    /// Liang–Barsky clipping, including touching edges and degenerate segments.
    pub fn intersects_segment(self, a: [f64; 2], b: [f64; 2]) -> bool {
        let delta = [b[0] - a[0], b[1] - a[1]];
        let mut lo: f64 = 0.;
        let mut hi: f64 = 1.;
        for (p, q) in [
            (-delta[0], a[0] - self.x),
            (delta[0], self.x + self.width - a[0]),
            (-delta[1], a[1] - self.y),
            (delta[1], self.y + self.height - a[1]),
        ] {
            if p == 0. {
                if q < 0. {
                    return false;
                }
            } else {
                let t = q / p;
                if p < 0. {
                    lo = lo.max(t);
                } else {
                    hi = hi.min(t);
                }
                if lo > hi {
                    return false;
                }
            }
        }
        true
    }
    pub fn intersects(self, other: Self) -> bool {
        self.x <= other.x + other.width
            && self.x + self.width >= other.x
            && self.y <= other.y + other.height
            && self.y + self.height >= other.y
    }
    pub fn contains(self, p: [f64; 2]) -> bool {
        p[0] >= self.x
            && p[0] <= self.x + self.width
            && p[1] >= self.y
            && p[1] <= self.y + self.height
    }
}
/// Uniform grid index. Oversized primitives are indexed separately to bound cell expansion.
#[derive(Default)]
pub struct SceneIndex {
    cells: BTreeMap<(i32, i32), Vec<String>>,
    bounds: BTreeMap<String, Rect>,
    large: Vec<String>,
}
impl SceneIndex {
    pub fn from_document(d: &Document, parent: &str) -> Self {
        let mut index = Self::default();
        if let Ok(p) = d.node(parent) {
            for id in &p.children {
                if let Ok(n) = d.node(id)
                    && !n.is_link()
                {
                    let r = &n.layout;
                    index.insert(
                        id.clone(),
                        Rect {
                            x: r.x,
                            y: r.y,
                            width: r.width,
                            height: r.height,
                        },
                    );
                }
            }
        }
        index
    }
    pub fn insert(&mut self, id: String, r: Rect) {
        let (x0, y0, x1, y1) = cells(r);
        let count = (i64::from(x1) - i64::from(x0) + 1) * (i64::from(y1) - i64::from(y0) + 1);
        if count > 256 {
            self.large.push(id.clone());
        } else {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    self.cells.entry((x, y)).or_default().push(id.clone());
                }
            }
        }
        self.bounds.insert(id, r);
    }
    pub fn query(&self, r: Rect) -> Vec<String> {
        let (x0, y0, x1, y1) = cells(r);
        let mut ids: BTreeSet<&String> = self.large.iter().collect();
        if (i64::from(x1) - i64::from(x0) + 1) * (i64::from(y1) - i64::from(y0) + 1) > 100_000 {
            ids.extend(self.bounds.keys());
        } else {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    if let Some(cell) = self.cells.get(&(x, y)) {
                        ids.extend(cell);
                    }
                }
            }
        }
        ids.into_iter()
            .filter(|id| self.bounds[*id].intersects(r))
            .cloned()
            .collect()
    }
}
fn cells(r: Rect) -> (i32, i32, i32, i32) {
    (
        (r.x / 256.).floor() as i32,
        (r.y / 256.).floor() as i32,
        ((r.x + r.width) / 256.).floor() as i32,
        ((r.y + r.height) / 256.).floor() as i32,
    )
}
/// Cached by the client for a document revision. Endpoint stubs and orthogonal segments.
pub fn route(a: Rect, b: Rect) -> Vec<[f64; 2]> {
    let start = [a.x + a.width, a.y + a.height / 2.];
    let end = [b.x, b.y + b.height / 2.];
    if (a.x - b.x).abs() < f64::EPSILON && (a.y - b.y).abs() < f64::EPSILON {
        return vec![
            start,
            [start[0] + 30., start[1]],
            [start[0] + 30., a.y - 30.],
            [a.x - 30., a.y - 30.],
            [a.x - 30., end[1]],
            end,
        ];
    }
    let mid = if end[0] > start[0] + 40. {
        (start[0] + end[0]) / 2.
    } else {
        start[0] + 30.
    };
    vec![start, [mid, start[1]], [mid, end[1]], end]
}

pub fn distance_to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = dx * dx + dy * dy;
    let t = if length == 0. {
        0.
    } else {
        ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / length
    }
    .clamp(0., 1.);
    ((p[0] - a[0] - t * dx).powi(2) + (p[1] - a[1] - t * dy).powi(2)).sqrt()
}

/// Immutable segment BVH for routed links. A route's empty bounding-box interior
/// must not enqueue it for painting or hit testing. Built once per graph revision.
#[derive(Default)]
pub struct RouteIndex {
    ids: Vec<String>,
    segments: Vec<RouteSegment>,
    nodes: Vec<RouteBranch>,
}
struct RouteSegment {
    route: usize,
    bounds: Rect,
}
struct RouteBranch {
    bounds: Rect,
    children: RouteChildren,
}
enum RouteChildren {
    Leaf { start: usize, end: usize },
    Branch { left: usize, right: usize },
}
impl RouteIndex {
    pub fn new<'a>(routes: impl IntoIterator<Item = (&'a str, &'a [[f64; 2]])>) -> Self {
        let mut index = Self::default();
        for (id, points) in routes {
            let route = index.ids.len();
            index.ids.push(id.into());
            for pair in points.windows(2) {
                index.segments.push(RouteSegment {
                    route,
                    bounds: Rect {
                        x: pair[0][0].min(pair[1][0]) - 6.,
                        y: pair[0][1].min(pair[1][1]) - 24.,
                        width: (pair[0][0] - pair[1][0]).abs() + 12.,
                        height: (pair[0][1] - pair[1][1]).abs() + 48.,
                    },
                });
            }
        }
        if !index.segments.is_empty() {
            Self::build(&mut index.segments, 0, &mut index.nodes);
        }
        index
    }
    fn build(segments: &mut [RouteSegment], start: usize, nodes: &mut Vec<RouteBranch>) -> usize {
        let left = segments
            .iter()
            .map(|s| s.bounds.x)
            .fold(f64::INFINITY, f64::min);
        let top = segments
            .iter()
            .map(|s| s.bounds.y)
            .fold(f64::INFINITY, f64::min);
        let right = segments
            .iter()
            .map(|s| s.bounds.x + s.bounds.width)
            .fold(f64::NEG_INFINITY, f64::max);
        let bottom = segments
            .iter()
            .map(|s| s.bounds.y + s.bounds.height)
            .fold(f64::NEG_INFINITY, f64::max);
        let bounds = Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        };
        let node = nodes.len();
        nodes.push(RouteBranch {
            bounds,
            children: RouteChildren::Leaf {
                start,
                end: start + segments.len(),
            },
        });
        if segments.len() > 8 {
            let horizontal = bounds.width >= bounds.height;
            let mid = segments.len() / 2;
            let center = |s: &RouteSegment| {
                if horizontal {
                    s.bounds.x + s.bounds.width / 2.
                } else {
                    s.bounds.y + s.bounds.height / 2.
                }
            };
            segments.select_nth_unstable_by(mid, |a, b| center(a).total_cmp(&center(b)));
            let (a, b) = segments.split_at_mut(mid);
            let left = Self::build(a, start, nodes);
            let right = Self::build(b, start + mid, nodes);
            nodes[node].children = RouteChildren::Branch { left, right };
        }
        node
    }
    pub fn query(&self, rect: Rect) -> Vec<String> {
        if self.nodes.is_empty() {
            return Vec::new();
        }
        let mut pending = vec![0];
        let mut routes = BTreeSet::new();
        while let Some(index) = pending.pop() {
            let node = &self.nodes[index];
            if !node.bounds.intersects(rect) {
                continue;
            }
            match node.children {
                RouteChildren::Leaf { start, end } => {
                    for segment in &self.segments[start..end] {
                        if segment.bounds.intersects(rect) {
                            routes.insert(segment.route);
                        }
                    }
                }
                RouteChildren::Branch { left, right } => {
                    pending.push(left);
                    pending.push(right);
                }
            }
        }
        routes.into_iter().map(|id| self.ids[id].clone()).collect()
    }
}
