use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use wirecraft_core::{store, *};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub command: String,
    #[serde(default = "empty")]
    pub arguments: Value,
    #[serde(default)]
    pub expected_revision: Option<u64>,
}
fn empty() -> Value {
    json!({})
}
impl Request {
    pub fn new(command: &str, arguments: Value) -> Self {
        Self {
            command: command.into(),
            arguments,
            expected_revision: None,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub document: Document,
    pub revision: u64,
    pub dirty: bool,
    pub path: Option<PathBuf>,
    pub can_undo: bool,
    pub can_redo: bool,
}
/// A single project session. Transport hosts serialize access; no UI types or network code.
pub struct Service {
    pub document: Document,
    pub revision: u64,
    pub path: Option<PathBuf>,
    saved: Value,
    undo: Vec<Document>,
    redo: Vec<Document>,
}
impl Service {
    pub fn new(document: Document, path: Option<PathBuf>) -> Self {
        let saved = serde_json::to_value(&document).unwrap();
        Self {
            document,
            revision: 0,
            path,
            saved,
            undo: vec![],
            redo: vec![],
        }
    }
    pub fn write_recovery(&self, path: &Path) -> Result<()> {
        store::atomic_write(
            path,
            &serde_json::to_vec(
                &json!({"document":self.document,"path":self.path,"saved":self.saved}),
            )?,
        )
    }
    pub fn read_recovery(path: &Path) -> Result<Self> {
        let v: Value = serde_json::from_slice(&std::fs::read(path)?)?;
        let mut document: Document = serde_json::from_value(v["document"].clone())?;
        document.repair()?;
        document.ensure_valid()?;
        let mut service = Self::new(document, v["path"].as_str().map(PathBuf::from));
        service.saved = v["saved"].clone();
        Ok(service)
    }
    pub fn dirty(&self) -> bool {
        serde_json::to_value(&self.document).unwrap() != self.saved
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            document: self.document.clone(),
            revision: self.revision,
            dirty: self.dirty(),
            path: self.path.clone(),
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
        }
    }
    pub fn execute(&mut self, r: Request) -> Result<Value> {
        if let Some(expected) = r.expected_revision
            && expected != self.revision
        {
            return Err(Error::new(
                "conflict",
                format!("Expected revision {expected}, current {}", self.revision),
            ));
        }
        let command = r.command.strip_prefix("threadwork.").unwrap_or(&r.command);
        let a = &r.arguments;
        match command {
            "session.status" => {
                return Ok(json!({"revision":self.revision,"dirty":self.dirty(),"path":self.path}));
            }
            "session.snapshot" => return Ok(serde_json::to_value(self.snapshot())?),
            "file.open" => {
                self.check_discard(a)?;
                let path = PathBuf::from(required(a, "path")?);
                let d = store::load(&path)?;
                self.document = d;
                self.path = Some(path);
                self.reset_history();
                return Ok(serde_json::to_value(self.snapshot())?);
            }
            "file.new" => {
                self.check_discard(a)?;
                self.document = Document::new(a["name"].as_str().unwrap_or("Untitled Wirecraft"));
                self.path = None;
                self.reset_history();
                return Ok(serde_json::to_value(self.snapshot())?);
            }
            "file.commit_and_save" => {
                // A failed draft commit must never be followed by saving the older graph.
                if let Some(patch) = a.get("patch") {
                    self.execute(Request::new(
                        "threadwork.update_entity",
                        json!({"nodeId":required(a,"nodeId")?,"patch":patch}),
                    ))?;
                }
                let mut result = self.execute(Request::new("file.save", a.clone()))?;
                if let Some(id) = a["nodeId"].as_str() {
                    result["entity"] = serde_json::to_value(self.document.node(id)?)?;
                }
                return Ok(result);
            }
            "file.save" => {
                let path = a["path"]
                    .as_str()
                    .map(PathBuf::from)
                    .or(self.path.clone())
                    .ok_or_else(|| Error::new("invalid_input", "Save requires a path"))?;
                store::save(&self.document, &path)?;
                self.path = Some(path);
                self.saved = serde_json::to_value(&self.document)?;
                return Ok(json!({"path":self.path,"saved":true}));
            }
            "edit.undo" => {
                let previous = self
                    .undo
                    .pop()
                    .ok_or_else(|| Error::new("unavailable", "Nothing to undo"))?;
                self.redo
                    .push(std::mem::replace(&mut self.document, previous));
                self.revision += 1;
                return Ok(serde_json::to_value(self.snapshot())?);
            }
            "edit.redo" => {
                let next = self
                    .redo
                    .pop()
                    .ok_or_else(|| Error::new("unavailable", "Nothing to redo"))?;
                self.undo.push(std::mem::replace(&mut self.document, next));
                self.revision += 1;
                return Ok(serde_json::to_value(self.snapshot())?);
            }
            "get_design" => {
                return Ok(
                    json!({"projectName":self.document.project_name(),"document":self.document}),
                );
            }
            "get_entity" => return entity(&self.document, required(a, "nodeId")?),
            "get_fragment" => {
                let id = required(a, "nodeId")?;
                self.document.node(id)?;
                let mut ids = if a["includeDescendants"].as_bool().unwrap_or(true) {
                    self.document.descendants(id)
                } else {
                    BTreeSet::from([id.into()])
                };
                if a["includeLinks"].as_bool().unwrap_or(true) {
                    let links: Vec<_> = self
                        .document
                        .nodes
                        .values()
                        .filter(|n| {
                            n.link.as_ref().is_some_and(|l| {
                                ids.contains(&l.source_node_id) || ids.contains(&l.target_node_id)
                            })
                        })
                        .map(|n| n.id.clone())
                        .collect();
                    ids.extend(links);
                }
                return Ok(
                    json!({"rootNodeId":id,"entities":ids.iter().filter_map(|id|self.document.nodes.get(id)).collect::<Vec<_>>()}),
                );
            }
            "list_entities" => {
                let ids = if let Some(p) = a["parentId"].as_str().filter(|s| !s.is_empty()) {
                    self.document.node(p)?;
                    if a["recursive"].as_bool().unwrap_or(true) {
                        let mut ids = self.document.descendants(p);
                        ids.remove(p);
                        ids
                    } else {
                        self.document.nodes[p].children.iter().cloned().collect()
                    }
                } else {
                    self.document.nodes.keys().cloned().collect()
                };
                let kind = a["kind"].as_str();
                let mut nodes = ids
                    .iter()
                    .filter_map(|id| self.document.nodes.get(id))
                    .filter(|n| kind.is_none_or(|k| serde_json::to_value(n.kind).unwrap() == k))
                    .collect::<Vec<_>>();
                nodes.sort_by_key(|n| self.document.qualified_name(&n.id));
                return Ok(
                    json!({"entities":nodes.iter().map(|n|json!({"id":n.id,"name":n.name,"kind":n.kind,"parentId":n.parent_id.as_deref().unwrap_or(""),"fullyQualifiedName":self.document.qualified_name(&n.id),"isComposite":n.is_composite(),"status":n.status.as_deref().unwrap_or(""),"technologyId":n.technology.technology_id})).collect::<Vec<_>>()}),
                );
            }
            "get_metadata" => {
                return if let Some(id) = a["nodeId"].as_str().filter(|s| !s.is_empty()) {
                    Ok(
                        json!({"scope":"entity","nodeId":id,"metadata":self.document.node(id)?.metadata}),
                    )
                } else {
                    Ok(json!({"scope":"document","metadata":self.document.metadata}))
                };
            }
            "list_users" => return Ok(json!({"users":self.document.users})),
            "validate_design" => {
                let diagnostics = self.document.validate();
                return Ok(json!({"valid":diagnostics.is_empty(),"diagnostics":diagnostics}));
            }
            "analyze_network" => return Ok(self.document.network_counts()),
            "list_technologies" => {
                return Ok(
                    json!({"technologies":[],"languageServers":[{"languageIds":["c","cpp"],"executable":"clangd","available":executable_exists("clangd")}],"note":"Graph-to-source compiler migration pending; external build and LSP services are separate."}),
                );
            }
            "get_command_catalog" => return Ok(command_catalog()),
            "execute_command" => {
                let id = required(a, "commandId")?;
                if !["edit.undo", "edit.redo", "file.save"].contains(&id) {
                    return Err(Error::new(
                        "unavailable",
                        format!("Unknown or disabled command '{id}'"),
                    ));
                }
                self.execute(Request::new(id, json!({})))?;
                return Ok(json!({"commandId":id,"executed":true}));
            }
            _ => {}
        }
        // Candidate snapshot isolates partial failures (patches, link repair, validation).
        let mut candidate = self.document.clone();
        let result = match command {
            "create_entity" => {
                let kind: NodeKind =
                    serde_json::from_value(a.get("kind").cloned().unwrap_or(json!("Processor")))?;
                let parent = a["parentId"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&candidate.root_node_id)
                    .to_string();
                let id = candidate.create(&parent, required(a, "name")?, kind)?;
                if let Some(patch) = a.get("attributes") {
                    candidate.patch(&id, patch)?;
                }
                entity(&candidate, &id)?
            }
            "update_entity" => {
                let id = required(a, "nodeId")?;
                let mut patch = a.get("patch").cloned().unwrap_or(a.clone());
                if let Some(o) = patch.as_object_mut() {
                    o.remove("nodeId");
                }
                candidate.patch(id, &patch)?;
                entity(&candidate, id)?
            }
            "delete_entity" => {
                let id = required(a, "nodeId")?;
                candidate.delete(id)?;
                json!({"nodeId":id,"deleted":true})
            }
            "move_entity" => {
                let id = required(a, "nodeId")?;
                candidate.move_node(id, a["parentId"].as_str())?;
                entity(&candidate, id)?
            }
            "create_link" => {
                let mut link: Link =
                    serde_json::from_value(a.get("link").cloned().unwrap_or(json!({})))?;
                link.source_node_id = required(a, "sourceNodeId")?.into();
                link.target_node_id = required(a, "targetNodeId")?.into();
                link.source_port_name = a["sourcePortName"].as_str().unwrap_or("out").into();
                link.target_port_name = a["targetPortName"].as_str().unwrap_or("in").into();
                let id = candidate.create_link(required(a, "name")?, link)?;
                entity(&candidate, &id)?
            }
            "update_link" => {
                let id = required(a, "nodeId")?;
                if candidate.node(id)?.link.is_none() {
                    return Err(Error::new("invalid_input", "Entity has no link data"));
                }
                candidate.patch(id, &json!({"link":a.get("patch").unwrap_or(a)}))?;
                entity(&candidate, id)?
            }
            "set_document_metadata" => {
                candidate.metadata = serde_json::from_value(a["metadata"].clone())?;
                json!({"metadata":candidate.metadata})
            }
            "register_user" => {
                let user = a["user"].clone();
                if !user.is_object() {
                    return Err(Error::new("invalid_input", "user must be an object"));
                }
                let key = user_key(&user);
                if let Some(i) = candidate.users.iter().position(|u| user_key(u) == key) {
                    candidate.users[i] = user.clone();
                } else {
                    candidate.users.push(user.clone());
                }
                json!({"user":user,"users":candidate.users})
            }
            "entities.patch" => {
                let changes = a["changes"]
                    .as_array()
                    .ok_or_else(|| Error::new("invalid_input", "changes must be an array"))?;
                for change in changes {
                    candidate.patch(required(change, "nodeId")?, &change["patch"])?;
                }
                json!({"updated":changes.len()})
            }
            _ => {
                return Err(Error::new(
                    "not_found",
                    format!("Unknown command '{}'", r.command),
                ));
            }
        };
        candidate.ensure_valid()?;
        if serde_json::to_value(&candidate)? != serde_json::to_value(&self.document)? {
            self.undo
                .push(std::mem::replace(&mut self.document, candidate));
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.revision += 1;
        }
        Ok(result)
    }
    fn check_discard(&self, a: &Value) -> Result<()> {
        if self.dirty() && !a["discardChanges"].as_bool().unwrap_or(false) {
            Err(Error::new(
                "unsaved_changes",
                "Save the project or explicitly pass discardChanges=true",
            ))
        } else {
            Ok(())
        }
    }
    fn reset_history(&mut self) {
        self.saved = serde_json::to_value(&self.document).unwrap();
        self.undo.clear();
        self.redo.clear();
        self.revision += 1;
    }
}
pub fn required<'a>(a: &'a Value, key: &str) -> Result<&'a str> {
    a[key]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            Error::new(
                "invalid_input",
                format!("'{key}' is required and must be a nonempty string"),
            )
        })
}
fn entity(d: &Document, id: &str) -> Result<Value> {
    Ok(
        json!({"entity":d.node(id)?,"fullyQualifiedName":d.qualified_name(id),"projectName":d.project_name()}),
    )
}
fn user_key(u: &Value) -> String {
    let designator = [
        "emailAddress",
        "username",
        "userId",
        "fullName",
        "identifier",
    ]
    .iter()
    .find_map(|k| u[k].as_str().map(str::trim).filter(|s| !s.is_empty()))
    .unwrap_or("local user");
    format!(
        "{}|{designator}|{}",
        u["source"].as_str().unwrap_or("").trim(),
        u["role"].as_str().unwrap_or("").trim()
    )
}
pub fn command_catalog() -> Value {
    json!({"commands":[{"id":"edit.undo","title":"Edit: Undo"},{"id":"edit.redo","title":"Edit: Redo"},{"id":"file.save","title":"File: Save"}]})
}
pub fn executable_exists(name: &str) -> bool {
    if Path::new(name).components().count() > 1 {
        return Path::new(name).is_file();
    }
    std::env::var_os("PATH")
        .is_some_and(|v| std::env::split_paths(&v).any(|p| p.join(name).is_file()))
}
