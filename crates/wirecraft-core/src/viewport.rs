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
impl Rect {
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
