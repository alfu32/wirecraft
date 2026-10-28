//! Pluggable components (docs/design/components.md).
//!
//! A design publishes ports by linking entities to Exterior placements, one link each.
//! Another design places a Reference to it, pinned by SHA-256, and links to those ports.
//! Expansion copies the component in place of its Reference and joins every published
//! port with its consumer links into one channel, so the boundary adds no transport hop.
use crate::{
    Diagnostic, Direction, Document, Error, Node, NodeKind, Result,
    analysis::{Relation, relation},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

pub const EXTERIOR: &str = "Exterior";
pub const REFERENCE: &str = "Reference";
/// An Exterior circle is as tall as a single-port processing node.
pub const EXTERIOR_DIAMETER: f64 = 73.;
/// Link extras set by expansion only; the C17 emitter reads them, files never store them.
pub const SOURCE_ARGUMENT: &str = "wirecraftSourceArgument";
pub const TARGET_ARGUMENT: &str = "wirecraftTargetArgument";
pub const JOIN: &str = "wirecraftJoin";

/// A referenced design loaded from disk, or why it could not be.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub path: PathBuf,
    pub loaded: std::result::Result<(Arc<Document>, String), String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedPort {
    pub name: String,
    pub direction: Direction,
    #[serde(default)]
    pub type_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ComponentRef {
    pub path: String,
    pub sha256: String,
    pub name: String,
    pub date: String,
    pub ports: Vec<PublishedPort>,
    #[serde(flatten)]
    pub extra: crate::Extras,
}

pub fn declared(n: &Node) -> &str {
    n.extra
        .get("stereotype")
        .and_then(Value::as_str)
        .unwrap_or("")
}
pub fn is_exterior(n: &Node) -> bool {
    !n.is_link() && declared(n) == EXTERIOR
}
pub fn is_reference(n: &Node) -> bool {
    !n.is_link() && declared(n) == REFERENCE
}
pub fn reference(n: &Node) -> Option<ComponentRef> {
    n.extra
        .get("reference")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

/// Links attached to an entity, each with whether the entity is that link's target.
fn attached<'a>(d: &'a Document, n: &Node) -> Vec<(&'a Node, bool)> {
    n.outgoing_links
        .iter()
        .map(|id| (id, false))
        .chain(n.incoming_links.iter().map(|id| (id, true)))
        .filter_map(|(id, incoming)| d.nodes.get(id).map(|l| (l, incoming)))
        .collect()
}
/// `<link>Out` when the Exterior receives (an output port), `<link>In` when it sends.
pub fn derived_name(link: &Node, output: bool) -> String {
    format!("{}{}", link.name, if output { "Out" } else { "In" })
}
/// Display and comparison name of a link's payload type.
pub fn link_type(d: &Document, link: &Node) -> String {
    let Some(l) = &link.link else {
        return String::new();
    };
    if let Some(t) = d.nodes.get(&l.type_definition_id) {
        return t.name.clone();
    }
    // `<reference>::<type>` names a component type that exists only after expansion;
    // its typeName carries the display name. Other IDs are built-in types.
    let namespaced = l.type_definition_id.contains("::");
    if !l.type_definition_id.trim().is_empty() && !namespaced {
        return l.type_definition_id.clone();
    }
    if !l.type_name.trim().is_empty() {
        return l.type_name.clone();
    }
    if namespaced {
        return l.type_definition_id.clone();
    }
    l.type_expression
        .as_ref()
        .map(|e| e.to_string())
        .unwrap_or_default()
}
/// Ports a design publishes: one per Exterior that has exactly one link.
pub fn published_ports(d: &Document) -> Vec<(PublishedPort, String)> {
    let mut ports = vec![];
    for n in d.nodes.values().filter(|n| is_exterior(n)) {
        if let [(link, output)] = attached(d, n)[..] {
            ports.push((
                PublishedPort {
                    name: link.name.clone(),
                    direction: if output {
                        Direction::Output
                    } else {
                        Direction::Input
                    },
                    type_name: link_type(d, link),
                },
                link.id.clone(),
            ));
        }
    }
    ports.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    ports
}
pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Load every referenced design relative to `base`, the referencing file's directory.
pub fn resolve(d: &mut Document, base: &Path) {
    resolve_within(d, base, &mut vec![]);
}
fn resolve_within(d: &mut Document, base: &Path, stack: &mut Vec<PathBuf>) {
    d.components.clear();
    let references: Vec<(String, ComponentRef)> = d
        .nodes
        .values()
        .filter(|n| is_reference(n))
        .filter_map(|n| reference(n).map(|r| (n.id.clone(), r)))
        .collect();
    for (id, r) in references {
        let path = base.join(&r.path);
        let loaded = (|| {
            let relative = Path::new(&r.path);
            if r.path.trim().is_empty()
                || relative
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
            {
                return Err(format!(
                    "Component path '{}' must stay inside the design's directory",
                    r.path
                ));
            }
            let bytes = std::fs::read(&path)
                .map_err(|e| format!("Cannot read component '{}': {e}", r.path))?;
            let canonical = path.canonicalize().unwrap_or(path.clone());
            if stack.contains(&canonical) {
                return Err(format!("Component '{}' references itself", r.path));
            }
            let text = String::from_utf8(bytes.clone())
                .map_err(|_| format!("Component '{}' is not UTF-8", r.path))?;
            let mut component = crate::store::decode(&text)
                .map_err(|e| format!("Component '{}' is invalid: {}", r.path, e.message))?;
            stack.push(canonical);
            resolve_within(
                &mut component,
                path.parent().unwrap_or(Path::new(".")),
                stack,
            );
            stack.pop();
            Ok((Arc::new(component), sha256(&bytes)))
        })();
        d.components.insert(id, Resolved { path, loaded });
    }
}

/// Re-pin a Reference to its resolved design: hash, master revision and published ports.
pub fn pin(d: &mut Document, id: &str) -> Result<ComponentRef> {
    let resolved = d
        .components
        .get(id)
        .ok_or_else(|| Error::new("not_found", "Reference has not been resolved"))?;
    let (component, hash) = resolved
        .loaded
        .clone()
        .map_err(|message| Error::new("invalid_input", message))?;
    let node = d
        .nodes
        .get_mut(id)
        .ok_or_else(|| Error::new("not_found", "Reference does not exist"))?;
    let mut r = reference(node).unwrap_or_default();
    r.sha256 = hash;
    r.name = component.project_name().into();
    r.date = component.master_revision["date"]
        .as_str()
        .unwrap_or("")
        .into();
    r.ports = published_ports(&component)
        .into_iter()
        .map(|(port, _)| port)
        .collect();
    node.extra
        .insert("reference".into(), serde_json::to_value(&r)?);
    Ok(r)
}

/// Keep derived state current after an edit: Exterior names follow their link, and
/// links to a Reference name one of its published ports.
pub fn normalize(d: &mut Document) -> Result<()> {
    let mut names = vec![];
    for n in d.nodes.values().filter(|n| is_exterior(n)) {
        if let [(link, output)] = attached(d, n)[..] {
            names.push((n.id.clone(), derived_name(link, output)));
        }
    }
    for (id, name) in names {
        d.nodes.get_mut(&id).unwrap().name = name;
    }
    let mut assignments = vec![];
    for n in d.nodes.values().filter(|n| is_reference(n)) {
        let Some(r) = reference(n) else {
            continue;
        };
        for (link, incoming) in attached(d, n) {
            let l = link.link.as_ref().unwrap();
            let (current, direction) = if incoming {
                (&l.target_port_name, Direction::Input)
            } else {
                (&l.source_port_name, Direction::Output)
            };
            let candidates: Vec<&PublishedPort> = r
                .ports
                .iter()
                .filter(|p| p.direction == direction)
                .collect();
            if candidates.iter().any(|p| &p.name == current) {
                continue;
            }
            let ty = link_type(d, link);
            let typed: Vec<&PublishedPort> = candidates
                .iter()
                .copied()
                .filter(|p| !ty.is_empty() && p.type_name == ty)
                .collect();
            let chosen = candidates
                .iter()
                .copied()
                .find(|p| p.name == link.name)
                .or((typed.len() == 1).then(|| typed[0]))
                .or((candidates.len() == 1).then(|| candidates[0]));
            if let Some(port) = chosen {
                assignments.push((link.id.clone(), incoming, port.clone()));
            }
        }
    }
    let changed = !assignments.is_empty();
    for (id, incoming, port) in assignments {
        let l = d.nodes.get_mut(&id).unwrap().link.as_mut().unwrap();
        if incoming {
            l.target_port_name = port.name;
        } else {
            l.source_port_name = port.name;
        }
        if l.type_definition_id.trim().is_empty()
            && l.type_name.trim().is_empty()
            && l.type_expression.is_none()
        {
            l.type_name = port.type_name;
        }
    }
    if changed {
        d.sync_links()?;
    }
    // Ports a Reference no longer uses (such as the default out/in) are not published.
    let used: BTreeMap<String, BTreeSet<String>> = d
        .nodes
        .values()
        .filter(|n| is_reference(n))
        .map(|n| {
            let names = attached(d, n)
                .into_iter()
                .map(|(link, incoming)| {
                    let l = link.link.as_ref().unwrap();
                    if incoming {
                        l.target_port_name.clone()
                    } else {
                        l.source_port_name.clone()
                    }
                })
                .collect();
            (n.id.clone(), names)
        })
        .collect();
    for (id, names) in used {
        d.nodes
            .get_mut(&id)
            .unwrap()
            .ports
            .retain(|p| names.contains(&p.name));
    }
    Ok(())
}

/// Component rules. Errors block generation; warnings only inform.
pub fn diagnostics(d: &Document) -> Vec<Diagnostic> {
    let mut out = vec![];
    let mut add = |severity: &str, id: &str, message: String| {
        out.push(Diagnostic {
            severity: severity.into(),
            message,
            node_id: Some(id.into()),
        })
    };
    let mut published = BTreeMap::<String, usize>::new();
    for n in d.nodes.values().filter(|n| is_exterior(n)) {
        if !n.children.is_empty()
            || [&n.text.source, &n.text.initialization, &n.text.tests]
                .iter()
                .any(|t| !t.trim().is_empty())
        {
            add(
                "Error",
                &n.id,
                "An Exterior has no children, source, initialization or tests".into(),
            );
        }
        let links = attached(d, n);
        let [(link, output)] = links[..] else {
            // A freshly placed port is not published until it is linked.
            if links.is_empty() {
                add(
                    "Warning",
                    &n.id,
                    format!("Port '{}' is not linked yet; link it to publish it", n.name),
                );
            } else {
                add(
                    "Error",
                    &n.id,
                    format!(
                        "An Exterior needs exactly one link; '{}' has {}",
                        n.name,
                        links.len()
                    ),
                );
            }
            continue;
        };
        *published.entry(derived_name(link, output)).or_default() += 1;
        let l = link.link.as_ref().unwrap();
        let other = if output {
            &l.source_node_id
        } else {
            &l.target_node_id
        };
        if d.nodes.get(other).is_some_and(is_exterior) {
            add(
                "Error",
                &link.id,
                "An Exterior cannot link to another Exterior".into(),
            );
        }
        if !matches!(
            relation(d, link),
            Some(Relation::Principal | Relation::Error)
        ) {
            add(
                "Error",
                &link.id,
                format!(
                    "Published port '{}' must be a data link, not a library, source or type link",
                    link.name
                ),
            );
        }
        if link_type(d, link).is_empty() {
            add(
                "Error",
                &link.id,
                format!("Published port '{}' needs an explicit link type", link.name),
            );
        }
    }
    for (name, count) in published.into_iter().filter(|(_, c)| *c > 1) {
        add(
            "Error",
            &d.root_node_id,
            format!("Duplicate published port '{name}' ({count} Exteriors)"),
        );
    }
    let mut components = BTreeMap::<PathBuf, String>::new();
    for n in d.nodes.values().filter(|n| is_reference(n)) {
        let Some(r) = reference(n).filter(|r| !r.path.trim().is_empty()) else {
            add("Error", &n.id, "A Reference needs a component path".into());
            continue;
        };
        if !n.children.is_empty() || !n.text.source.trim().is_empty() {
            add(
                "Error",
                &n.id,
                "A Reference is read-only; edit the referenced component instead".into(),
            );
        }
        if let Some(resolved) = d.components.get(&n.id) {
            match &resolved.loaded {
                Err(message) => add("Error", &n.id, message.clone()),
                Ok((_, hash)) if hash != &r.sha256 => add(
                    "Error",
                    &n.id,
                    format!(
                        "Component '{}' changed since it was pinned; update the reference",
                        r.path
                    ),
                ),
                Ok(_) => {}
            }
            let key = resolved
                .path
                .canonicalize()
                .unwrap_or(resolved.path.clone());
            if let Some(first) = components.insert(key, n.name.clone()) {
                add(
                    "Error",
                    &n.id,
                    format!(
                        "'{}' and '{first}' reference the same component; one instance per program is supported until per-instance state exists",
                        n.name
                    ),
                );
            }
        }
        let mut outputs = BTreeMap::<String, usize>::new();
        for (link, incoming) in attached(d, n) {
            let l = link.link.as_ref().unwrap();
            if !matches!(
                relation(d, link),
                Some(Relation::Principal | Relation::Error)
            ) {
                add(
                    "Error",
                    &link.id,
                    format!("Link '{}' to a Reference must be a data link", link.name),
                );
                continue;
            }
            let (name, direction, label) = if incoming {
                (&l.target_port_name, Direction::Input, "input")
            } else {
                (&l.source_port_name, Direction::Output, "output")
            };
            let Some(port) = r
                .ports
                .iter()
                .find(|p| &p.name == name && p.direction == direction)
            else {
                add(
                    "Error",
                    &link.id,
                    format!(
                        "Link '{}' uses unknown {label} port '{name}' of '{}'",
                        link.name, n.name
                    ),
                );
                continue;
            };
            let ty = link_type(d, link);
            if !ty.is_empty() && ty != port.type_name {
                add(
                    "Error",
                    &link.id,
                    format!(
                        "Link '{}' carries {ty}, but port '{name}' of '{}' carries {}",
                        link.name, n.name, port.type_name
                    ),
                );
            }
            if !incoming {
                *outputs.entry(name.clone()).or_default() += 1;
            }
        }
        for port in r.ports.iter().filter(|p| p.direction == Direction::Output) {
            match outputs.get(&port.name).copied().unwrap_or(0) {
                0 => add(
                    "Warning",
                    &n.id,
                    format!(
                        "Output port '{}' of '{}' is not connected; it must be before generation",
                        port.name, n.name
                    ),
                ),
                1 => {}
                count => add(
                    "Error",
                    &n.id,
                    format!(
                        "Output port '{}' of '{}' feeds {count} links; route it through a splitter entity",
                        port.name, n.name
                    ),
                ),
            }
        }
    }
    out
}

/// Replace every Reference by a copy of its component, with published ports joined to
/// their consumer links. Designs without References are returned unchanged.
pub fn expand(d: &Document) -> Result<Cow<'_, Document>> {
    if !d.nodes.values().any(is_reference) {
        return Ok(Cow::Borrowed(d));
    }
    let failures: Vec<String> = diagnostics(d)
        .into_iter()
        .filter(|e| e.severity == "Error")
        .map(|e| e.message)
        .collect();
    if !failures.is_empty() {
        return Err(Error::new("invalid_graph", failures.join("; ")));
    }
    let mut out = d.clone();
    let references: Vec<String> = d
        .nodes
        .values()
        .filter(|n| is_reference(n))
        .map(|n| n.id.clone())
        .collect();
    for id in references {
        let resolved = d.components.get(&id).ok_or_else(|| {
            Error::new(
                "invalid_input",
                format!("Reference '{}' has not been resolved", d.nodes[&id].name),
            )
        })?;
        let (component, _) = resolved
            .loaded
            .clone()
            .map_err(|message| Error::new("invalid_input", message))?;
        let component = expand(&component)?.into_owned();
        splice(&mut out, &id, &component)?;
    }
    out.sync_links()?;
    out.ensure_valid()?;
    Ok(Cow::Owned(out))
}
fn splice(out: &mut Document, reference_id: &str, c: &Document) -> Result<()> {
    let reference_name = out.nodes[reference_id].name.clone();
    let map = |id: &str| {
        if id == c.root_node_id {
            reference_id.to_string()
        } else {
            format!("{reference_id}::{id}")
        }
    };
    let ids: BTreeSet<&String> = c.nodes.keys().collect();
    let exteriors: BTreeSet<&String> = c
        .nodes
        .values()
        .filter(|n| is_exterior(n))
        .map(|n| &n.id)
        .collect();
    let port_links: BTreeMap<&String, (&Node, bool)> = c
        .nodes
        .values()
        .filter_map(|n| {
            let l = n.link.as_ref()?;
            if exteriors.contains(&l.target_node_id) {
                Some((&n.id, (n, true)))
            } else if exteriors.contains(&l.source_node_id) {
                Some((&n.id, (n, false)))
            } else {
                None
            }
        })
        .collect();
    let skipped = |id: &String| exteriors.contains(id) || port_links.contains_key(id);
    for n in c.nodes.values() {
        if n.id == c.root_node_id || skipped(&n.id) {
            continue;
        }
        let mut copy = n.clone();
        copy.id = map(&n.id);
        copy.parent_id = n.parent_id.as_deref().map(map);
        copy.children = n
            .children
            .iter()
            .filter(|id| !skipped(id))
            .map(|id| map(id))
            .collect();
        copy.incoming_links.clear();
        copy.outgoing_links.clear();
        if let Some(t) = &mut copy.type_definition {
            remap(t, &ids, &map);
        }
        if let Some(l) = &mut copy.link {
            l.source_node_id = map(&l.source_node_id);
            l.target_node_id = map(&l.target_node_id);
            remap_link(l, &ids, &map);
        }
        out.nodes.insert(copy.id.clone(), copy);
    }
    let children: Vec<String> = c.nodes[&c.root_node_id]
        .children
        .iter()
        .filter(|id| !skipped(id))
        .map(|id| map(id))
        .collect();
    let node = out.nodes.get_mut(reference_id).unwrap();
    node.kind = NodeKind::Processor;
    node.children = children;
    node.extra.remove("stereotype");
    node.extra.remove("reference");
    node.ports.clear();
    for (port, (link, output)) in port_links {
        let l = link.link.as_ref().unwrap();
        let mut typed = l.clone();
        remap_link(&mut typed, &ids, &map);
        let consumers: Vec<String> = out
            .nodes
            .values()
            .filter(|n| {
                n.link.as_ref().is_some_and(|x| {
                    if output {
                        x.source_node_id == reference_id && x.source_port_name == link.name
                    } else {
                        x.target_node_id == reference_id && x.target_port_name == link.name
                    }
                })
            })
            .map(|n| n.id.clone())
            .collect();
        let argument = crate::c17::identifier(&link.name);
        if output {
            let [consumer] = &consumers[..] else {
                return Err(Error::new(
                    "invalid_graph",
                    format!(
                        "Output port '{}' of '{reference_name}' needs exactly one consumer link",
                        link.name
                    ),
                ));
            };
            let n = out.nodes.get_mut(consumer).unwrap();
            let x = n.link.as_mut().unwrap();
            x.source_node_id = map(&l.source_node_id);
            x.source_port_name = l.source_port_name.clone();
            adopt_type(x, &typed);
            x.extra.insert(SOURCE_ARGUMENT.into(), json!(argument));
        } else if consumers.is_empty() {
            // Unconnected input: the internal link stays, fed by nothing.
            let mut copy = link.clone();
            copy.id = map(port);
            copy.parent_id = Some(reference_id.into());
            let x = copy.link.as_mut().unwrap();
            *x = typed.clone();
            x.source_node_id = reference_id.into();
            x.target_node_id = map(&l.target_node_id);
            out.nodes.insert(copy.id.clone(), copy);
            out.nodes
                .get_mut(reference_id)
                .unwrap()
                .children
                .push(map(port));
        } else {
            // Fan-in: every consumer delivers into the internal link's single input buffer.
            for consumer in consumers {
                let n = out.nodes.get_mut(&consumer).unwrap();
                let x = n.link.as_mut().unwrap();
                x.target_node_id = map(&l.target_node_id);
                x.target_port_name = l.target_port_name.clone();
                adopt_type(x, &typed);
                x.extra.insert(TARGET_ARGUMENT.into(), json!(argument));
                x.extra.insert(JOIN.into(), json!(map(port)));
            }
        }
    }
    Ok(())
}
fn adopt_type(x: &mut crate::Link, from: &crate::Link) {
    x.type_definition_id = from.type_definition_id.clone();
    x.type_name = from.type_name.clone();
    x.payload_definition = from.payload_definition.clone();
    x.type_expression = from.type_expression.clone();
}
fn remap_link(l: &mut crate::Link, ids: &BTreeSet<&String>, map: &impl Fn(&str) -> String) {
    if ids.contains(&l.type_definition_id) {
        l.type_definition_id = map(&l.type_definition_id);
    }
    if let Some(e) = &mut l.type_expression {
        remap(e, ids, map);
    }
    l.composite_boundary_ids.clear();
}
/// Rewrite every string that names a component entity, such as type references.
fn remap(v: &mut Value, ids: &BTreeSet<&String>, map: &impl Fn(&str) -> String) {
    match v {
        Value::String(s) if ids.contains(s) => *s = map(s),
        Value::Array(a) => a.iter_mut().for_each(|v| remap(v, ids, map)),
        Value::Object(o) => o.values_mut().for_each(|v| remap(v, ids, map)),
        _ => {}
    }
}
