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
                            v["severity"] == 1
                                || v["severity"]
                                    .as_str()
                                    .is_some_and(|s| s.eq_ignore_ascii_case("error"))
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
                    source_label: format!(
                        "{}{}{}",
                        l.source_port_name,
                        if type_name.is_empty() { "" } else { ":" },
                        type_name
                    ),
                    target_label: l.target_port_name.clone(),
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
                    source_label: String::new(),
                    target_label: String::new(),
                    points: vec![],
                    crossings: vec![],
                    relation: relation(d, n),
                    color_key: "linkDependency",
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
                        color_key: "linkDependency",
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
                        color_key: "linkDependency",
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
