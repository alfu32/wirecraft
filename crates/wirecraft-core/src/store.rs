//! Compatible .orch codec and atomic replacement on the destination filesystem.
use crate::{Document, Error, Node, Result};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};
use uuid::Uuid;

pub fn decode(text: &str) -> Result<Document> {
    let mut v: Value = serde_json::from_str(text)?;
    let nodes = v
        .get_mut("nodes")
        .ok_or_else(|| Error::new("invalid_input", "Missing nodes"))?;
    let entries: Vec<Value> = match nodes {
        Value::Array(a) => a.clone(),
        Value::Object(m) => m.values().cloned().collect(),
        _ => return Err(Error::new("invalid_input", "nodes must be a list or map")),
    };
    let mut map = BTreeMap::new();
    for v in entries {
        let n: Node = serde_json::from_value(v)?;
        let id = n.id.clone();
        if map.insert(id.clone(), n).is_some() {
            return Err(Error::new(
                "invalid_graph",
                format!("Duplicate node id '{id}'"),
            ));
        }
    }
    *nodes = serde_json::to_value(map)?;
    let mut d: Document = serde_json::from_value(v)?;
    d.repair()?;
    d.ensure_valid()?;
    Ok(d)
}
pub fn encode(document: &Document) -> Result<String> {
    document.ensure_valid()?;
    let mut v = serde_json::to_value(document)?;
    v["name"] = json!(document.project_name());
    let mut ordered = vec![];
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![document.root_node_id.clone()];
    while let Some(id) = stack.pop() {
        if seen.insert(id.clone()) {
            let n = document.node(&id)?;
            ordered.push(n);
            stack.extend(n.children.iter().rev().cloned());
        }
    }
    ordered.extend(document.nodes.values().filter(|n| !seen.contains(&n.id)));
    v["nodes"] = serde_json::to_value(ordered)?;
    Ok(serde_json::to_string_pretty(&v)? + "\n")
}
/// Decode a project file and resolve the components it references, relative to it.
pub fn load(path: &Path) -> Result<Document> {
    let mut d = decode(&fs::read_to_string(path)?)?;
    crate::components::resolve(&mut d, path.parent().unwrap_or(Path::new(".")));
    Ok(d)
}
pub fn save(document: &Document, path: &Path) -> Result<()> {
    atomic_write(path, encode(document)?.as_bytes())
}
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    // Unique sibling temp file prevents cross-device rename and partial project writes.
    let temporary = parent.join(format!(".wirecraft-{}.tmp", Uuid::new_v4()));
    let result = (|| -> Result<()> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        #[cfg(unix)]
        {
            fs::File::open(parent)?.sync_all()?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
