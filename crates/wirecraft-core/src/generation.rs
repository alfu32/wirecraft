//! Virtual files and origin mappings for literal filesystem workflows.
//! Graph-to-source language wrappers are intentionally a separate capability.
use crate::{Document, Error, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMap {
    pub node_id: String,
    pub section: String,
    pub generated_start_line: usize,
    pub generated_end_line: usize,
    pub source_start_line: usize,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualFile {
    pub path: String,
    pub content: String,
    pub binary_content: Option<String>,
    pub source_map: Vec<SourceMap>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedProject {
    pub name: String,
    pub files: Vec<VirtualFile>,
    pub capability: String,
}
pub fn literal_files(document: &Document, node_id: &str) -> Result<GeneratedProject> {
    document.node(node_id)?;
    let ids = document.descendants(node_id);
    let mut paths = BTreeSet::new();
    let mut files = vec![];
    for id in ids {
        let n = document.node(&id)?;
        if n.is_link() || !n.children.is_empty() {
            continue;
        }
        let technology = document.effective(&id, "technologyId");
        if !["none", "file-export", "multi-tech"].contains(&technology.as_str()) {
            return Err(Error::new(
                "unsupported",
                format!(
                    "Entity '{}' requires the {technology} graph compiler, which is not migrated yet. Literal export is limited to file-export/multi-tech entities.",
                    n.name
                ),
            ));
        }
        let explicit = n
            .metadata
            .get("path")
            .or_else(|| n.metadata.get("file"))
            .filter(|s| !s.trim().is_empty());
        let path = if let Some(path) = explicit {
            path.replace('\\', "/")
        } else {
            let mut parents = document
                .ancestors(&id)
                .into_iter()
                .skip(1)
                .take_while(|id| id != node_id && id != &document.root_node_id)
                .collect::<Vec<_>>();
            parents.reverse();
            let mut parts = parents
                .iter()
                .filter_map(|id| document.nodes.get(id))
                .map(|n| segment(&n.name))
                .collect::<Vec<_>>();
            parts.push(segment(n.name.trim_start_matches('@')));
            parts.join("/")
        };
        safe_relative_path(&path)?;
        if !paths.insert(path.clone()) {
            return Err(Error::new(
                "invalid_input",
                format!("Duplicate generated path '{path}'"),
            ));
        }
        let (content, section) = if n.text.source.is_empty() {
            (n.text.specification.clone(), "specification")
        } else {
            (n.text.source.clone(), "source")
        };
        if let Some(binary) = &n.binary_content {
            base64::engine::general_purpose::STANDARD
                .decode(binary)
                .map_err(|e| Error::new("invalid_input", e.to_string()))?;
        }
        let source_map = if n.binary_content.is_none() {
            vec![SourceMap {
                node_id: id,
                section: section.into(),
                generated_start_line: 1,
                generated_end_line: content.lines().count().max(1),
                source_start_line: 1,
            }]
        } else {
            vec![]
        };
        files.push(VirtualFile {
            path,
            content,
            binary_content: n.binary_content.clone(),
            source_map,
        });
    }
    Ok(GeneratedProject {
        name: document.project_name().into(),
        files,
        capability: "literal-filesystem".into(),
    })
}
pub fn safe_relative_path(path: &str) -> Result<()> {
    if path.is_empty()
        || path.contains(':')
        || path.contains('\\')
        || path
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
        || Path::new(path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(Error::new(
            "invalid_input",
            format!("Unsafe relative generated path '{path}'"),
        ));
    }
    Ok(())
}
fn segment(name: &str) -> String {
    let name: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "_.-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let name = name.trim_matches('_');
    if name.is_empty() || name == "." || name == ".." {
        "file".into()
    } else {
        name.into()
    }
}
