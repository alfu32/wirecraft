//! Portable graph fragments for clipboard, duplication, and archetype import.
use crate::{Document, Error, Node, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Fragment {
    pub format: String,
    pub roots: Vec<String>,
    pub nodes: Vec<Node>,
}
impl Fragment {
    pub fn copy(document: &Document, selected: &[String]) -> Result<Self> {
        let mut roots = BTreeSet::new();
        for id in selected {
            let n = document.node(id)?;
            if !n.is_link() {
                roots.insert(id.clone());
            }
        }
        if roots.is_empty() {
            return Err(Error::new(
                "invalid_input",
                "Select at least one entity other than a link",
            ));
        }
        let chosen = roots.clone();
        roots.retain(|id| {
            !document
                .ancestors(id)
                .iter()
                .skip(1)
                .any(|p| chosen.contains(p))
        });
        let mut ids = BTreeSet::new();
        for root in &roots {
            ids.extend(document.descendants(root));
        }
        let links: Vec<_> = document
            .nodes
            .values()
            .filter(|n| {
                n.link.as_ref().is_some_and(|l| {
                    ids.contains(&l.source_node_id) && ids.contains(&l.target_node_id)
                })
            })
            .map(|n| n.id.clone())
            .collect();
        ids.extend(links);
        // Exclude links crossing the fragment boundary; those stay in the original design.
        let nodes = ids
            .iter()
            .filter_map(|id| document.nodes.get(id))
            .filter(|n| {
                n.link.as_ref().is_none_or(|l| {
                    ids.contains(&l.source_node_id) && ids.contains(&l.target_node_id)
                })
            })
            .cloned()
            .collect();
        Ok(Self {
            format: "wirecraft-fragment-v1".into(),
            roots: roots.into_iter().collect(),
            nodes,
        })
    }
    pub fn from_document(document: &Document) -> Result<Self> {
        Self::copy(document, std::slice::from_ref(&document.root_node_id))
    }
    pub fn paste(
        &self,
        document: &mut Document,
        parent: &str,
        offset: [f64; 2],
    ) -> Result<Vec<String>> {
        document.node(parent)?;
        if self.format != "wirecraft-fragment-v1" {
            return Err(Error::new(
                "unsupported",
                "Unknown clipboard fragment format",
            ));
        }
        if self.nodes.is_empty()
            || self.nodes.len() > 100_000
            || !offset.iter().all(|n| n.is_finite())
        {
            return Err(Error::new(
                "invalid_input",
                "Invalid fragment size or offset",
            ));
        }
        let mut mapping = BTreeMap::new();
        for n in &self.nodes {
            if mapping
                .insert(n.id.clone(), uuid::Uuid::new_v4().to_string())
                .is_some()
            {
                return Err(Error::new("invalid_graph", "Duplicate fragment node ID"));
            }
        }
        let roots: BTreeSet<_> = self.roots.iter().cloned().collect();
        if roots.is_empty() || roots.iter().any(|r| !mapping.contains_key(r)) {
            return Err(Error::new("invalid_graph", "Fragment roots are missing"));
        }
        let ids = self
            .nodes
            .iter()
            .map(|n| n.id.clone())
            .collect::<BTreeSet<_>>();
        for n in &self.nodes {
            if !roots.contains(&n.id)
                && !n.is_link()
                && !n.parent_id.as_ref().is_some_and(|p| ids.contains(p))
            {
                return Err(Error::new(
                    "invalid_graph",
                    "Fragment contains a disconnected entity",
                ));
            }
        }
        let mut added = vec![];
        for original in &self.nodes {
            let mut n = original.clone();
            n.id = mapping[&original.id].clone();
            n.parent_id = Some(if roots.contains(&original.id) {
                parent.into()
            } else {
                original
                    .parent_id
                    .as_ref()
                    .and_then(|p| mapping.get(p))
                    .cloned()
                    .unwrap_or(parent.into())
            });
            n.children = original
                .children
                .iter()
                .filter_map(|id| mapping.get(id).cloned())
                .collect();
            n.incoming_links.clear();
            n.outgoing_links.clear();
            if roots.contains(&original.id) {
                n.layout.x += offset[0];
                n.layout.y += offset[1];
                added.push(n.id.clone());
            }
            if let Some(link) = &mut n.link {
                link.source_node_id =
                    mapping.get(&link.source_node_id).cloned().ok_or_else(|| {
                        Error::new("invalid_graph", "Fragment link has external source")
                    })?;
                link.target_node_id =
                    mapping.get(&link.target_node_id).cloned().ok_or_else(|| {
                        Error::new("invalid_graph", "Fragment link has external target")
                    })?;
                if let Some(id) = mapping.get(&link.type_definition_id) {
                    link.type_definition_id = id.clone();
                }
                if let Some(e) = &mut link.type_expression {
                    remap_types(e, &mapping);
                }
                link.composite_boundary_ids.clear();
            }
            if let Some(t) = &mut n.type_definition {
                remap_types(t, &mapping);
            }
            if n.parent_id.as_deref() == Some(parent) {
                document
                    .nodes
                    .get_mut(parent)
                    .unwrap()
                    .children
                    .push(n.id.clone());
            }
            document.nodes.insert(n.id.clone(), n);
        }
        document.sync_links()?;
        document.ensure_valid()?;
        Ok(added)
    }
}
fn remap_types(v: &mut Value, mapping: &BTreeMap<String, String>) {
    match v {
        Value::Object(o) => {
            for (key, v) in o {
                if ["typeId", "typeDefinitionId"].contains(&key.as_str()) {
                    if let Some(id) = v.as_str().and_then(|id| mapping.get(id)) {
                        *v = Value::String(id.clone());
                    }
                } else if key == "genericTypeIds" {
                    if let Some(ids) = v.as_array_mut() {
                        for id in ids {
                            if let Some(new) = id.as_str().and_then(|id| mapping.get(id)) {
                                *id = Value::String(new.clone());
                            }
                        }
                    }
                } else {
                    remap_types(v, mapping);
                }
            }
        }
        Value::Array(a) => {
            for v in a {
                remap_types(v, mapping);
            }
        }
        _ => {}
    }
}
