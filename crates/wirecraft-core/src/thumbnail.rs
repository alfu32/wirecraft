//! A small picture of a design for the home screen, from the shared drawing projection.
use crate::{Document, drawing::Drawing};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThumbnailShape {
    Entity,
    Type,
    Composite,
    Port,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThumbnailBox {
    pub shape: ThumbnailShape,
    /// x, y, width, height from the design's top-left corner.
    pub rect: [f64; 4],
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Thumbnail {
    pub width: f64,
    pub height: f64,
    /// Outermost first, so that children paint over the composites holding them.
    pub boxes: Vec<ThumbnailBox>,
    pub links: Vec<Vec<[f64; 2]>>,
}
/// The design as its root view draws it, with collapsed composites and routed links.
pub fn of(d: &Document) -> Thumbnail {
    let drawing = Drawing::new(d, &d.root_node_id);
    let mut shapes = drawing.shapes.values().collect::<Vec<_>>();
    shapes.sort_by_key(|s| s.depth);
    let links = drawing
        .edges
        .values()
        .filter(|e| !e.annotation_only && e.points.len() > 1)
        .map(|e| e.points.clone())
        .collect::<Vec<_>>();
    let points = shapes
        .iter()
        .flat_map(|s| {
            let r = s.rect;
            [[r.x, r.y], [r.x + r.width, r.y + r.height]]
        })
        .chain(links.iter().flatten().copied())
        .collect::<Vec<_>>();
    if points.is_empty() {
        return Thumbnail::default();
    }
    let bound = |axis: usize, start: f64, pick: fn(f64, f64) -> f64| {
        points.iter().map(|p| p[axis]).fold(start, pick)
    };
    let (left, top) = (
        bound(0, f64::INFINITY, f64::min),
        bound(1, f64::INFINITY, f64::min),
    );
    let (right, bottom) = (
        bound(0, f64::NEG_INFINITY, f64::max),
        bound(1, f64::NEG_INFINITY, f64::max),
    );
    Thumbnail {
        width: right - left,
        height: bottom - top,
        boxes: shapes
            .iter()
            .map(|s| ThumbnailBox {
                shape: if s.exterior.is_some() {
                    ThumbnailShape::Port
                } else if s.kind == crate::NodeKind::Type {
                    ThumbnailShape::Type
                } else if s.composite && s.expanded {
                    ThumbnailShape::Composite
                } else {
                    ThumbnailShape::Entity
                },
                rect: [s.rect.x - left, s.rect.y - top, s.rect.width, s.rect.height],
            })
            .collect(),
        links: links
            .into_iter()
            .map(|line| {
                line.into_iter()
                    .map(|p| [p[0] - left, p[1] - top])
                    .collect()
            })
            .collect(),
    }
}
