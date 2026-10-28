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
    undo: Vec<(Document, u64)>,
    redo: Vec<(Document, u64)>,
    state_id: u64,
    saved_state_id: u64,
    next_state_id: u64,
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
            state_id: 0,
            saved_state_id: 0,
            next_state_id: 1,
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
        let path = v["path"].as_str().map(PathBuf::from);
        if let Some(base) = path.as_deref().and_then(Path::parent) {
            wirecraft_core::components::resolve(&mut document, base);
        }
        let mut service = Self::new(document, path);
        service.saved = v["saved"].clone();
        if serde_json::to_value(&service.document)? != service.saved {
            service.saved_state_id = u64::MAX;
        }
        Ok(service)
    }
    pub fn dirty(&self) -> bool {
        self.state_id != self.saved_state_id
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
                    result["masterRevision"] = self.document.master_revision.clone();
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
                // Reference paths are relative to the file, which may have moved.
                if let Some(base) = path.parent() {
                    wirecraft_core::components::resolve(&mut self.document, base);
                }
                self.path = Some(path);
                self.saved = serde_json::to_value(&self.document)?;
                self.saved_state_id = self.state_id;
                return Ok(json!({"path":self.path,"saved":true}));
            }
            "edit.undo" => {
                let previous = self
                    .undo
                    .pop()
                    .ok_or_else(|| Error::new("unavailable", "Nothing to undo"))?;
                self.redo.push((
                    std::mem::replace(&mut self.document, previous.0),
                    self.state_id,
                ));
                self.state_id = previous.1;
                self.revision += 1;
                return Ok(serde_json::to_value(self.snapshot())?);
            }
            "edit.redo" => {
                let next = self
                    .redo
                    .pop()
                    .ok_or_else(|| Error::new("unavailable", "Nothing to redo"))?;
                self.undo
                    .push((std::mem::replace(&mut self.document, next.0), self.state_id));
                self.state_id = next.1;
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
            "compiler.templates" => {
                return if let Some(id) = a["templateSet"].as_str() {
                    Ok(serde_json::to_value(
                        wirecraft_core::templates::TemplateSet::builtin(id)?,
                    )?)
                } else {
                    Ok(wirecraft_core::templates::TemplateSet::catalog())
                };
            }
            "compiler.export" => {
                let bundle = wirecraft_core::compiler::export_bundle(
                    &self.document,
                    a["templateSet"].as_str(),
                )?;
                let project = wirecraft_core::generation::GeneratedProject {
                    name: self.document.project_name().into(),
                    capability: "portable-compiler-bundle".into(),
                    files: vec![wirecraft_core::generation::VirtualFile {
                        path: "compiler.wirecompiler.json".into(),
                        content: serde_json::to_string_pretty(&bundle)?,
                        binary_content: None,
                        source_map: vec![],
                    }],
                };
                if let Some(output) = a["outputDir"].as_str() {
                    crate::generation::write(&project, Path::new(output))?;
                }
                return Ok(serde_json::to_value(project)?);
            }
            "compile.project" => {
                let root = a["nodeId"].as_str().unwrap_or(&self.document.root_node_id);
                let options = serde_json::from_value(a.clone())?;
                let project = wirecraft_core::compiler::generate(&self.document, root, options)?;
                if let Some(output) = a["outputDir"].as_str() {
                    crate::generation::write(&project, Path::new(output))?;
                }
                return Ok(serde_json::to_value(project)?);
            }
            "compile.c17" => {
                let root = a["nodeId"].as_str().unwrap_or(&self.document.root_node_id);
                let project = wirecraft_core::c17::C17::generate(&self.document, root)?;
                if let Some(output) = a["outputDir"].as_str() {
                    crate::generation::write(&project, Path::new(output))?;
                }
                return Ok(serde_json::to_value(project)?);
            }
            "generate.files" => {
                let id = a["nodeId"].as_str().unwrap_or(&self.document.root_node_id);
                let project = wirecraft_core::generation::literal_files(&self.document, id)?;
                if let Some(output) = a["outputDir"].as_str() {
                    crate::generation::write(&project, Path::new(output))?;
                }
                return Ok(serde_json::to_value(project)?);
            }
            "archetypes.save" => {
                let id = required(a, "nodeId")?;
                let fragment =
                    wirecraft_core::fragment::Fragment::copy(&self.document, &[id.into()])?;
                let mut document = self.document.clone();
                document.root_node_id = id.into();
                document.nodes = fragment
                    .nodes
                    .into_iter()
                    .map(|n| (n.id.clone(), n))
                    .collect();
                document.repair()?;
                store::save(&document, Path::new(required(a, "path")?))?;
                return Ok(json!({"saved":true,"path":a["path"]}));
            }
            "entities.copy" => {
                let ids: Vec<String> = serde_json::from_value(a["nodeIds"].clone())?;
                return Ok(serde_json::to_value(
                    wirecraft_core::fragment::Fragment::copy(&self.document, &ids)?,
                )?);
            }
            "ai.prompt" => {
                let ids: Vec<String> = serde_json::from_value(a["nodeIds"].clone())?;
                return wirecraft_core::assistance::prompt(
                    &self.document,
                    &ids,
                    required(a, "task")?,
                );
            }
            "analysis.report" => return Ok(wirecraft_core::analysis::report(&self.document)),
            "archetypes.list" => return Ok(crate::archetypes::list()),
            "archetypes.preview" => {
                return Ok(
                    json!({"id":required(a,"id")?, "document":crate::archetypes::load(required(a,"id")?)?}),
                );
            }
            "export.markdown" => {
                let ids: Vec<String> = if let Some(ids) = a.get("nodeIds") {
                    serde_json::from_value(ids.clone())?
                } else {
                    self.document.nodes.keys().cloned().collect()
                };
                let markdown = wirecraft_core::export::markdown(&self.document, &ids)?;
                if let Some(path) = a["path"].as_str() {
                    store::atomic_write(Path::new(path), markdown.as_bytes())?;
                }
                return Ok(json!({"markdown":markdown}));
            }
            "export.preview" | "export.write" => {
                let parent = a["nodeId"].as_str().unwrap_or(&self.document.root_node_id);
                let documentation = a["kind"] == "documentation";
                let format = a["format"].as_str().unwrap_or("pdf");
                if r.command == "export.write"
                    && documentation
                    && ["html", "markdown"].contains(&format)
                {
                    let text = if format == "html" {
                        wirecraft_core::documentation::html(&self.document, parent)?
                    } else {
                        wirecraft_core::documentation::markdown_document(&self.document, parent)?
                    };
                    store::atomic_write(Path::new(required(a, "path")?), text.as_bytes())?;
                    return Ok(json!({"exported":true,"format":format,"path":a["path"]}));
                }
                if r.command == "export.write" && format == "mermaid" {
                    let text = wirecraft_core::export::mermaid(&self.document, parent)?;
                    store::atomic_write(Path::new(required(a, "path")?), text.as_bytes())?;
                    return Ok(json!({"exported":true,"path":a["path"]}));
                }
                let (pages, summary, overview) = if a["kind"] == "all" {
                    let sheet = serde_json::from_value(
                        a.get("sheetOptions").cloned().unwrap_or(json!({})),
                    )?;
                    let doc = serde_json::from_value(
                        a.get("documentOptions").cloned().unwrap_or(json!({})),
                    )?;
                    let mut plan = wirecraft_core::sheets::plan(&self.document, parent, &sheet)?;
                    let plan_pages = plan.pages.len();
                    let documentation =
                        wirecraft_core::documentation::pages(&self.document, parent, &doc)?;
                    let documentation_pages = documentation.len();
                    plan.pages.extend(documentation);
                    (
                        plan.pages,
                        json!({"kind":"all","planPages":plan_pages,"documentationPages":documentation_pages}),
                        None,
                    )
                } else if documentation {
                    let options =
                        serde_json::from_value(a.get("options").cloned().unwrap_or(json!({})))?;
                    (
                        wirecraft_core::documentation::pages(&self.document, parent, &options)?,
                        json!({"kind":"documentation"}),
                        None,
                    )
                } else {
                    let options =
                        serde_json::from_value(a.get("options").cloned().unwrap_or(json!({})))?;
                    let plan = wirecraft_core::sheets::plan(&self.document, parent, &options)?;
                    let summary = json!({"format":plan.format,"widthMm":plan.width_mm,"heightMm":plan.height_mm,"rows":plan.rows,"columns":plan.columns,"clipped":plan.clipped,"overviewWidthMm":plan.overview_width_mm,"overviewHeightMm":plan.overview_height_mm,"tiles":plan.tiles});
                    (plan.pages, summary, Some(plan.overview))
                };
                if r.command == "export.write" {
                    crate::export::pages_with_mode(
                        &pages,
                        Path::new(required(a, "path")?),
                        format,
                        a["rasterized"].as_bool().unwrap_or(false),
                    )?;
                    return Ok(
                        json!({"exported":true,"path":a["path"],"pages":pages.len(),"summary":summary}),
                    );
                }
                let page = a["page"].as_u64().unwrap_or(0) as usize;
                let is_overview = a["overview"] == true && overview.is_some();
                let svg = if is_overview {
                    overview.as_ref().unwrap()
                } else {
                    pages
                        .get(page)
                        .ok_or_else(|| Error::new("invalid_input", "Preview page out of range"))?
                };
                use base64::Engine;
                let png = base64::engine::general_purpose::STANDARD
                    .encode(crate::export::thumbnail(svg)?);
                return Ok(
                    json!({"pngBase64":png,"svg":svg,"page":page,"pages":pages.len(),"overview":is_overview,"summary":summary,"generation":a["generation"],"revision":self.revision}),
                );
            }
            "export.diagram" => {
                let parent = a["nodeId"].as_str().unwrap_or(&self.document.root_node_id);
                let path = Path::new(required(a, "path")?);
                let format = a["format"].as_str().unwrap_or("svg");
                crate::export::diagram(&self.document, parent, path, format)?;
                return Ok(
                    json!({"path":path,"format":format,"exported":true,"scope":"focused graph","sheet":"free-size"}),
                );
            }
            "list_users" => return Ok(json!({"users":self.document.users})),
            "validate_design" => {
                let mut diagnostics = self.document.validate();
                diagnostics.extend(wirecraft_core::components::diagnostics(&self.document));
                let valid = diagnostics.iter().all(|d| d.severity != "Error");
                return Ok(json!({"valid":valid,"diagnostics":diagnostics}));
            }
            "analyze_network" => return Ok(self.document.network_counts()),
            "list_technologies" => {
                let mut catalog = wirecraft_core::templates::TemplateSet::catalog();
                catalog["technologies"] = json!([
                    {"compilerId":"c-compiler","compilerName":"C17 graph compiler","languageId":"c","technologyId":"c-native"},
                    {"compilerId":"go-compiler","compilerName":"Go templates","languageId":"go","technologyId":"go"},
                    {"compilerId":"naive-kotlin","compilerName":"Kotlin templates","languageId":"kotlin","technologyId":"kotlin-jvm"},
                    {"compilerId":"nodejs-compiler","compilerName":"Node.js templates","languageId":"javascript","technologyId":"nodejs"},
                    {"compilerId":"php-compiler","compilerName":"PHP templates","languageId":"php","technologyId":"php"},
                    {"compilerId":"quickjs-compiler","compilerName":"QuickJS templates","languageId":"javascript","technologyId":"quickjs"},
                    {"compilerId":"generic","compilerName":"Model-defined templates","languageId":"any","technologyId":"compiler-template-set"},
                    {"compilerId":"multi-tech","compilerName":"Literal filesystem export","languageId":"any","technologyId":"file-export"},
                    {"compilerId":"multi-tech","compilerName":"Literal filesystem export","languageId":"any","technologyId":"multi-tech"}]);
                catalog["languageServers"] = json!([{"languageIds":["c","cpp"],"executable":"clangd","available":executable_exists("clangd")}]);
                catalog["note"] = json!(
                    "Template generation is separate from tool availability and editor intelligence. Representative Go/Node/Kotlin/PHP/QuickJS dataflows are tested; execution requires their external tools. C17 model-template overrides and source/runnable capabilities remain pending. See docs/migration/compiler-templates.md for verified scope."
                );
                return Ok(catalog);
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
            "entities.paste" => {
                let fragment: wirecraft_core::fragment::Fragment =
                    serde_json::from_value(a["fragment"].clone())?;
                let parent = a["parentId"]
                    .as_str()
                    .unwrap_or(&candidate.root_node_id)
                    .to_string();
                let ids = fragment.paste(
                    &mut candidate,
                    &parent,
                    [
                        a["offsetX"].as_f64().unwrap_or(30.),
                        a["offsetY"].as_f64().unwrap_or(30.),
                    ],
                )?;
                json!({"nodeIds":ids})
            }
            "entities.cut" => {
                let ids: Vec<String> = serde_json::from_value(a["nodeIds"].clone())?;
                let fragment = wirecraft_core::fragment::Fragment::copy(&candidate, &ids)?;
                for id in &fragment.roots {
                    candidate.delete(id)?;
                }
                serde_json::to_value(fragment)?
            }
            "archetypes.import" | "archetypes.import_file" => {
                let source = if command == "archetypes.import" {
                    crate::archetypes::load(required(a, "id")?)?
                } else {
                    store::load(Path::new(required(a, "path")?))?
                };
                let fragment = wirecraft_core::fragment::Fragment::from_document(&source)?;
                let parent = a["parentId"]
                    .as_str()
                    .unwrap_or(&candidate.root_node_id)
                    .to_string();
                let ids = fragment.paste(
                    &mut candidate,
                    &parent,
                    [
                        a["x"].as_f64().unwrap_or(40.),
                        a["y"].as_f64().unwrap_or(40.),
                    ],
                )?;
                json!({"nodeIds":ids,"imported":true})
            }
            "entities.align" => {
                let ids: Vec<String> = serde_json::from_value(a["nodeIds"].clone())?;
                let operation = required(a, "operation")?;
                let nodes = ids
                    .iter()
                    .map(|id| candidate.node(id).cloned())
                    .collect::<Result<Vec<_>>>()?
                    .into_iter()
                    .filter(|n| !n.is_link())
                    .collect::<Vec<_>>();
                if nodes.len() < 2 {
                    return Err(Error::new("invalid_input", "Select at least two boxes"));
                }
                if nodes.iter().any(|n| n.parent_id != nodes[0].parent_id) {
                    return Err(Error::new(
                        "invalid_input",
                        "Align boxes in the same parent",
                    ));
                }
                let aligned = nodes.iter().map(|n| n.id.clone()).collect::<Vec<_>>();
                // Align the projected boxes: simple boxes are auto-sized and an expanded
                // composite's contour follows its children, so stored sizes are not extents.
                let parent = nodes[0]
                    .parent_id
                    .clone()
                    .unwrap_or_else(|| candidate.root_node_id.clone());
                let drawing = wirecraft_core::drawing::Drawing::new(&candidate, &parent);
                let rects = nodes
                    .iter()
                    .map(|n| {
                        drawing.shapes.get(&n.id).map_or(
                            wirecraft_core::viewport::Rect {
                                x: n.layout.x,
                                y: n.layout.y,
                                width: n.layout.width,
                                height: n.layout.height,
                            },
                            |s| s.rect,
                        )
                    })
                    .collect::<Vec<_>>();
                let left = rects.iter().map(|r| r.x).fold(f64::INFINITY, f64::min);
                let top = rects.iter().map(|r| r.y).fold(f64::INFINITY, f64::min);
                let right = rects
                    .iter()
                    .map(|r| r.x + r.width)
                    .fold(f64::NEG_INFINITY, f64::max);
                let bottom = rects
                    .iter()
                    .map(|r| r.y + r.height)
                    .fold(f64::NEG_INFINITY, f64::max);
                let mut moves = vec![];
                if operation == "distribute-horizontal" || operation == "distribute-vertical" {
                    if nodes.len() < 3 {
                        return Err(Error::new(
                            "invalid_input",
                            "Distribution requires at least three boxes",
                        ));
                    }
                    let horizontal = operation == "distribute-horizontal";
                    let mut order = (0..nodes.len()).collect::<Vec<_>>();
                    order.sort_by(|a, b| {
                        if horizontal {
                            rects[*a].x.total_cmp(&rects[*b].x)
                        } else {
                            rects[*a].y.total_cmp(&rects[*b].y)
                        }
                    });
                    let extent = |r: &wirecraft_core::viewport::Rect| {
                        if horizontal { r.width } else { r.height }
                    };
                    let total = rects.iter().map(extent).sum::<f64>();
                    let gap = ((if horizontal {
                        right - left
                    } else {
                        bottom - top
                    }) - total)
                        / (nodes.len() - 1) as f64;
                    let mut cursor = if horizontal { left } else { top };
                    for i in order {
                        let r = rects[i];
                        moves.push(if horizontal {
                            (i, [cursor - r.x, 0.])
                        } else {
                            (i, [0., cursor - r.y])
                        });
                        cursor += extent(&r) + gap;
                    }
                } else {
                    for (i, r) in rects.iter().enumerate() {
                        moves.push((
                            i,
                            match operation {
                                "left" => [left - r.x, 0.],
                                "right" => [right - r.x - r.width, 0.],
                                "top" => [0., top - r.y],
                                "bottom" => [0., bottom - r.y - r.height],
                                _ => {
                                    return Err(Error::new(
                                        "invalid_input",
                                        "Unknown alignment operation",
                                    ));
                                }
                            },
                        ));
                    }
                }
                // Children use absolute coordinates, so a box moves with its whole subtree.
                for (i, [dx, dy]) in moves {
                    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
                        continue;
                    }
                    let mut stack = vec![nodes[i].id.clone()];
                    while let Some(id) = stack.pop() {
                        let n = candidate.node(&id)?.clone();
                        if !n.is_link() {
                            candidate.patch(
                                &id,
                                &json!({"layout":{"x":n.layout.x + dx,"y":n.layout.y + dy}}),
                            )?;
                        }
                        stack.extend(n.children);
                    }
                }
                json!({"aligned":aligned,"operation":operation})
            }
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
            "components.publish" => {
                // An Exterior and its single link are created together, so the design
                // never holds an Exterior without a name.
                let entity = required(a, "nodeId")?;
                candidate.node(entity)?;
                let output = match required(a, "direction")? {
                    "output" => true,
                    "input" => false,
                    _ => {
                        return Err(Error::new(
                            "invalid_input",
                            "direction must be 'output' or 'input'",
                        ));
                    }
                };
                let base = a["name"].as_str().unwrap_or("port").trim().to_string();
                let taken = |name: &str| {
                    candidate.nodes.values().any(|n| {
                        wirecraft_core::components::is_exterior(n)
                            && n.name == format!("{name}{}", if output { "Out" } else { "In" })
                    })
                };
                let mut name = base.clone();
                let mut suffix = 2;
                while taken(&name) {
                    name = format!("{base}{suffix}");
                    suffix += 1;
                }
                let parent = a["parentId"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&candidate.root_node_id)
                    .to_string();
                let exterior = candidate.create(&parent, &name, NodeKind::Node)?;
                let diameter = wirecraft_core::components::EXTERIOR_DIAMETER;
                candidate.patch(
                    &exterior,
                    &json!({"stereotype":wirecraft_core::components::EXTERIOR,"layout":{"x":a["x"].as_f64().unwrap_or(0.),"y":a["y"].as_f64().unwrap_or(0.),"width":diameter,"height":diameter}}),
                )?;
                let text = |key: &str| a[key].as_str().unwrap_or("").to_string();
                let mut link = Link {
                    type_name: text("typeName"),
                    type_definition_id: text("typeDefinitionId"),
                    payload_definition: text("payloadDefinition"),
                    ..Link::default()
                };
                (link.source_node_id, link.target_node_id) = if output {
                    (entity.to_string(), exterior.clone())
                } else {
                    (exterior.clone(), entity.to_string())
                };
                let link = candidate.create_link(&name, link)?;
                json!({"exteriorId":exterior,"linkId":link})
            }
            "links.reconnect" => {
                // Moves one end of a link; without entityId the end becomes a new
                // Exterior at x/y in parentId, publishing a port like Link mode does.
                let id = required(a, "nodeId")?;
                let source = match required(a, "end")? {
                    "source" => true,
                    "target" => false,
                    _ => {
                        return Err(Error::new(
                            "invalid_input",
                            "end must be 'source' or 'target'",
                        ));
                    }
                };
                let entity = match a["entityId"].as_str().filter(|s| !s.is_empty()) {
                    Some(entity) => entity.to_string(),
                    None => {
                        let parent = a["parentId"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&candidate.root_node_id)
                            .to_string();
                        let exterior = candidate.create(&parent, "port", NodeKind::Node)?;
                        let diameter = wirecraft_core::components::EXTERIOR_DIAMETER;
                        candidate.patch(
                            &exterior,
                            &json!({"stereotype":wirecraft_core::components::EXTERIOR,"layout":{"x":a["x"].as_f64().unwrap_or(0.),"y":a["y"].as_f64().unwrap_or(0.),"width":diameter,"height":diameter}}),
                        )?;
                        exterior
                    }
                };
                candidate.reconnect(id, source, &entity)?;
                json!({"linkId":id,"entityId":entity})
            }
            "components.reference" => {
                let base = self
                    .path
                    .as_deref()
                    .and_then(Path::parent)
                    .ok_or_else(|| {
                        Error::new(
                            "invalid_input",
                            "Save the design first: a reference path is relative to its file",
                        )
                    })?
                    .to_path_buf();
                let requested = PathBuf::from(required(a, "path")?);
                let relative = if requested.is_absolute() {
                    let canonical = requested.canonicalize()?;
                    canonical
                        .strip_prefix(base.canonicalize()?)
                        .map_err(|_| {
                            Error::new(
                                "invalid_input",
                                "A referenced component must be inside the design's directory",
                            )
                        })?
                        .to_path_buf()
                } else {
                    requested
                };
                let parent = a["parentId"]
                    .as_str()
                    .unwrap_or(&candidate.root_node_id)
                    .to_string();
                let id = candidate.create(&parent, "component", NodeKind::Node)?;
                candidate.patch(
                    &id,
                    &json!({"stereotype":wirecraft_core::components::REFERENCE,"reference":{"path":relative.to_string_lossy()},"layout":{"x":a["x"].as_f64().unwrap_or(0.),"y":a["y"].as_f64().unwrap_or(0.)}}),
                )?;
                wirecraft_core::components::resolve(&mut candidate, &base);
                let pinned = wirecraft_core::components::pin(&mut candidate, &id)?;
                let name = a["name"].as_str().unwrap_or(&pinned.name).to_string();
                candidate.patch(&id, &json!({"name":name}))?;
                json!({"nodeId":id,"reference":pinned})
            }
            "components.update_reference" => {
                let id = required(a, "nodeId")?;
                let before = wirecraft_core::components::reference(candidate.node(id)?)
                    .ok_or_else(|| Error::new("invalid_input", "Entity is not a Reference"))?;
                if let Some(base) = self.path.as_deref().and_then(Path::parent) {
                    wirecraft_core::components::resolve(&mut candidate, base);
                }
                let after = wirecraft_core::components::pin(&mut candidate, id)?;
                let removed: Vec<_> = before
                    .ports
                    .iter()
                    .filter(|p| !after.ports.contains(p))
                    .collect();
                let added: Vec<_> = after
                    .ports
                    .iter()
                    .filter(|p| !before.ports.contains(p))
                    .collect();
                json!({"nodeId":id,"reference":after,"removedPorts":removed,"addedPorts":added})
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
        wirecraft_core::components::normalize(&mut candidate)?;
        candidate.ensure_valid()?;
        if serde_json::to_value(&candidate)? != serde_json::to_value(&self.document)? {
            self.undo.push((
                std::mem::replace(&mut self.document, candidate),
                self.state_id,
            ));
            self.state_id = self.next_state_id;
            self.next_state_id += 1;
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
        self.state_id = self.next_state_id;
        self.next_state_id += 1;
        self.saved_state_id = self.state_id;
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
        json!({"entity":d.node(id)?,"fullyQualifiedName":d.qualified_name(id),"projectName":d.project_name(),"masterRevision":d.master_revision,"effectiveRevision":d.effective_revision(id)}),
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
