use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub type Extras = BTreeMap<String, Value>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, thiserror::Error, Serialize)]
#[error("{message}")]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
impl Error {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::new("invalid_input", e.to_string())
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::new("io", e.to_string())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Node,
    #[default]
    Processor,
    Link,
    Group,
    Type,
    Note,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Layout {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub closed_width: f64,
    pub closed_height: f64,
    pub open_width: f64,
    pub open_height: f64,
    pub is_expanded: bool,
    #[serde(flatten)]
    pub extra: Extras,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            x: 0.,
            y: 0.,
            width: 200.,
            height: 70.,
            closed_width: 200.,
            closed_height: 70.,
            open_width: 200.,
            open_height: 70.,
            is_expanded: true,
            extra: Extras::new(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NodeText {
    #[serde(alias = "declaration")]
    pub source: String,
    #[serde(alias = "declarationLanguageId")]
    pub source_language_id: String,
    #[serde(alias = "instantiation")]
    pub initialization: String,
    #[serde(alias = "instantiationLanguageId")]
    pub initialization_language_id: String,
    pub specification: String,
    pub specification_language_id: String,
    pub tests: String,
    pub tests_language_id: String,
    pub ai_instructions: String,
    pub ai_instructions_language_id: String,
    #[serde(flatten)]
    pub extra: Extras,
}
impl Default for NodeText {
    fn default() -> Self {
        Self {
            source: String::new(),
            source_language_id: String::new(),
            initialization: String::new(),
            initialization_language_id: String::new(),
            specification: String::new(),
            specification_language_id: "markdown".into(),
            tests: String::new(),
            tests_language_id: "json".into(),
            ai_instructions: String::new(),
            ai_instructions_language_id: "markdown".into(),
            extra: Extras::new(),
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Technology {
    pub language_id: String,
    pub technology_id: String,
    pub compiler_id: String,
    pub file_extension: String,
    pub content_type: String,
    #[serde(flatten)]
    pub extra: Extras,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Input,
    Output,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Port {
    pub id: String,
    pub name: String,
    pub direction: Direction,
    #[serde(default)]
    pub data_type: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(flatten)]
    pub extra: Extras,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Link {
    pub source_node_id: String,
    pub source_port_name: String,
    pub target_node_id: String,
    pub target_port_name: String,
    pub transport_kind: String,
    pub interaction_kind: String,
    pub type_name: String,
    pub payload_definition: String,
    pub type_definition_id: String,
    pub composite_boundary_ids: Vec<String>,
    pub type_expression: Option<Value>,
    #[serde(flatten)]
    pub extra: Extras,
}
impl Default for Link {
    fn default() -> Self {
        Self {
            source_node_id: String::new(),
            source_port_name: "out".into(),
            target_node_id: String::new(),
            target_port_name: "in".into(),
            transport_kind: "in-process".into(),
            interaction_kind: "auto".into(),
            type_name: String::new(),
            payload_definition: String::new(),
            type_definition_id: String::new(),
            composite_boundary_ids: vec![],
            type_expression: None,
            extra: Extras::new(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default)]
    pub incoming_links: Vec<String>,
    #[serde(default)]
    pub outgoing_links: Vec<String>,
    #[serde(default)]
    pub layout: Layout,
    #[serde(default = "none")]
    pub file_layout_strategy_id: String,
    #[serde(default)]
    pub text: NodeText,
    #[serde(default)]
    pub binary_content: Option<String>,
    #[serde(default)]
    pub technology: Technology,
    #[serde(default)]
    pub ports: Vec<Port>,
    #[serde(default)]
    pub link: Option<Link>,
    #[serde(default)]
    pub type_definition: Option<Value>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(default)]
    pub plugin_data: Extras,
    #[serde(default)]
    pub revision: Option<Value>,
    #[serde(default)]
    pub responsible: Option<String>,
    #[serde(default = "modified_default")]
    pub modified: Value,
    #[serde(default)]
    pub diagnostics: Vec<Value>,
    #[serde(default)]
    pub name_detail: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub status_changes: Vec<Value>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(flatten)]
    pub extra: Extras,
}
fn none() -> String {
    "none".into()
}
fn modified_default() -> Value {
    json!({"date":"","user":""})
}
impl Node {
    pub fn new(id: String, name: String, kind: NodeKind, parent_id: Option<String>) -> Self {
        Self {
            id,
            name,
            kind,
            parent_id,
            children: vec![],
            incoming_links: vec![],
            outgoing_links: vec![],
            layout: Layout::default(),
            file_layout_strategy_id: none(),
            text: NodeText::default(),
            binary_content: None,
            technology: Technology::default(),
            ports: vec![],
            link: None,
            type_definition: (kind == NodeKind::Type)
                .then(|| json!({"fields":[],"qualifier":"object","genericTypeIds":[]})),
            metadata: BTreeMap::new(),
            plugin_data: Extras::new(),
            revision: None,
            responsible: None,
            modified: modified_default(),
            diagnostics: vec![],
            name_detail: String::new(),
            status: None,
            status_changes: vec![],
            assignee: None,
            extra: Extras::new(),
        }
    }
    pub fn is_link(&self) -> bool {
        self.kind == NodeKind::Link || self.link.is_some()
    }
    pub fn is_composite(&self) -> bool {
        self.kind == NodeKind::Group || !self.children.is_empty()
    }
    fn ensure_port(&mut self, name: &str, direction: Direction) {
        if !self
            .ports
            .iter()
            .any(|p| p.name == name && p.direction == direction)
        {
            self.ports.push(Port {
                id: Uuid::new_v4().to_string(),
                name: name.into(),
                direction,
                data_type: String::new(),
                metadata: BTreeMap::new(),
                extra: Extras::new(),
            });
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: String,
    pub name: String,
    pub root_node_id: String,
    pub nodes: BTreeMap<String, Node>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(default = "revision_default")]
    pub master_revision: Value,
    #[serde(default)]
    pub users: Vec<Value>,
    #[serde(flatten)]
    pub extra: Extras,
}
fn revision_default() -> Value {
    json!({"name":"","date":""})
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: String,
    pub message: String,
    pub node_id: Option<String>,
}
impl Document {
    pub fn new(name: &str) -> Self {
        let root = Node::new("root".into(), name.into(), NodeKind::Processor, None);
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            root_node_id: root.id.clone(),
            nodes: BTreeMap::from([(root.id.clone(), root)]),
            metadata: BTreeMap::new(),
            master_revision: revision_default(),
            users: vec![],
            extra: Extras::new(),
        }
    }
    pub fn node(&self, id: &str) -> Result<&Node> {
        self.nodes
            .get(id)
            .ok_or_else(|| Error::new("not_found", format!("Entity '{id}' does not exist")))
    }
    pub fn project_name(&self) -> &str {
        self.nodes
            .get(&self.root_node_id)
            .map(|n| n.name.as_str())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&self.name)
    }
    pub fn ancestors(&self, id: &str) -> Vec<String> {
        let mut ids = vec![];
        let mut seen = BTreeSet::new();
        let mut current = Some(id.to_owned());
        while let Some(id) = current {
            if !seen.insert(id.clone()) {
                break;
            }
            current = self.nodes.get(&id).and_then(|n| n.parent_id.clone());
            ids.push(id);
        }
        ids
    }
    pub fn qualified_name(&self, id: &str) -> String {
        self.ancestors(id)
            .iter()
            .rev()
            .filter_map(|id| self.nodes.get(id))
            .map(|n| n.name.as_str())
            .collect::<Vec<_>>()
            .join("/")
    }
    pub fn descendants(&self, id: &str) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        let mut stack = vec![id.to_string()];
        while let Some(id) = stack.pop() {
            if found.insert(id.clone())
                && let Some(n) = self.nodes.get(&id)
            {
                stack.extend(n.children.clone());
            }
        }
        found
    }
    pub fn effective(&self, id: &str, field: &str) -> String {
        if let Some(node) = self.nodes.get(id) {
            let export = matches!(field, "languageId" | "fileLayoutStrategyId")
                && ["file-export", "multi-tech"]
                    .contains(&self.effective(id, "technologyId").as_str());
            if field == "languageId" && export && node.technology.language_id.is_empty() {
                return "plain".into();
            }
            if field == "fileLayoutStrategyId"
                && export
                && ["", "none"].contains(&node.file_layout_strategy_id.as_str())
            {
                return "none".into();
            }
        }
        for id in self.ancestors(id) {
            if let Some(n) = self.nodes.get(&id) {
                let v = match field {
                    "languageId" => &n.technology.language_id,
                    "technologyId" => &n.technology.technology_id,
                    "compilerId" => &n.technology.compiler_id,
                    "fileLayoutStrategyId" => &n.file_layout_strategy_id,
                    "responsible" => n.responsible.as_deref().unwrap_or(""),
                    _ => "",
                };
                if !v.is_empty() && v != "none" {
                    return v.into();
                }
            }
        }
        if field == "languageId" {
            "plain".into()
        } else {
            none()
        }
    }
    pub fn effective_text_language(&self, id: &str, section: &str) -> String {
        let key = match section {
            "source" => "sourceLanguageId",
            "initialization" => "initializationLanguageId",
            "specification" => "specificationLanguageId",
            "tests" => "testsLanguageId",
            _ => "aiInstructionsLanguageId",
        };
        for current in self.ancestors(id) {
            if let Some(node) = self.nodes.get(&current) {
                let text = serde_json::to_value(&node.text).unwrap();
                let language = text[key].as_str().unwrap_or("").trim();
                if !language.is_empty() {
                    return language.into();
                }
                if !["source", "initialization"].contains(&section) {
                    return if section == "tests" {
                        "json".into()
                    } else {
                        "markdown".into()
                    };
                }
            }
        }
        self.effective(id, "languageId")
    }
    pub fn common_parent(&self, a: &str, b: &str) -> String {
        if a == b {
            return self
                .nodes
                .get(a)
                .and_then(|n| n.parent_id.clone())
                .unwrap_or(self.root_node_id.clone());
        }
        let chain = self.ancestors(a);
        self.ancestors(b)
            .into_iter()
            .find(|id| chain.contains(id))
            .unwrap_or(self.root_node_id.clone())
    }
    pub fn boundaries(&self, a: &str, b: &str) -> Vec<String> {
        let common = self.common_parent(a, b);
        let chain = |id: &str| {
            self.ancestors(id)
                .into_iter()
                .skip(1)
                .take_while(|id| id != &common)
                .filter(|id| {
                    id != &self.root_node_id
                        && self.nodes.get(id).is_some_and(|n| {
                            n.kind == NodeKind::Group
                                || n.children
                                    .iter()
                                    .any(|c| self.nodes.get(c).is_some_and(|x| !x.is_link()))
                        })
                })
                .collect::<Vec<_>>()
        };
        let mut ids = chain(a);
        ids.extend(chain(b).into_iter().rev());
        let mut seen = BTreeSet::new();
        ids.retain(|id| seen.insert(id.clone()));
        ids
    }
    /// Rebuild derived topology while preserving sibling order and unknown payloads.
    pub fn repair(&mut self) -> Result<()> {
        self.node(&self.root_node_id)?;
        let old = self.nodes.clone();
        let legacy: Vec<_> = old
            .values()
            .filter(|n| {
                n.link.as_ref().is_some_and(|l| {
                    [&l.source_node_id, &l.target_node_id]
                        .iter()
                        .any(|id| old.get(*id).is_some_and(Node::is_link))
                })
            })
            .map(|n| n.id.clone())
            .collect();
        for id in legacy {
            let n = self.nodes.get_mut(&id).unwrap();
            let l = n.link.take().unwrap();
            for (key, value) in [
                ("SourceId", l.source_node_id),
                ("TargetId", l.target_node_id),
                ("SourcePort", l.source_port_name.clone()),
                ("TargetPort", l.target_port_name.clone()),
                ("TransportKind", l.transport_kind),
                ("TypeDefinitionId", l.type_definition_id),
                ("TypeName", l.type_name),
                ("PayloadDefinition", l.payload_definition),
                ("BoundaryIds", l.composite_boundary_ids.join(",")),
            ] {
                n.metadata
                    .entry(format!("threadwork.migratedLink{key}"))
                    .or_insert(value);
            }
            n.ensure_port(&l.source_port_name, Direction::Input);
            n.ensure_port(&l.target_port_name, Direction::Output);
            n.kind = NodeKind::Processor;
        }
        let mut inferred = BTreeMap::new();
        for n in old.values() {
            for child in &n.children {
                inferred.entry(child.clone()).or_insert(n.id.clone());
            }
        }
        for n in self.nodes.values_mut() {
            if n.id == self.root_node_id {
                n.parent_id = None;
            } else if !n
                .parent_id
                .as_ref()
                .is_some_and(|p| p != &n.id && old.contains_key(p))
            {
                n.parent_id = Some(
                    inferred
                        .get(&n.id)
                        .filter(|p| *p != &n.id)
                        .cloned()
                        .unwrap_or(self.root_node_id.clone()),
                );
            }
            n.children.clear();
            n.incoming_links.clear();
            n.outgoing_links.clear();
        }
        self.check_cycles()?;
        // Old sibling order first; disconnected or repaired nodes follow in stable ID order.
        for p in old.values() {
            for c in &p.children {
                if self
                    .nodes
                    .get(c)
                    .is_some_and(|n| n.parent_id.as_ref() == Some(&p.id))
                {
                    let v = &mut self.nodes.get_mut(&p.id).unwrap().children;
                    if !v.contains(c) {
                        v.push(c.clone());
                    }
                }
            }
        }
        let parents: Vec<_> = self
            .nodes
            .values()
            .filter_map(|n| n.parent_id.clone().map(|p| (n.id.clone(), p)))
            .collect();
        for (id, p) in parents {
            let v = &mut self.nodes.get_mut(&p).unwrap().children;
            if !v.contains(&id) {
                v.push(id);
            }
        }
        self.sync_links()?;
        self.name = self.project_name().into();
        Ok(())
    }
    fn check_cycles(&self) -> Result<()> {
        let mut complete = BTreeSet::new();
        for start in self.nodes.keys() {
            let mut seen = BTreeSet::new();
            let mut current = Some(start.as_str());
            while let Some(id) = current {
                if complete.contains(id) {
                    break;
                }
                if !seen.insert(id.to_string()) {
                    return Err(Error::new(
                        "invalid_graph",
                        format!("Hierarchy cycle at '{id}'"),
                    ));
                }
                current = self.nodes.get(id).and_then(|n| n.parent_id.as_deref());
            }
            complete.extend(seen);
        }
        Ok(())
    }
    pub fn sync_links(&mut self) -> Result<()> {
        for n in self.nodes.values_mut() {
            n.incoming_links.clear();
            n.outgoing_links.clear();
        }
        let links: Vec<_> = self
            .nodes
            .values()
            .filter_map(|n| n.link.clone().map(|l| (n.id.clone(), l)))
            .collect();
        for (id, mut l) in links {
            for endpoint in [&l.source_node_id, &l.target_node_id] {
                if self.node(endpoint)?.is_link() {
                    return Err(Error::new(
                        "invalid_graph",
                        "A link cannot use another link as endpoint",
                    ));
                }
            }
            if l.source_port_name.is_empty() {
                l.source_port_name = "out".into();
            }
            if l.target_port_name.is_empty() {
                l.target_port_name = "in".into();
            }
            l.interaction_kind = match l.interaction_kind.trim().to_lowercase().as_str() {
                "automatic" => "auto",
                "transport" => "data",
                "library" => "lib",
                "source" => "src",
                "runnable" => "run",
                s => s,
            }
            .to_string();
            let p = self.common_parent(&l.source_node_id, &l.target_node_id);
            l.composite_boundary_ids = self.boundaries(&l.source_node_id, &l.target_node_id);
            let previous = self.nodes[&id].parent_id.clone();
            if previous.as_ref() != Some(&p) {
                if let Some(n) = previous.and_then(|p| self.nodes.get_mut(&p)) {
                    n.children.retain(|c| c != &id);
                }
                let parent = self.nodes.get_mut(&p).unwrap();
                if !parent.children.contains(&id) {
                    parent.children.push(id.clone());
                }
                self.nodes.get_mut(&id).unwrap().parent_id = Some(p);
            }
            let src = self.nodes.get_mut(&l.source_node_id).unwrap();
            src.ensure_port(&l.source_port_name, Direction::Output);
            src.outgoing_links.push(id.clone());
            let dst = self.nodes.get_mut(&l.target_node_id).unwrap();
            dst.ensure_port(&l.target_port_name, Direction::Input);
            dst.incoming_links.push(id.clone());
            self.nodes.get_mut(&id).unwrap().link = Some(l);
        }
        Ok(())
    }
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut errors = vec![];
        let mut add = |id: Option<&str>, message: String| {
            errors.push(Diagnostic {
                severity: "Error".into(),
                message,
                node_id: id.map(str::to_string),
            })
        };
        if !self.nodes.contains_key(&self.root_node_id) {
            add(Some(&self.root_node_id), "Root node does not exist".into());
        }
        if let Err(e) = self.check_cycles() {
            add(None, e.message);
        }
        for (key, n) in &self.nodes {
            if key != &n.id {
                add(Some(key), "Node key differs from ID".into());
            }
            if n.id == self.root_node_id && n.parent_id.is_some() {
                add(Some(key), "Root cannot have a parent".into());
            }
            if let Some(p) = &n.parent_id
                && !self.nodes.get(p).is_some_and(|p| p.children.contains(key))
            {
                add(Some(key), format!("Parent '{p}' does not contain child"));
            }
            let mut children = BTreeSet::new();
            for c in &n.children {
                if !children.insert(c)
                    || !self
                        .nodes
                        .get(c)
                        .is_some_and(|c| c.parent_id.as_ref() == Some(key))
                {
                    add(Some(key), format!("Invalid or duplicate child '{c}'"));
                }
            }
            let r = &n.layout;
            if [
                r.x,
                r.y,
                r.width,
                r.height,
                r.open_width,
                r.open_height,
                r.closed_width,
                r.closed_height,
            ]
            .iter()
            .any(|v| !v.is_finite())
                || r.width <= 0.
                || r.height <= 0.
            {
                add(Some(key), "Invalid layout geometry".into());
            }
            if let Some(binary) = &n.binary_content {
                use base64::Engine;
                if base64::engine::general_purpose::STANDARD
                    .decode(binary)
                    .is_err()
                {
                    add(Some(key), "Invalid Base64 binary content".into());
                }
            }
            let mut port_names = BTreeSet::new();
            let mut ports = BTreeSet::new();
            for p in &n.ports {
                if p.name.trim().is_empty()
                    || !ports.insert(&p.id)
                    || !port_names.insert((format!("{:?}", p.direction), p.name.clone()))
                {
                    add(Some(key), "Blank port name or duplicate port ID".into());
                }
            }
            if n.is_link() && n.link.is_none() {
                add(Some(key), "Link has no link data".into());
            }
            if let Some(l) = &n.link {
                let type_usage = self
                    .nodes
                    .get(&l.source_node_id)
                    .is_some_and(|n| n.kind == NodeKind::Type)
                    && ["auto", "type-usage"].contains(&l.interaction_kind.as_str());
                if !["auto", "data", "lib", "src", "run", "type-usage"]
                    .contains(&l.interaction_kind.as_str())
                {
                    add(Some(key), "Unknown interaction kind".into());
                }
                for (endpoint, port, direction) in [
                    (&l.source_node_id, &l.source_port_name, Direction::Output),
                    (&l.target_node_id, &l.target_port_name, Direction::Input),
                ] {
                    match self.nodes.get(endpoint) {
                        None => add(Some(key), format!("Missing link endpoint '{endpoint}'")),
                        Some(e) if e.is_link() => {
                            add(Some(key), "Link endpoint cannot be a link".into())
                        }
                        Some(e) if e.kind == NodeKind::Type && !type_usage => add(
                            Some(key),
                            "Type endpoint requires type-usage interaction".into(),
                        ),
                        Some(e)
                            if e.kind != NodeKind::Type
                                && !e
                                    .ports
                                    .iter()
                                    .any(|p| &p.name == port && p.direction == direction) =>
                        {
                            add(Some(key), format!("Missing endpoint port '{port}'"))
                        }
                        _ => {}
                    }
                }
                if n.parent_id.as_ref()
                    != Some(&self.common_parent(&l.source_node_id, &l.target_node_id))
                {
                    add(Some(key), "Link has incorrect common parent".into());
                }
            }
            if n.kind == NodeKind::Type {
                if let Some(t) = &n.type_definition {
                    let qualifier = t["qualifier"]
                        .as_str()
                        .unwrap_or("object")
                        .trim()
                        .to_lowercase();
                    let arity = match qualifier.as_str() {
                        "object" => Some(0),
                        "array" | "list" => Some(1),
                        "map" => Some(2),
                        _ => None,
                    };
                    let generic = t["genericTypeIds"].as_array();
                    if !t.is_object()
                        || arity.is_none()
                        || generic.map_or(0, Vec::len) != arity.unwrap_or(0)
                    {
                        add(Some(key), "Invalid type qualifier or generic arity".into());
                    }
                    if generic
                        .into_iter()
                        .flatten()
                        .any(|id| id.as_str().is_none_or(|s| s.trim().is_empty()))
                    {
                        add(Some(key), "Generic type ID cannot be blank".into());
                    }
                    let mut names = BTreeSet::new();
                    for f in t["fields"].as_array().into_iter().flatten() {
                        let name = f["name"].as_str().unwrap_or("").trim();
                        if name.is_empty() || !names.insert(name) {
                            add(Some(key), "Blank or duplicate type field name".into());
                        }
                        if f.get("typeExpression").is_none_or(Value::is_null) {
                            let id = f["typeId"].as_str().unwrap_or("string");
                            if id.trim().is_empty()
                                || self.nodes.get(id).is_some_and(|n| n.kind != NodeKind::Type)
                            {
                                add(Some(key), "Invalid type field reference".into());
                            }
                        }
                    }
                } else {
                    add(Some(key), "Type entity has no type definition".into());
                }
            }
        }
        errors
    }
    pub fn ensure_valid(&self) -> Result<()> {
        let errors = self.validate();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::new(
                "invalid_graph",
                errors
                    .iter()
                    .map(|e| e.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
            ))
        }
    }
    pub fn create(&mut self, parent: &str, name: &str, kind: NodeKind) -> Result<String> {
        self.node(parent)?;
        if kind == NodeKind::Link {
            return Err(Error::new("invalid_input", "Use create_link for links"));
        }
        let id = Uuid::new_v4().to_string();
        let mut n = Node::new(id.clone(), name.into(), kind, Some(parent.into()));
        n.status = Some("BUSINESS".into());
        n.status_changes.push(json!({"userId":"local","changedDate":timestamp(),"initStatus":null,"endStatus":"BUSINESS"}));
        n.modified = json!({"date":timestamp(),"user":"local"});
        self.nodes.insert(id.clone(), n);
        self.nodes
            .get_mut(parent)
            .unwrap()
            .children
            .push(id.clone());
        Ok(id)
    }
    pub fn create_link(&mut self, name: &str, link: Link) -> Result<String> {
        self.node(&link.source_node_id)?;
        self.node(&link.target_node_id)?;
        let id = Uuid::new_v4().to_string();
        let p = self.common_parent(&link.source_node_id, &link.target_node_id);
        let mut n = Node::new(id.clone(), name.into(), NodeKind::Link, Some(p.clone()));
        n.link = Some(link);
        self.nodes.insert(id.clone(), n);
        self.nodes.get_mut(&p).unwrap().children.push(id.clone());
        self.sync_links()?;
        Ok(id)
    }
    pub fn delete(&mut self, id: &str) -> Result<()> {
        self.node(id)?;
        if id == self.root_node_id {
            return Err(Error::new("invalid_input", "Cannot delete root node"));
        }
        let mut ids = self.descendants(id);
        for n in self.nodes.values() {
            if n.link
                .as_ref()
                .is_some_and(|l| ids.contains(&l.source_node_id) || ids.contains(&l.target_node_id))
            {
                ids.insert(n.id.clone());
            }
        }
        self.nodes.retain(|id, _| !ids.contains(id));
        for n in self.nodes.values_mut() {
            n.children.retain(|c| !ids.contains(c));
        }
        self.sync_links()
    }
    pub fn move_node(&mut self, id: &str, parent: Option<&str>) -> Result<()> {
        self.node(id)?;
        let parent = parent.unwrap_or(&self.root_node_id).to_string();
        self.node(&parent)?;
        if id == self.root_node_id || self.descendants(id).contains(&parent) {
            return Err(Error::new(
                "invalid_graph",
                "Cannot move root or create a hierarchy cycle",
            ));
        }
        if let Some(p) = self.nodes[id].parent_id.clone() {
            self.nodes.get_mut(&p).unwrap().children.retain(|c| c != id);
        }
        self.nodes.get_mut(id).unwrap().parent_id = Some(parent.clone());
        self.nodes
            .get_mut(&parent)
            .unwrap()
            .children
            .push(id.into());
        self.sync_links()
    }
    pub fn patch(&mut self, id: &str, patch: &Value) -> Result<()> {
        let current = self.node(id)?.clone();
        let obj = patch
            .as_object()
            .ok_or_else(|| Error::new("invalid_input", "Patch must be an object"))?;
        for key in [
            "id",
            "parentId",
            "children",
            "incomingLinks",
            "outgoingLinks",
        ] {
            if obj.contains_key(key) {
                return Err(Error::new(
                    "invalid_input",
                    format!("'{key}' is structural; use a repository command"),
                ));
            }
        }
        let mut value = serde_json::to_value(&current)?;
        for (k, v) in obj {
            // Text/layout/technology/link patch objects merge; metadata and collections replace.
            if ["text", "layout", "technology", "link"].contains(&k.as_str()) && v.is_object() {
                if value[k].is_null() {
                    value[k] = json!({});
                }
                for (field, v) in v.as_object().unwrap() {
                    let field = if k == "text" {
                        match field.as_str() {
                            "declaration" => "source",
                            "instantiation" => "initialization",
                            "declarationLanguageId" => "sourceLanguageId",
                            "instantiationLanguageId" => "initializationLanguageId",
                            s => s,
                        }
                    } else {
                        field
                    };
                    value[k][field] = v.clone();
                }
            } else {
                value[k] = v.clone();
            }
        }
        let mut n: Node = serde_json::from_value(value)?;
        if current.link.is_some() && (n.kind != NodeKind::Link || n.link.is_none()) {
            return Err(Error::new(
                "invalid_input",
                "Cannot remove link identity via patch",
            ));
        }
        if n.kind == NodeKind::Type && n.type_definition.is_none() {
            n.type_definition = Some(json!({"fields":[],"qualifier":"object","genericTypeIds":[]}));
        }
        if let Some(status) = &n.status
            && ![
                "BUSINESS",
                "FUNCTIONAL",
                "IMPLEMENTATION",
                "TESTING",
                "DEPLOYED",
                "CANCELED",
            ]
            .contains(&status.as_str())
        {
            return Err(Error::new("invalid_input", "Unknown project status"));
        }
        if current.status != n.status {
            n.status_changes.push(json!({"userId":"local","changedDate":timestamp(),"initStatus":current.status,"endStatus":n.status}));
        }
        n.modified = json!({"date":timestamp(),"user":"local"});
        self.nodes.insert(id.into(), n);
        self.name = self.project_name().into();
        self.sync_links()
    }
    pub fn network_counts(&self) -> Value {
        json!({"entities":self.nodes.values().filter(|n|!n.is_link()).count(),"links":self.nodes.values().filter(|n|n.is_link()).count(),"types":self.nodes.values().filter(|n|n.kind==NodeKind::Type).count(),"composites":self.nodes.values().filter(|n|n.is_composite()&&!n.is_link()).count(),"ports":self.nodes.values().map(|n|n.ports.len()).sum::<usize>(),"users":self.users.len(),"projectName":self.project_name()})
    }
}
pub fn timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
