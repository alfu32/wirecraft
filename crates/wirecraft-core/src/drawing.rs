//! Visible diagram projection shared by interactive canvas and portable exports.
//! Source layouts use absolute world coordinates, including composite children.
use crate::{
    Document, NodeKind,
    analysis::{Relation, relation, stereotype},
    viewport::Rect,
};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone)]
pub struct Shape {
    pub name: String,
    pub detail: String,
    pub kind: NodeKind,
    pub rect: Rect,
    pub depth: usize,
    pub composite: bool,
    pub expanded: bool,
    pub role: String,
    pub technology: String,
    pub fields: Vec<String>,
    pub diagnostics: usize,
}
#[derive(Clone)]
pub struct Edge {
    pub name: String,
    pub annotation_only: bool,
    pub route_error: Option<String>,
    pub source_label: String,
    pub target_label: String,
    pub points: Vec<[f64; 2]>,
    pub crossings: Vec<[f64; 2]>,
    pub relation: Option<Relation>,
    pub color_key: &'static str,
    pub dashed: bool,
    pub source: String,
    pub target: String,
}
/// Technical drawing port symbols follow the actual endpoint directions.
pub fn port_glyphs(points: &[[f64; 2]]) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    if points.len() < 2 {
        return (vec![], vec![]);
    }
    let unit = |a: [f64; 2], b: [f64; 2]| {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = dx.hypot(dy).max(0.001);
        [dx / len, dy / len]
    };
    let a = points[0];
    let u = unit(a, points[1]);
    let b = *points.last().unwrap();
    let v = unit(points[points.len() - 2], b);
    // Exact 0.6-scaled JVM portCapPoints templates, expressed in each
    // endpoint's local direction. Outputs sit 20 units outside the box;
    // inputs stay at its boundary. The five-sided caps are not diamonds.
    let transform = |origin: [f64; 2], unit: [f64; 2], x: f64, y: f64| {
        [
            origin[0] + unit[0] * x - unit[1] * y,
            origin[1] + unit[1] * x + unit[0] * y,
        ]
    };
    (
        [
            (-19.8, -4.8),
            (-4.8, -4.8),
            (0., 0.),
            (-4.8, 4.8),
            (-19.8, 4.8),
        ]
        .into_iter()
        .map(|(x, y)| transform(a, u, x, y))
        .collect(),
        [
            (-19.8, -4.8),
            (-15., 0.),
            (-19.8, 4.8),
            (0., 4.8),
            (0., -4.8),
        ]
        .into_iter()
        .map(|(x, y)| transform(b, v, x, y))
        .collect(),
    )
}
#[derive(Clone)]
pub struct Annotation {
    pub owner: String,
    pub flag: bool,
    pub color_key: &'static str,
    pub target: String,
    pub label: String,
    pub rect: Rect,
}
#[derive(Clone)]
pub struct BoundaryPort {
    pub owner: String,
    pub edge: String,
    pub point: [f64; 2],
    pub label: String,
}
pub struct Drawing {
    pub boundary_ports: Vec<BoundaryPort>,
    /// Includes hidden descendants projected to their visible collapsed ancestor.
    pub visible_ancestor: BTreeMap<String, String>,
    pub annotations: BTreeMap<String, Annotation>,
    pub shapes: BTreeMap<String, Shape>,
    pub edges: BTreeMap<String, Edge>,
}
impl Drawing {
    pub fn new(d: &Document, parent: &str) -> Self {
        let mut shapes = BTreeMap::new();
        let mut visible_ancestor = BTreeMap::new();
        let mut stack = Vec::new();
        if let Some(p) = d.nodes.get(parent) {
            stack.extend(
                p.children
                    .iter()
                    .rev()
                    .map(|id| (id.clone(), 0, None::<String>)),
            );
        }
        let mut visited = BTreeSet::new();
        while let Some((id, depth, hidden)) = stack.pop() {
            if !visited.insert(id.clone()) {
                continue;
            }
            let Some(n) = d.nodes.get(&id) else {
                continue;
            };
            if n.is_link() {
                continue;
            }
            if let Some(ancestor) = hidden {
                visible_ancestor.insert(id.clone(), ancestor.clone());
                stack.extend(
                    n.children
                        .iter()
                        .rev()
                        .map(|id| (id.clone(), depth + 1, Some(ancestor.clone()))),
                );
                continue;
            }
            let composite = n.is_composite();
            let expanded = n.layout.is_expanded;
            let rect = Rect {
                x: n.layout.x,
                y: n.layout.y,
                width: if composite && !expanded {
                    n.layout.closed_width
                } else {
                    n.layout.width
                },
                height: if composite && !expanded {
                    n.layout.closed_height
                } else {
                    n.layout.height
                },
            };
            let fields = n
                .type_definition
                .as_ref()
                .and_then(|v| v["fields"].as_array())
                .into_iter()
                .flatten()
                .map(|f| {
                    format!(
                        "{}: {}",
                        f["name"].as_str().unwrap_or(""),
                        f["typeId"]
                            .as_str()
                            .or_else(|| f["typeName"].as_str())
                            .map(|id| d.nodes.get(id).map(|n| n.name.as_str()).unwrap_or(id))
                            .unwrap_or("?")
                    )
                })
                .collect();
            shapes.insert(
                id.clone(),
                Shape {
                    name: n.name.clone(),
                    detail: n.name_detail.clone(),
                    kind: n.kind,
                    rect,
                    depth,
                    composite,
                    expanded,
                    role: stereotype(d, n).into(),
                    technology: format!(
                        "{} · {}",
                        d.effective(&id, "languageId"),
                        d.effective(&id, "technologyId")
                    ),
                    fields,
                    diagnostics: n
                        .diagnostics
                        .iter()
                        .filter(|v| {
                            matches!(v["severity"].as_u64(), Some(1 | 2))
                                || v["severity"].as_str().is_some_and(|s| {
                                    s.eq_ignore_ascii_case("error")
                                        || s.eq_ignore_ascii_case("warning")
                                })
                        })
                        .count(),
                },
            );
            visible_ancestor.insert(id.clone(), id.clone());
            stack.extend(n.children.iter().rev().map(|child| {
                (
                    child.clone(),
                    depth + 1,
                    if composite && !expanded {
                        Some(id.clone())
                    } else {
                        None
                    },
                )
            }));
        }
        // Stable port order, independently for incoming and outgoing links.
        let mut projected = Vec::new();
        let mut capabilities = Vec::new();
        for n in d.nodes.values() {
            if let Some(l) = &n.link {
                let (Some(a), Some(b)) = (
                    visible_ancestor.get(&l.source_node_id),
                    visible_ancestor.get(&l.target_node_id),
                ) else {
                    continue;
                };
                if a == b && l.source_node_id != l.target_node_id {
                    continue;
                }
                if matches!(
                    relation(d, n),
                    Some(
                        Relation::Dependency
                            | Relation::Source
                            | Relation::Runnable
                            | Relation::TypeUsage
                    )
                ) {
                    capabilities.push((n, a.clone(), b.clone()));
                    continue;
                }
                projected.push((n, a.clone(), b.clone()));
            }
        }
        let mut edges = BTreeMap::new();
        for (n, a, b) in projected {
            let l = n.link.as_ref().unwrap();
            let src = &shapes[&a];
            let dst = &shapes[&b];
            let type_name = if l.type_name.is_empty() {
                d.nodes
                    .get(&l.type_definition_id)
                    .map(|n| n.name.as_str())
                    .unwrap_or("")
            } else {
                &l.type_name
            };
            edges.insert(
                n.id.clone(),
                Edge {
                    name: n.name.clone(),
                    annotation_only: false,
                    route_error: None,
                    source_label: format!(
                        "{}{}{}",
                        n.name,
                        if type_name.is_empty() { "" } else { ":" },
                        type_name
                    ),
                    target_label: format!(
                        "{}{}{}",
                        n.name,
                        if type_name.is_empty() { "" } else { ":" },
                        type_name
                    ),
                    points: vec![],
                    crossings: vec![],
                    relation: relation(d, n),
                    color_key: match relation(d, n) {
                        Some(Relation::Error) => "linkError",
                        Some(Relation::Dependency) => {
                            if ["usage", "use", "import", "library"]
                                .contains(&l.transport_kind.as_str())
                            {
                                "linkLibrary"
                            } else {
                                "linkDependency"
                            }
                        }
                        Some(Relation::TypeUsage) => "typeText",
                        Some(Relation::Source) => "linkSource",
                        Some(Relation::Runnable) => "linkRunnable",
                        _ => "linkDefault",
                    },
                    dashed: dst.rect.x < src.rect.x
                        || matches!(
                            relation(d, n),
                            Some(Relation::Dependency | Relation::Source | Relation::Runnable)
                        ),
                    source: a.clone(),
                    target: b.clone(),
                },
            );
        }
        // Right-side callouts stack beside their owner and count as lateral rows.
        let mut callouts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for (_, a, _) in &capabilities {
            callouts.entry(a.clone()).or_default().0 += 1;
        }
        for n in d.nodes.values() {
            if let Some(link) = &n.link
                && !link.type_definition_id.is_empty()
            {
                callouts
                    .entry(link.type_definition_id.clone())
                    .or_default()
                    .1 += 1;
            }
            for field in n
                .type_definition
                .as_ref()
                .and_then(|v| v["fields"].as_array())
                .into_iter()
                .flatten()
            {
                if let Some(type_id) = field["typeId"].as_str()
                    && d.nodes.contains_key(type_id)
                {
                    callouts.entry(type_id.into()).or_default().1 += 1;
                }
            }
        }
        auto_size(d, &mut shapes, &edges, &callouts);
        let mut references: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for n in d.nodes.values() {
            if let Some(link) = &n.link
                && !link.type_definition_id.is_empty()
            {
                references
                    .entry(link.type_definition_id.clone())
                    .or_default()
                    .push((n.id.clone(), n.name.clone()));
            }
            if let Some(definition) = &n.type_definition {
                for field in definition["fields"].as_array().into_iter().flatten() {
                    if let Some(type_id) = field["typeId"].as_str()
                        && d.nodes.contains_key(type_id)
                    {
                        references.entry(type_id.into()).or_default().push((
                            n.id.clone(),
                            format!("{}:{}", n.name, field["name"].as_str().unwrap_or("")),
                        ));
                    }
                }
            }
        }
        let mut annotations = BTreeMap::new();
        for n in d.nodes.values() {
            let Some(shape) = shapes.get(&n.id) else {
                continue;
            };
            let usages = n
                .type_definition
                .as_ref()
                .and_then(|v| v["fields"].as_array())
                .into_iter()
                .flatten()
                .filter_map(|field| {
                    let id = field["typeId"].as_str()?;
                    let provider = d.nodes.get(id)?;
                    Some((
                        id.to_string(),
                        format!(
                            "{}: {}",
                            provider.name,
                            field["name"].as_str().unwrap_or("")
                        ),
                    ))
                })
                .collect::<Vec<_>>();
            for (i, (target, label)) in usages.iter().enumerate() {
                annotations.insert(
                    format!("type-input:{}:{i}", n.id),
                    Annotation {
                        owner: n.id.clone(),
                        target: target.clone(),
                        flag: true,
                        color_key: "typeStroke",
                        rect: Rect {
                            x: shape.rect.x + 36.,
                            y: shape.rect.y - usages.len() as f64 * 24. - 26. + i as f64 * 24.,
                            width: (label.chars().count() as f64 * 7. + 16.).max(80.),
                            height: 26.,
                        },
                        label: label.clone(),
                    },
                );
            }
        }

        for (type_id, usages) in references {
            if let Some(shape) = shapes.get(&type_id) {
                for (i, (target, label)) in usages.into_iter().enumerate() {
                    let rect = Rect {
                        x: shape.rect.x + shape.rect.width + 22.,
                        y: shape.rect.y + 8. + i as f64 * 24.,
                        width: (label.chars().count() as f64 * 7. + 16.).clamp(80., 320.),
                        height: 18.,
                    };
                    annotations.insert(
                        format!("type-usage:{type_id}:{i}"),
                        Annotation {
                            owner: type_id.clone(),
                            flag: false,
                            color_key: "typeStroke",
                            target,
                            label,
                            rect,
                        },
                    );
                }
            }
        }
        let mut providers: BTreeMap<String, Vec<_>> = BTreeMap::new();
        let mut consumers: BTreeMap<String, Vec<_>> = BTreeMap::new();
        for (n, a, b) in capabilities {
            providers.entry(a.clone()).or_default().push(n);
            consumers.entry(b.clone()).or_default().push(n);
            edges.insert(
                n.id.clone(),
                Edge {
                    name: n.name.clone(),
                    annotation_only: true,
                    route_error: None,
                    source_label: String::new(),
                    target_label: String::new(),
                    points: vec![],
                    crossings: vec![],
                    relation: relation(d, n),
                    color_key: capability_color(relation(d, n)),
                    dashed: true,
                    source: a,
                    target: b,
                },
            );
        }
        for (owner, links) in providers {
            let shape = &shapes[&owner];
            for (i, n) in links.iter().enumerate() {
                let source = d.node(&n.link.as_ref().unwrap().source_node_id).ok();
                let label = format!(
                    "{}: {}",
                    n.name,
                    source.map(|n| n.name.as_str()).unwrap_or(&shape.name)
                );
                annotations.insert(
                    format!("provider:{}", n.id),
                    Annotation {
                        owner: owner.clone(),
                        target: n.id.clone(),
                        flag: false,
                        color_key: capability_color(relation(d, n)),
                        rect: Rect {
                            x: shape.rect.x + shape.rect.width + 24.,
                            y: shape.rect.y + 8. + i as f64 * 30.,
                            width: (label.chars().count() as f64 * 7. + 16.).max(80.),
                            height: 24.,
                        },
                        label,
                    },
                );
            }
        }
        for (owner, links) in consumers {
            let shape = &shapes[&owner];
            let count = links.len();
            for (i, n) in links.iter().enumerate() {
                let source = d.node(&n.link.as_ref().unwrap().source_node_id).ok();
                let label = format!(
                    "{}: {}",
                    n.name,
                    source.map(|n| n.name.as_str()).unwrap_or("")
                );
                annotations.insert(
                    format!("consumer:{}", n.id),
                    Annotation {
                        owner: owner.clone(),
                        target: n.id.clone(),
                        flag: true,
                        color_key: capability_color(relation(d, n)),
                        rect: Rect {
                            x: shape.rect.x + 36.,
                            y: shape.rect.y - count as f64 * 24. - 26. + i as f64 * 24.,
                            width: (label.chars().count() as f64 * 7. + 16.).max(80.),
                            height: 26.,
                        },
                        label,
                    },
                );
            }
        }
        let mut drawing = Self {
            visible_ancestor,
            shapes,
            edges,
            annotations,
            boundary_ports: vec![],
        };
        crate::routing::route_drawing(d, &mut drawing);
        drawing
    }
}

/// Vertical distance reserved for each lateral port row or right-side callout.
pub const PORT_PITCH: f64 = 24.;
/// Free space between a composite's border and its children when unset in the model.
/// Wider than the router's 46-unit boundary stub so boundary links clear the children.
pub const COMPOSITE_PADDING: f64 = 64.;
/// Space above a composite's children for its three header lines.
const COMPOSITE_HEADER: f64 = 64.;
/// Stored `layout.padding`, the composite free space edited by its resize handle.
pub fn composite_padding(n: &crate::Node) -> f64 {
    n.layout
        .extra
        .get("padding")
        .and_then(|v| v.as_f64())
        .filter(|v| v.is_finite())
        .map_or(COMPOSITE_PADDING, |v| v.clamp(0., 2000.))
}
/// Text width estimate for the canvas sans-serif face; slightly generous so
/// proportional glyphs never cross the border.
fn text_width(text: &str, size: f64) -> f64 {
    text.chars().count() as f64 * size * 0.6
}
/// Simple boxes take their width from their text lines and their height from
/// the most occupied lateral side. Expanded composites enclose their visible
/// children plus padding. Notes keep their stored size. The stored layout is
/// not modified; this is the shared projection for canvas, routing and export.
fn auto_size(
    d: &Document,
    shapes: &mut BTreeMap<String, Shape>,
    edges: &BTreeMap<String, Edge>,
    callouts: &BTreeMap<String, (usize, usize)>,
) {
    let fitted = |s: &Shape| s.kind != NodeKind::Note && !(s.composite && s.expanded);
    for shape in shapes.values_mut().filter(|s| fitted(s)) {
        let title = format!(
            "{}{} {}",
            if shape.composite { "⊞ " } else { "" },
            shape.name,
            shape.detail
        );
        let text = [
            text_width(&title, 14.),
            text_width(&shape.role, 11.),
            text_width(&shape.technology, 10.),
        ]
        .into_iter()
        .chain(shape.fields.iter().map(|f| text_width(f, 10.)))
        .fold(0., f64::max);
        // Left text inset plus room for the diagnostic badge on the right.
        shape.rect.width = (9. + text + 26.).max(120.).ceil();
    }
    fit_composites(d, shapes);
    // Same side rule as the router: x-centers only, which sizing below does not change.
    let center = |s: &Shape| s.rect.x + s.rect.width / 2.;
    let mut sides: BTreeMap<(String, bool), usize> = BTreeMap::new();
    let mut loops: Vec<&str> = vec![];
    for edge in edges.values().filter(|e| !e.annotation_only) {
        let (Some(a), Some(b)) = (shapes.get(&edge.source), shapes.get(&edge.target)) else {
            continue;
        };
        if edge.source == edge.target {
            loops.push(&edge.source);
            continue;
        }
        *sides
            .entry((edge.source.clone(), center(b) >= center(a)))
            .or_default() += 1;
        *sides
            .entry((edge.target.clone(), center(a) >= center(b)))
            .or_default() += 1;
    }
    for owner in loops {
        let left = sides.get(&(owner.into(), false)).copied().unwrap_or(0);
        let right = sides.get(&(owner.into(), true)).copied().unwrap_or(0);
        *sides.entry((owner.into(), right <= left)).or_default() += 2;
    }
    for (id, shape) in shapes.iter_mut().filter(|(_, s)| fitted(s)) {
        let rows = sides
            .get(&(id.clone(), false))
            .copied()
            .unwrap_or(0)
            .max(sides.get(&(id.clone(), true)).copied().unwrap_or(0));
        let text = 8. + 3. * 19. + shape.fields.len() as f64 * 15. + 8.;
        // Ports are spread over the height minus a 12-unit inset at both ends.
        let ports = if rows == 0 {
            0.
        } else {
            2. * 12. + (rows + 1) as f64 * PORT_PITCH
        };
        let (providers, usages) = callouts.get(id).copied().unwrap_or((0, 0));
        let beside = (8. + providers as f64 * 30.).max(8. + usages as f64 * 24.);
        shape.rect.height = text.max(ports).max(beside).max(70.).ceil();
    }
    fit_composites(d, shapes);
}
/// Encloses each expanded composite's visible children, deepest first.
fn fit_composites(d: &Document, shapes: &mut BTreeMap<String, Shape>) {
    let mut composites = shapes
        .iter()
        .filter(|(_, s)| s.composite && s.expanded)
        .map(|(id, s)| (s.depth, id.clone()))
        .collect::<Vec<_>>();
    composites.sort_by_key(|c| std::cmp::Reverse(c.0));
    for (_, id) in composites {
        let Some(n) = d.nodes.get(&id) else {
            continue;
        };
        let Some(bounds) = n
            .children
            .iter()
            .filter_map(|c| shapes.get(c).map(|s| s.rect))
            .reduce(Rect::union)
        else {
            continue;
        };
        let padding = composite_padding(n);
        let title = text_width(&format!("⊟ {} {}", n.name, n.name_detail), 14.) + 9. + 26.;
        if let Some(shape) = shapes.get_mut(&id) {
            shape.rect = Rect {
                x: bounds.x - padding,
                y: bounds.y - padding - COMPOSITE_HEADER,
                width: (bounds.width + 2. * padding).max(title),
                height: bounds.height + 2. * padding + COMPOSITE_HEADER,
            };
        }
    }
}
fn capability_color(relation: Option<Relation>) -> &'static str {
    match relation {
        Some(Relation::Source) => "linkSource",
        Some(Relation::Runnable) => "linkRunnable",
        Some(Relation::TypeUsage) => "typeStroke",
        _ => "linkDependency",
    }
}

/// A layer-change pair around an unconnected crossing. The muted connector is
/// painted first, underneath every foreground trace; circles remain hollow.
#[derive(Clone)]
pub struct ViaPair {
    pub before: [f64; 2],
    pub after: [f64; 2],
    pub radius: f64,
    pub backside: Vec<[f64; 2]>,
}
pub struct TraceGeometry {
    pub fragments: Vec<Vec<[f64; 2]>>,
    pub vias: Vec<ViaPair>,
}
pub fn trace_geometry(points: &[[f64; 2]], crossings: &[[f64; 2]]) -> TraceGeometry {
    let mut geometry = TraceGeometry {
        fragments: vec![],
        vias: vec![],
    };
    if points.len() < 2 {
        return geometry;
    }
    let mut distances = vec![0.];
    for p in points.windows(2) {
        distances.push(distances.last().unwrap() + (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1]));
    }
    let total = *distances.last().unwrap();
    if total < 1e-6 {
        return geometry;
    }
    let at = |distance: f64| {
        let i = distances
            .partition_point(|v| *v < distance)
            .saturating_sub(1)
            .min(points.len() - 2);
        let length = distances[i + 1] - distances[i];
        let along = (distance - distances[i]).clamp(0., length);
        if length > 1e-9 {
            [
                points[i][0] + ((points[i + 1][0] - points[i][0]) / length) * along,
                points[i][1] + ((points[i + 1][1] - points[i][1]) / length) * along,
            ]
        } else {
            points[i]
        }
    };
    let slice = |a: f64, b: f64| {
        let mut result = vec![at(a)];
        for (i, distance) in distances.iter().enumerate() {
            if *distance > a + 1e-6 && *distance < b - 1e-6 {
                result.push(points[i]);
            }
        }
        result.push(at(b));
        result
    };
    let mut cuts = vec![];
    for crossing in crossings {
        for (i, p) in points.windows(2).enumerate() {
            let len = distances[i + 1] - distances[i];
            if len > 1e-9 && crate::viewport::distance_to_segment(*crossing, p[0], p[1]) < 0.01 {
                let along = ((crossing[0] - p[0][0]) * (p[1][0] - p[0][0])
                    + (crossing[1] - p[0][1]) * (p[1][1] - p[0][1]))
                    / len;
                let t = distances[i] + along.clamp(0., len);
                if t > 1. && t < total - 1. {
                    cuts.push(t);
                }
                break;
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    let mut cursor = 0.;
    for (i, &t) in cuts.iter().enumerate() {
        let room = t
            .min(total - t)
            .min(if i > 0 { (t - cuts[i - 1]) / 2. } else { total })
            .min(cuts.get(i + 1).map(|v| (v - t) / 2.).unwrap_or(total));
        let radius = 2.5_f64.min(room / 5.);
        let offset = 12_f64.min(room - radius - 0.5);
        if offset <= radius {
            continue;
        }
        let left = t - offset - radius;
        let right = t + offset + radius;
        if left > cursor + 1e-6 {
            geometry.fragments.push(slice(cursor, left));
        }
        geometry.vias.push(ViaPair {
            before: at(t - offset),
            after: at(t + offset),
            radius,
            backside: slice(t - offset + radius, t + offset - radius),
        });
        cursor = right;
    }
    if cursor < total - 1e-6 {
        geometry.fragments.push(slice(cursor, total));
    }
    geometry
}
/// The reference flow arrow sits three quarters along the visible copper.
/// Keeping it on a foreground fragment avoids drawing over a layer crossing.
pub fn flow_arrow(fragments: &[Vec<[f64; 2]>]) -> Vec<[f64; 2]> {
    let length = |a: [f64; 2], b: [f64; 2]| (b[0] - a[0]).hypot(b[1] - a[1]);
    let total = fragments
        .iter()
        .flat_map(|p| p.windows(2))
        .map(|p| length(p[0], p[1]))
        .sum::<f64>();
    let mut remaining = total * 0.75;
    for p in fragments.iter().flat_map(|p| p.windows(2)) {
        let len = length(p[0], p[1]);
        if len < 1e-6 {
            continue;
        }
        if remaining <= len {
            let u = [(p[1][0] - p[0][0]) / len, (p[1][1] - p[0][1]) / len];
            let extent = 10_f64.min(len / 2.);
            let remaining = remaining.clamp(extent, len - extent);
            let center = [p[0][0] + u[0] * remaining, p[0][1] + u[1] * remaining];
            return vec![
                [center[0] + u[0] * extent, center[1] + u[1] * extent],
                [
                    center[0] - u[0] * extent - u[1] * (extent / 2.),
                    center[1] - u[1] * extent + u[0] * (extent / 2.),
                ],
                [
                    center[0] - u[0] * extent + u[1] * (extent / 2.),
                    center[1] - u[1] * extent - u[0] * (extent / 2.),
                ],
            ];
        }
        remaining -= len;
    }
    vec![]
}
