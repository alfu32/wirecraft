//! Visible diagram projection shared by interactive canvas and portable exports.
//! Source layouts use absolute world coordinates, including composite children.
use crate::{
    Document, NodeKind,
    analysis::{Relation, relation, stereotype},
    viewport::{Rect, SceneIndex, route},
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
    pub source_label: String,
    pub target_label: String,
    pub points: Vec<[f64; 2]>,
    pub relation: Option<Relation>,
    pub color_key: &'static str,
    pub dashed: bool,
    pub source: String,
    pub target: String,
}
#[derive(Clone)]
pub struct Annotation {
    pub owner: String,
    pub target: String,
    pub label: String,
    pub rect: Rect,
}
pub struct Drawing {
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
                    diagnostics: n.diagnostics.len(),
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
        let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut incoming: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut projected = Vec::new();
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
                outgoing.entry(a.clone()).or_default().push(n.id.clone());
                incoming.entry(b.clone()).or_default().push(n.id.clone());
                projected.push((n, a.clone(), b.clone()));
            }
        }
        let mut index = SceneIndex::default();
        if !projected.is_empty() {
            for (id, s) in &shapes {
                if !s.composite || !s.expanded {
                    index.insert(id.clone(), s.rect);
                }
            }
        }
        let mut edges = BTreeMap::new();
        for (n, a, b) in projected {
            let l = n.link.as_ref().unwrap();
            let src = &shapes[&a];
            let dst = &shapes[&b];
            let port_y = |id: &str, ports: &BTreeMap<String, Vec<String>>, r: Rect| {
                r.y + r.height
                    * ((ports[id].iter().position(|v| v == &n.id).unwrap_or(0) + 1) as f64
                        / (ports[id].len() + 1) as f64)
            };
            let start = [src.rect.x + src.rect.width, port_y(&a, &outgoing, src.rect)];
            let end = [dst.rect.x, port_y(&b, &incoming, dst.rect)];
            let mut points = if a == b {
                route(src.rect, dst.rect)
            } else {
                let mid = (start[0] + end[0]) / 2.;
                vec![start, [mid, start[1]], [mid, end[1]], end]
            };
            // Try nearby horizontal corridors if the direct dogleg crosses a terminal.
            let clear = |points: &[[f64; 2]]| {
                points.windows(2).all(|pair| {
                    let r = Rect {
                        x: pair[0][0].min(pair[1][0]) - 3.,
                        y: pair[0][1].min(pair[1][1]) - 3.,
                        width: (pair[0][0] - pair[1][0]).abs() + 6.,
                        height: (pair[0][1] - pair[1][1]).abs() + 6.,
                    };
                    index.query(r).into_iter().all(|id| id == a || id == b)
                })
            };
            if a != b && !clear(&points) {
                for y in [
                    src.rect.y.min(dst.rect.y) - 30.,
                    (src.rect.y + src.rect.height).max(dst.rect.y + dst.rect.height) + 30.,
                    src.rect.y.min(dst.rect.y) - 80.,
                    (src.rect.y + src.rect.height).max(dst.rect.y + dst.rect.height) + 80.,
                ] {
                    let candidate = vec![
                        start,
                        [start[0] + 20., start[1]],
                        [start[0] + 20., y],
                        [end[0] - 20., y],
                        [end[0] - 20., end[1]],
                        end,
                    ];
                    if clear(&candidate) {
                        points = candidate;
                        break;
                    }
                }
            }
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
                    source_label: format!(
                        "{}{}{}",
                        l.source_port_name,
                        if type_name.is_empty() { "" } else { ":" },
                        type_name
                    ),
                    target_label: l.target_port_name.clone(),
                    points,
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
                            target,
                            label,
                            rect,
                        },
                    );
                }
            }
        }
        Self {
            shapes,
            edges,
            annotations,
        }
    }
}
