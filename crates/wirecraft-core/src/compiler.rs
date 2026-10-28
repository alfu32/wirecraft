//! Graph planning and artifact assembly for the portable compiler template sets.
//! Language syntax lives in the retained assets; ownership and paths live here.
use crate::{
    Document, Error, Node, NodeKind, Result,
    analysis::{Relation, relation, stereotype},
    generation::{GeneratedProject, SourceMap, VirtualFile, safe_relative_path},
    templates::{TemplateSet, render},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileOptions {
    pub template_set: Option<String>,
    pub layout: Option<String>,
    pub bundle: Option<TemplateSet>,
}
#[derive(Clone)]
struct Artifact {
    context: Value,
    declaration: String,
    initialization: String,
    hoisted: Vec<String>,
    forward: Vec<String>,
    files: Vec<VirtualFile>,
    path: String,
    layout: String,
}
struct Plan<'a> {
    d: &'a Document,
    root: &'a str,
    set: TemplateSet,
    options: CompileOptions,
    symbols: BTreeMap<String, String>,
    artifacts: BTreeMap<String, Artifact>,
    visiting: BTreeSet<String>,
}
/// Resolve the same technology identifiers used by stored Threadworks projects.
pub fn template_set_id(technology: &str) -> &'static str {
    match technology {
        "c-native" => "c",
        "go" => "go",
        "kotlin-jvm" => "kotlin",
        "nodejs" => "nodejs",
        "php" => "php",
        // Stored Threadworks projects may still name the systemJS set by its former name.
        "systemjs" | "quickjs" => "systemjs",
        "rust" => "rust",
        "compiler-template-set" | "compiler-compiler" => "generic",
        _ => "literal",
    }
}
pub fn export_bundle(d: &Document, id: Option<&str>) -> Result<TemplateSet> {
    let inferred = d.effective(&d.root_node_id, "technologyId");
    let id = id.unwrap_or_else(|| template_set_id(&inferred));
    let mut set = if id == "generic" {
        TemplateSet::generic(d)
    } else {
        TemplateSet::builtin(id)?
    };
    set.overlay(d)?;
    Ok(set)
}
pub fn generate(d: &Document, root: &str, options: CompileOptions) -> Result<GeneratedProject> {
    d.node(root)?;
    d.ensure_valid()?;
    let mut scope = BTreeSet::new();
    let mut pending = vec![root.to_string()];
    while let Some(id) = pending.pop() {
        if scope.insert(id.clone()) {
            pending.extend(d.node(&id)?.children.clone());
        }
    }
    for n in d.nodes.values().filter(|n| n.is_link()) {
        if let Some(l) = &n.link
            && relation(d, n) != Some(Relation::TypeUsage)
            && (scope.contains(&l.source_node_id) != scope.contains(&l.target_node_id))
        {
            return Err(Error::new(
                "unsupported",
                "Compile the enclosing composite: the selection has connections outside its scope",
            ));
        }
    }

    let tech = d.effective(root, "technologyId");
    let id = options
        .template_set
        .as_deref()
        .unwrap_or_else(|| template_set_id(&tech));
    if options.bundle.as_ref().is_some_and(|b| b.id == "c") {
        return Err(Error::new(
            "unsupported",
            "C template-bundle execution is pending; use the typed C17 compiler",
        ));
    }
    if options.bundle.is_none() && id == "c" {
        if options
            .layout
            .as_deref()
            .is_some_and(|l| l != "single-file")
        {
            return Err(Error::new("unsupported", "C17 supports single-file layout"));
        }
        if d.nodes
            .values()
            .any(|n| n.name.starts_with('@') && !n.name.eq_ignore_ascii_case("@StaticFile"))
        {
            return Err(Error::new(
                "unsupported",
                "C17 model-template overrides are pending; explicit C17 generation does not apply templates",
            ));
        }
        return crate::c17::C17::generate(d, root);
    }
    if d.nodes
        .values()
        .any(|n| crate::components::is_exterior(n) || crate::components::is_reference(n))
    {
        return Err(Error::new(
            "unsupported",
            "Component Exteriors and References are supported by the C17 compiler only",
        ));
    }
    if options.bundle.is_none() && id == "literal" {
        return crate::generation::literal_files(d, root);
    }
    let mut set = if let Some(bundle) = &options.bundle {
        bundle.clone()
    } else if id == "generic" {
        TemplateSet::generic(d)
    } else {
        TemplateSet::builtin(id)?
    };
    set.overlay(d)?;
    let symbols = d
        .nodes
        .values()
        .filter(|n| !n.is_link())
        .enumerate()
        .map(|(i, n)| {
            (
                n.id.clone(),
                format!("{}_{}", identifier(&n.name).to_lowercase(), i + 1),
            )
        })
        .collect();
    let mut plan = Plan {
        d,
        root,
        set,
        options,
        symbols,
        artifacts: BTreeMap::new(),
        visiting: BTreeSet::new(),
    };
    let artifact = plan.compile(root)?;
    let mut files = artifact.files;
    if plan.set.id == "go" {
        files.iter_mut().for_each(hoist_go_imports);
    }
    if plan.set.id == "rust" {
        files.iter_mut().for_each(comment_duplicate_rust_uses);
    }
    let project_context = plan.project_context();
    for file in &plan.set.project_files {
        if !file.layouts.is_empty() && !file.layouts.contains(&artifact.layout) {
            continue;
        }
        files.push(VirtualFile {
            path: render(&file.path, &project_context)?.trim().into(),
            content: render(&file.content, &project_context)?,
            binary_content: None,
            source_map: vec![],
        });
    }
    // Static companions bypass role rendering and remain files even in single-file graphs.
    for id in d.descendants(root) {
        let n = d.node(&id)?;
        if n.name.eq_ignore_ascii_case("@StaticFile") || plan.set.static_files.contains(&n.name) {
            let path = n
                .metadata
                .get("path")
                .or_else(|| n.metadata.get("file"))
                .cloned()
                .unwrap_or_else(|| n.name.trim_start_matches('@').into());
            let content = if n.text.source.is_empty() {
                n.text.specification.clone()
            } else {
                n.text.source.clone()
            };
            let end = content.lines().count();
            files.push(VirtualFile {
                path,
                content,
                binary_content: n.binary_content.clone(),
                source_map: if end == 0 {
                    vec![]
                } else {
                    vec![SourceMap {
                        node_id: n.id.clone(),
                        section: if n.text.source.is_empty() {
                            "specification".into()
                        } else {
                            "source".into()
                        },
                        generated_start_line: 1,
                        generated_end_line: end,
                        source_start_line: 1,
                        generated_column_offset: 0,
                    }]
                },
            });
        }
    }
    files.extend(crate::generation::file_exports(d, root)?);
    let mut paths = BTreeSet::new();
    let mut bytes = 0usize;
    for file in &files {
        safe_relative_path(&file.path)?;
        if !paths.insert(file.path.clone()) {
            return Err(Error::new(
                "invalid_input",
                format!("Duplicate generated path '{}'", file.path),
            ));
        }
        bytes = bytes.saturating_add(file.content.len());
    }
    if files.len() > 4096 || bytes > 64 * 1024 * 1024 {
        return Err(Error::new(
            "limit_exceeded",
            "Generated project exceeds 4096 files or 64 MiB",
        ));
    }
    Ok(GeneratedProject {
        name: d.project_name().into(),
        files,
        capability: format!("template-set:{}", plan.set.id),
    })
}
impl Plan<'_> {
    fn layout(&self, n: &Node) -> Result<String> {
        let value = self.options.layout.clone().unwrap_or_else(|| {
            let value = self.d.effective(&n.id, "fileLayoutStrategyId");
            if value == "none" {
                self.set.default_layout.clone()
            } else {
                value
            }
        });
        // Accept identifiers emitted by earlier Wirecraft inspector versions.
        let value = match value.as_str() {
            "direct-filesystem" => "direct-file-system-homomorphism".into(),
            "classified" => "classified-filesystem".into(),
            _ => value,
        };
        if ![
            "single-file",
            "direct-file-system-homomorphism",
            "classified-filesystem",
            "source-set",
        ]
        .contains(&value.as_str())
        {
            return Err(Error::new(
                "unsupported",
                format!("Unsupported compiler layout {value}"),
            ));
        }
        if ["go", "systemjs", "rust"].contains(&self.set.id.as_str()) && value != "single-file" {
            return Err(Error::new(
                "unsupported",
                format!("{} supports single-file layout", self.set.id),
            ));
        }
        Ok(value)
    }
    fn compile(&mut self, id: &str) -> Result<Artifact> {
        if let Some(a) = self.artifacts.get(id) {
            return Ok(a.clone());
        }
        if self.visiting.len() > 256 || !self.visiting.insert(id.into()) {
            return Err(Error::new(
                "invalid_graph",
                format!("Compiler dependency cycle or excessive depth at {id}"),
            ));
        }
        let n = self.d.node(id)?.clone();
        let layout = self.layout(&n)?;
        let path = self.path(&n, &layout)?;
        let skipped = (n.name.starts_with('@') && !n.name.eq_ignore_ascii_case("@StaticFile"))
            || self.set.static_files.contains(&n.name)
            || n.name.eq_ignore_ascii_case("@StaticFile")
            || self.d.effective(id, "technologyId") == "file-export"
            || relation(self.d, &n) == Some(Relation::TypeUsage);
        if skipped {
            self.visiting.remove(id);
            let artifact = Artifact {
                context: self.node_view(&n),
                declaration: String::new(),
                initialization: String::new(),
                hoisted: vec![],
                forward: vec![],
                files: vec![],
                path,
                layout,
            };
            self.artifacts.insert(id.into(), artifact.clone());
            return Ok(artifact);
        }
        if !n.is_link()
            && n.id != self.root
            && n.kind != NodeKind::Note
            && self.options.template_set.is_none()
            && self.options.bundle.is_none()
        {
            let technology = self.d.effective(id, "technologyId");
            if technology != self.set.technology {
                return Err(Error::new(
                    "unsupported",
                    format!(
                        "Mixed technology '{}' inside {} requires aggregate compiler delegation",
                        technology, self.set.technology
                    ),
                ));
            }
        }
        // A data link whose ends sit in different composites is declared and transported
        // by its owner; in a single file its buffers are visible to the nested end.
        if let Some(link) = &n.link
            && !capability(self.d, &n)
            && (self.d.node(&link.source_node_id)?.parent_id != n.parent_id
                || self.d.node(&link.target_node_id)?.parent_id != n.parent_id)
        {
            let ends = [&link.source_node_id, &link.target_node_id];
            for end in ends {
                if self.layout(self.d.node(end)?)? != "single-file" || layout != "single-file" {
                    return Err(Error::new(
                        "unsupported",
                        "Template compiler composite boundary channels need the single-file layout",
                    ));
                }
            }
        }
        // Supplier products must be available before rendering capability links.
        if let Some(link) = &n.link
            && matches!(
                relation(self.d, &n),
                Some(Relation::Source | Relation::Runnable)
            )
            && !self.artifacts.contains_key(&link.source_node_id)
        {
            self.compile(&link.source_node_id)?;
        }
        if relation(self.d, &n) == Some(Relation::Runnable)
            && ["go", "c", "rust"].contains(&self.set.id.as_str())
        {
            return Err(Error::new(
                "unsupported",
                format!(
                    "{} does not provide a runtime compiler for run capabilities",
                    self.set.id
                ),
            ));
        }
        let mut children = vec![];
        let mut links = vec![];
        for link in [false, true] {
            for child in &n.children {
                if self.d.nodes[child].is_link() == link {
                    let artifact = self.compile(child)?;
                    if link {
                        links.push(artifact);
                    } else {
                        children.push(artifact);
                    }
                }
            }
        }
        let mut context = self.context(&n, &layout, &path, &children, &links)?;
        let kind = kind(&n);
        let role = stereotype(self.d, &n);
        let render_phase = |phase: &str, c: &Value| -> Result<String> {
            self.set
                .role(kind, role, n.is_composite(), phase, &layout)
                .map(|s| render(s, c))
                .transpose()
                .map(|v| v.unwrap_or_default())
        };
        let own = render_phase("declaration", &context)?;
        let initialization = render_phase("instantiation", &context)?;
        let mut hoisted = children
            .iter()
            .chain(&links)
            .flat_map(|a| a.hoisted.iter().cloned())
            .collect::<Vec<_>>();
        let mut forward = children
            .iter()
            .chain(&links)
            .flat_map(|a| a.forward.iter().cloned())
            .collect::<Vec<_>>();
        let h = render_phase("hoisted-declaration", &context)?;
        if !h.trim().is_empty() {
            hoisted.push(h);
        }
        let f = render_phase("forward-declaration", &context)?;
        if !f.trim().is_empty() {
            forward.push(f);
        }
        dedup(&mut hoisted);
        dedup(&mut forward);
        let own = if self.set.id == "kotlin" && layout == "single-file" && n.is_composite() {
            format!(
                "{}\n{own}",
                context["linkDeclarations"].as_str().unwrap_or_default()
            )
        } else {
            own
        };
        context["ownDeclaration"] = json!(own);
        context["hoistedDeclarations"] = json!(hoisted.join("\n\n"));
        context["forwardDeclarations"] = json!(forward.join("\n\n"));
        context["runtimeSupport"] = json!(
            self.set
                .roles
                .get("runtime.support")
                .map(|s| render(s, &context))
                .transpose()?
                .unwrap_or_default()
        );
        let assembly_role = match layout.as_str() {
            "single-file" => "composite.single-file",
            "direct-file-system-homomorphism" => "composite.direct-file-system",
            "source-set" => "composite.source-set",
            _ => "composite.classified-file-system",
        };
        let assembly = self.set.roles.get(assembly_role).or_else(|| {
            if layout != "single-file" {
                self.set.roles.get("composite.file-based")
            } else {
                None
            }
        });
        let mut declaration = if n.is_composite() {
            assembly
                .map(|s| render(s, &context))
                .transpose()?
                .unwrap_or(own)
        } else {
            own
        };
        if self.set.id == "kotlin"
            && layout != "single-file"
            && !n.is_link()
            && !declaration.is_empty()
            && !declaration.trim_start().starts_with("package ")
        {
            declaration = format!("package generated.nodes\nimport generated.*\n{declaration}");
        }
        if self.set.id == "kotlin" && layout != "single-file" {
            declaration =
                declaration.replacen("import generated.RuntimeContext", "import generated.*", 1);
        }
        let mut files = vec![];
        if !n.is_link() && !declaration.trim().is_empty() {
            files.push(VirtualFile {
                path: path.clone(),
                content: declaration.clone(),
                binary_content: n.binary_content.clone(),
                source_map: self.source_maps(&declaration),
            });
        }
        for a in children.iter().chain(&links) {
            if layout != "single-file" || a.layout != "single-file" {
                files.extend(a.files.clone());
            }
        }
        context["declaration"] = json!(declaration);
        context["instantiation"] = json!(initialization);
        context["node"] = self.node_view(&n);
        context["isInline"] = json!(layout == "single-file");
        context["path"] = json!(path);
        // Artifact views are flat: retaining recursive child contexts amplifies nested graphs exponentially.
        let context = Value::Object(
            context
                .as_object()
                .unwrap()
                .iter()
                .filter(|(key, _)| {
                    [
                        "node",
                        "symbol",
                        "runSymbol",
                        "initializerSymbol",
                        "classFileSymbol",
                        "isCapabilityOnlyProvider",
                        "incomingDataLinks",
                        "outgoingDataLinks",
                        "dependencyInjectionLinks",
                        "capabilityLinks",
                        "link",
                        "declaration",
                        "instantiation",
                        "isInline",
                        "path",
                    ]
                    .contains(&key.as_str())
                })
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );
        let artifact = Artifact {
            context,
            declaration,
            initialization,
            hoisted,
            forward,
            files,
            path,
            layout,
        };
        self.visiting.remove(id);
        self.artifacts.insert(id.into(), artifact.clone());
        Ok(artifact)
    }
    fn node_view(&self, n: &Node) -> Value {
        let indexed = self
            .symbols
            .get(&n.id)
            .cloned()
            .unwrap_or_else(|| identifier(&n.name));
        let role = if n.is_link() {
            match relation(self.d, n) {
                Some(Relation::Principal) => "Transport",
                Some(Relation::Error) => "ErrorPipe",
                _ => "DependencyInjection",
            }
        } else {
            stereotype(self.d, n)
        };
        json!({"id":n.id,"name":n.name,"symbol":identifier(&n.name),"runSymbol":format!("run_{indexed}"),"initializerSymbol":format!("init_{indexed}"),"classFileSymbol":upper(&indexed),"kind":{"name":format!("{:?}",n.kind)},"isComposite":n.is_composite(),"isTerminal":!n.is_composite()&&!n.is_link(),"isLink":n.is_link(),"stereotype":role,"children":n.children,"metadata":n.metadata,"pluginData":n.plugin_data,"text":{"declaration":n.text.source,"instantiation":n.text.initialization,"specification":n.text.specification},"technology":n.technology,"layout":n.layout,"typeFields":self.fields(n),"ports":n.ports})
    }
    fn fields(&self, n: &Node) -> Vec<Value> {
        n.type_definition.as_ref().and_then(|v|v["fields"].as_array()).into_iter().flatten().map(|f|{let name=f["name"].as_str().unwrap_or("");let mut ty=self.type_name(f["typeId"].as_str().unwrap_or(""));if self.set.id=="kotlin"&&ty=="boolean"{ty="Boolean".into();}json!({"name":name,"symbol":self.member(&identifier(name)),"typeName":ty,"typeSymbol":ty,"isReference":f["isReference"].as_bool().unwrap_or(false)})}).collect()
    }
    /// Parameter and field names: Rust spells keywords as raw identifiers (`r#type`).
    fn member(&self, symbol: &str) -> String {
        const RUST_KEYWORDS: &[&str] = &[
            "as", "async", "await", "break", "const", "continue", "dyn", "else", "enum", "extern",
            "false", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod", "move",
            "mut", "pub", "ref", "return", "static", "struct", "trait", "true", "type", "unsafe",
            "use", "where", "while", "abstract", "become", "box", "do", "final", "macro",
            "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
        ];
        match symbol {
            // These cannot be raw identifiers.
            "self" | "Self" | "super" | "crate" if self.set.id == "rust" => format!("{symbol}_"),
            _ if self.set.id == "rust" && RUST_KEYWORDS.contains(&symbol) => format!("r#{symbol}"),
            _ => symbol.into(),
        }
    }
    fn type_name(&self, id: &str) -> String {
        self.d
            .nodes
            .get(id)
            .map(|n| identifier(&n.name))
            .unwrap_or_else(|| id.into())
    }
    fn descriptor(&self, n: &Node) -> Result<Value> {
        let l = n
            .link
            .as_ref()
            .ok_or_else(|| Error::new("invalid_input", "Expected a link"))?;
        let source = self.d.node(&l.source_node_id)?;
        let target = self.d.node(&l.target_node_id)?;
        let cap = capability(self.d, n);
        let ordinal = source
            .outgoing_links
            .iter()
            .filter_map(|id| self.d.nodes.get(id))
            .filter(|link| capability(self.d, link) == cap)
            .position(|link| link.id == n.id)
            .unwrap_or(0)
            + 1;
        let symbol = identifier(&n.name);
        let allocation = format!("{symbol}{ordinal}");
        let argument = self.member(&symbol);
        let source_product = self
            .artifacts
            .get(&source.id)
            .map(|a| a.declaration.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(&source.text.source);
        let ty = if !l.type_definition_id.is_empty() {
            self.type_name(&l.type_definition_id)
        } else {
            l.type_name.clone()
        };
        let declared = self.d.nodes.get(&l.type_definition_id);
        let escaped = serde_json::to_string(source_product).unwrap();
        let escaped = &escaped[1..escaped.len() - 1];
        let mut descriptor = json!({"id":n.id,"name":n.name,"symbol":symbol,"variableName":n.name,"argumentSymbol":argument,"allocationSymbol":allocation,"dependencySymbol":allocation,"capabilityTypeSymbol":format!("{}Capability",upper(&allocation)),"aPortSymbol":format!("{allocation}_a_port"),"bPortSymbol":format!("{allocation}_b_port"),"transportSymbol":format!("transport_{allocation}"),"typeName":ty,"typeSymbol":ty,"typeDefinitionId":l.type_definition_id,"typeDefinition":l.payload_definition,"payloadDefinition":l.payload_definition,"typeFields":declared.map(|n|self.fields(n)).unwrap_or_default(),"isCapability":cap,"isLibraryCapability":relation(self.d,n)==Some(Relation::Dependency),"isSourceCapability":relation(self.d,n)==Some(Relation::Source),"isRunnableCapability":relation(self.d,n)==Some(Relation::Runnable),"sourceNodeName":source.name,"sourceNodeSymbol":identifier(&source.name),"sourceNodeTypeSymbol":identifier(&source.name),"sourceRunSymbol":format!("run_{}",self.symbols[&source.id]),"sourceNodeDeclaration":source_product,"sourceNodeDeclarationDoubleQuoted":escaped,"sourceNodeDeclarationDollarEscaped":escaped.replace('$',"\\$"),"sourceNodeInstantiation":source.text.initialization,"libraryInstantiation":source.text.initialization,"sourceNodeId":source.id,"targetNodeId":target.id,"targetNodeName":target.name,"sourcePortName":l.source_port_name,"targetPortName":l.target_port_name,"transportKind":l.transport_kind,"interactionKind":l.interaction_kind});
        if relation(self.d, n) == Some(Relation::Dependency) {
            // Every link from a library lends the same shared instance; the link name is
            // only the consumer's argument name (`argumentSymbol`).
            let (ty, instance) = crate::libraries::symbols(source);
            let mut lent = instance.clone();
            if self.set.id == "nodejs"
                && let Some(parent) = target
                    .parent_id
                    .as_deref()
                    .and_then(|p| self.d.nodes.get(p))
            {
                // File-based modules reach the instance through the (cached) library module.
                let caller = self.layout(parent)?;
                let module = self.layout(source)?;
                if caller != "single-file" || module != "single-file" {
                    let from = self.path(parent, &caller)?;
                    let to = self.path(source, &module)?;
                    if from != to {
                        lent = format!("require({:?}).{instance}", relative(&from, &to, true));
                    }
                }
            }
            descriptor["libraryTypeSymbol"] = json!(ty);
            descriptor["libraryInstanceSymbol"] = json!(instance);
            descriptor["dependencySymbol"] = json!(lent);
        }
        Ok(descriptor)
    }
    fn context(
        &self,
        n: &Node,
        layout: &str,
        path: &str,
        children: &[Artifact],
        links: &[Artifact],
    ) -> Result<Value> {
        let mut incoming = n
            .incoming_links
            .iter()
            .filter_map(|id| self.d.nodes.get(id))
            .filter(|n| relation(self.d, n) != Some(Relation::TypeUsage))
            .map(|n| self.descriptor(n))
            .collect::<Result<Vec<_>>>()?;
        let mut outgoing = n
            .outgoing_links
            .iter()
            .filter_map(|id| self.d.nodes.get(id))
            .filter(|n| relation(self.d, n) != Some(Relation::TypeUsage))
            .map(|n| self.descriptor(n))
            .collect::<Result<Vec<_>>>()?;
        for (values, suffix) in [(&mut incoming, "in"), (&mut outgoing, "out")] {
            for link in values.iter_mut() {
                if link["sourceNodeId"] == n.id && link["targetNodeId"] == n.id {
                    link["argumentSymbol"] =
                        json!(format!("{}_{suffix}", link["symbol"].as_str().unwrap()));
                }
            }
        }
        let indexed = self
            .symbols
            .get(&n.id)
            .cloned()
            .unwrap_or_else(|| identifier(&n.name));
        let source = marked(n, "source", &n.text.source);
        let initialization = marked(n, "initialization", &n.text.initialization);
        let artifact_views = |items: &[Artifact]| {
            items
                .iter()
                .map(|a| {
                    let mut v = a.context.clone();
                    v["relativeModulePath"] = json!(relative(path, &a.path, true));
                    v["relativePath"] = json!(relative(path, &a.path, false));
                    v
                })
                .collect::<Vec<_>>()
        };
        let child_views = artifact_views(
            &children
                .iter()
                .filter(|a| !a.declaration.is_empty())
                .cloned()
                .collect::<Vec<_>>(),
        );
        let link_views = artifact_views(links);
        let inline = children
            .iter()
            .filter(|a| a.layout == "single-file" && !a.declaration.is_empty())
            .map(|a| a.declaration.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let library = (stereotype(self.d, n) == "ServiceLibrary").then(|| {
            crate::libraries::instance(
                &self.set.id,
                n,
                self.set.id == "nodejs" && layout != "single-file",
            )
        });
        let mut c = json!({"id":n.id,"name":n.name,"node":self.node_view(n),"self":self.node_view(n),"document":{"id":self.d.id,"name":self.d.project_name(),"rootNodeId":self.d.root_node_id},"options":{"projectName":self.d.project_name()},"metadata":n.metadata,"symbol":identifier(&n.name),"safeName":identifier(&n.name),"runSymbol":format!("run_{indexed}"),"initializerSymbol":format!("init_{indexed}"),"classFileSymbol":upper(&indexed),"isCompilationRoot":n.id==self.root,"isCapabilityOnlyProvider":!outgoing.is_empty()&&outgoing.iter().all(|l|l["isCapability"]==true),"language":self.set.language,"stereotype":stereotype(self.d,n),"layoutStrategy":{"id":layout},"primaryPath":path,"declaration":source,"instantiation":initialization,"declarationIndent2":indent(&source,2),"declarationIndent4":indent(&source,4),"instantiationIndent2":indent(&initialization,2),"instantiationIndent4":indent(&initialization,4),"typeFields":self.fields(n),"specification":n.text.specification,"tests":n.text.tests,"usageInstructions":n.text.ai_instructions,"childArtifacts":child_views,"inlineChildArtifacts":artifact_views(&children.iter().filter(|a|a.layout=="single-file").cloned().collect::<Vec<_>>()),"externalChildArtifacts":artifact_views(&children.iter().filter(|a|a.layout!="single-file").cloned().collect::<Vec<_>>()),"linkArtifacts":link_views,"childDeclarations":children.iter().map(|a|a.declaration.as_str()).collect::<Vec<_>>().join("\n\n"),"inlineChildDeclarations":inline,"inlineChildDeclarationsWithoutPhpTag":inline.replace("<?php", ""),"childInstantiations":children.iter().map(|a|a.initialization.as_str()).collect::<Vec<_>>().join("\n"),"linkDeclarations":links.iter().map(|a|a.declaration.as_str()).collect::<Vec<_>>().join("\n\n"),"linkInstantiations":links.iter().map(|a|a.initialization.as_str()).collect::<Vec<_>>().join("\n"),"nodeProvenanceComment":format!("// {} [{}]",n.name.replace(['\n','\r']," "),n.id),"initializerProvenanceComment":format!("// Initialize {}",n.name.replace(['\n','\r']," ")),"runProvenanceComment":format!("// Execute {}",n.name.replace(['\n','\r']," ")),"linkProvenanceComment":format!("// Link {}",n.name.replace(['\n','\r']," ")),"transportProvenanceComment":format!("// Transport {}",n.name.replace(['\n','\r']," ")),"safeProjectName":identifier(self.d.project_name()),"projectName":self.d.project_name()});
        if let Some(library) = library {
            c["libraryInstance"] = json!(library.code);
            c["libraryInstanceSymbol"] = json!(library.instance_symbol);
            c["libraryTypeSymbol"] = json!(library.type_symbol);
        }
        for (name, values) in [
            ("incomingLinks", incoming.clone()),
            ("outgoingLinks", outgoing.clone()),
            (
                "incomingDataLinks",
                incoming
                    .iter()
                    .filter(|l| l["isCapability"] != true)
                    .cloned()
                    .collect(),
            ),
            (
                "outgoingDataLinks",
                outgoing
                    .iter()
                    .filter(|l| l["isCapability"] != true)
                    .cloned()
                    .collect(),
            ),
            (
                "dependencyInjectionLinks",
                incoming
                    .iter()
                    .filter(|l| l["isCapability"] == true)
                    .cloned()
                    .collect(),
            ),
            (
                "capabilityLinks",
                incoming
                    .iter()
                    .filter(|l| l["isCapability"] == true)
                    .cloned()
                    .collect(),
            ),
        ] {
            c[name] = json!(values);
        }
        if n.is_link() {
            c["link"] = self.descriptor(n)?;
        }
        let mut imports = vec![];
        if let Some(template) = self.set.roles.get("child.import") {
            for child in c["externalChildArtifacts"].as_array().unwrap() {
                let mut context = c.clone();
                context["child"] = child.clone();
                imports.push(render(template, &context)?);
            }
        }
        c["childImports"] = json!(imports.join("\n"));
        for phase in ["Hoisted", "Forward"] {
            let key = phase.to_lowercase();
            for (name, items) in [("child", children), ("link", links)] {
                let values = items
                    .iter()
                    .flat_map(|a| {
                        if key == "hoisted" {
                            a.hoisted.iter()
                        } else {
                            a.forward.iter()
                        }
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                c[format!("{name}{phase}Declarations")] = json!(values.join("\n\n"));
            }
        }
        Ok(c)
    }
    fn path(&self, n: &Node, layout: &str) -> Result<String> {
        let explicit = n.metadata.get("path").or_else(|| n.metadata.get("file"));
        if let Some(path) = explicit {
            safe_relative_path(path)?;
            return Ok(path.clone());
        }
        let name = identifier(&n.name);
        let extension = &self.set.extension;
        let default = match layout {
            "single-file" => format!("{name}.{extension}"),
            "source-set" => format!(
                "src/main/{}/generated/nodes/{}.{}",
                self.set.language,
                upper(
                    &self
                        .symbols
                        .get(&n.id)
                        .cloned()
                        .unwrap_or_else(|| identifier(&n.name))
                ),
                extension
            ),
            "classified-filesystem" => format!(
                "{}/{name}.{extension}",
                if n.is_composite() {
                    "composites"
                } else if stereotype(self.d, n) == "ServiceLibrary" {
                    "libraries"
                } else {
                    "nodes"
                }
            ),
            _ => {
                let mut parents = self.d.ancestors(&n.id);
                parents.reverse();
                let mut parts = parents
                    .iter()
                    .filter(|id| id.as_str() != n.id)
                    .filter_map(|id| self.d.nodes.get(id))
                    .map(|n| identifier(&n.name))
                    .collect::<Vec<_>>();
                parts.push(format!("{name}.{extension}"));
                parts.join("/")
            }
        };
        let context = json!({"node":self.node_view(n),"name":n.name,"classFileSymbol":upper(&self.symbols.get(&n.id).cloned().unwrap_or_else(||identifier(&n.name))),"defaultPath":default,"safeProjectName":identifier(self.d.project_name())});
        if let Some(template) = self
            .set
            .roles
            .get(&format!("primary-file.path.{layout}"))
            .or_else(|| self.set.roles.get("primary-file.path"))
        {
            Ok(render(template, &context)?.trim().into())
        } else {
            Ok(default)
        }
    }
    fn project_context(&self) -> Value {
        let mut c = self.node_view(&self.d.nodes[self.root]);
        c["root"] = self.node_view(&self.d.nodes[self.root]);
        c["projectName"] = json!(self.d.project_name());
        c["safeProjectName"] = json!(identifier(self.d.project_name()));
        c["safePackageName"] = json!(identifier(self.d.project_name()).to_lowercase());
        c["projectIdentifier"] = json!(identifier(self.d.project_name()));
        c["document"] = json!({"name":self.d.project_name()});
        c["scopeRoots"] = json!([self.node_view(&self.d.nodes[self.root])]);
        c["layoutStrategy"] = json!({"id":self.layout(&self.d.nodes[self.root]).unwrap()});
        c["singleFileBaseName"] = json!(identifier(&self.d.nodes[self.root].name));
        c["mainClassName"] = json!(if self.layout(&self.d.nodes[self.root]).unwrap()
            == "single-file"
        {
            format!(
                "generated.{}Kt",
                upper(&identifier(&self.d.nodes[self.root].name))
            )
        } else {
            "generated.MainKt".into()
        });
        c
    }
    fn source_maps(&self, content: &str) -> Vec<SourceMap> {
        let mut maps = vec![];
        let mut pending = None;
        for (i, line) in content.lines().enumerate() {
            let trim = line.trim();
            if let Some(rest) = trim.strip_prefix("// wirecraft-source:")
                && let Some((token, phase)) = rest.rsplit_once(':')
            {
                if phase == "begin" {
                    if let Some((id, section)) = token.rsplit_once(':') {
                        pending = Some((
                            id.to_string(),
                            section.to_string(),
                            i + 2,
                            line.len() - line.trim_start().len(),
                        ));
                    }
                } else if phase == "end"
                    && let Some((id, section, start, offset)) = pending.take()
                    && start <= i
                {
                    maps.push(SourceMap {
                        node_id: id,
                        section,
                        generated_start_line: start,
                        generated_end_line: i,
                        source_start_line: 1,
                        generated_column_offset: offset,
                    });
                }
            }
        }
        maps
    }
}
/// A Go file has one import list, before any declaration, but service libraries are
/// rendered further down. Their `import` declarations move into the file's first import
/// block; each original line stays as a comment, and source maps below the block shift
/// by the lines added to it.
fn hoist_go_imports(file: &mut VirtualFile) {
    let lines: Vec<&str> = file.content.lines().collect();
    let Some(open) = lines.iter().position(|l| l.trim() == "import (") else {
        return;
    };
    let Some(close) = lines[open..]
        .iter()
        .position(|l| l.trim() == ")")
        .map(|i| open + i)
    else {
        return;
    };
    let mut present: BTreeSet<String> = lines[open + 1..close]
        .iter()
        .map(|l| l.trim().to_string())
        .collect();
    let mut added = vec![];
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    let mut block = false;
    for (i, line) in lines.iter().enumerate().skip(close + 1) {
        let trimmed = line.trim();
        let spec = if block {
            if trimmed == ")" {
                block = false;
                out[i] = format!("// {trimmed} (hoisted)");
                continue;
            }
            Some(trimmed)
        } else if trimmed == "import (" {
            block = true;
            out[i] = format!("// {trimmed} (hoisted)");
            continue;
        } else {
            trimmed.strip_prefix("import ").map(str::trim)
        };
        let Some(spec) = spec.filter(|s| s.contains('"')) else {
            continue;
        };
        out[i] = format!("// import {spec} (hoisted)");
        if present.insert(spec.to_string()) {
            added.push(format!("\t{spec}"));
        }
    }
    if added.is_empty() {
        return;
    }
    let shift = added.len();
    out.splice(close..close, added);
    let mut content = out.join("\n");
    if file.content.ends_with('\n') {
        content.push('\n');
    }
    file.content = content;
    // Maps use one-based lines; everything after the block's opening line moves down.
    for map in &mut file.source_map {
        if map.generated_start_line > open + 1 {
            map.generated_start_line += shift;
            map.generated_end_line += shift;
        }
    }
}
/// Library sources share the file's module scope, where importing one name twice is an
/// error: a repeated single-line file-scope `use` stays as a comment, so lines keep their
/// numbers and source maps stay valid.
fn comment_duplicate_rust_uses(file: &mut VirtualFile) {
    let mut seen = BTreeSet::new();
    let mut changed = false;
    let lines = file
        .content
        .lines()
        .map(|line| {
            let statement = line.trim_end();
            let import = (statement.starts_with("use ") || statement.starts_with("pub use "))
                && statement.ends_with(';');
            if import && !seen.insert(statement.to_string()) {
                changed = true;
                format!("// {statement} (duplicate)")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>();
    if changed {
        let mut content = lines.join("\n");
        if file.content.ends_with('\n') {
            content.push('\n');
        }
        file.content = content;
    }
}
fn marked(n: &Node, section: &str, text: &str) -> String {
    if text.is_empty() {
        String::new()
    } else {
        format!(
            "// wirecraft-source:{}:{section}:begin\n{text}\n// wirecraft-source:{}:{section}:end",
            n.id, n.id
        )
    }
}
fn capability(d: &Document, n: &Node) -> bool {
    matches!(
        relation(d, n),
        Some(Relation::Dependency | Relation::Source | Relation::Runnable | Relation::TypeUsage)
    )
}
fn kind(n: &Node) -> &'static str {
    match n.kind {
        NodeKind::Node => "node",
        NodeKind::Processor => "processor",
        NodeKind::Group => "group",
        NodeKind::Type => "type",
        NodeKind::Link => "link",
        NodeKind::Note => "note",
    }
}
fn identifier(name: &str) -> String {
    let value = crate::c17::identifier(name);
    if value.is_empty() {
        "node".into()
    } else {
        value
    }
}
fn upper(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|v| v.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}
fn indent(s: &str, n: usize) -> String {
    s.lines()
        .map(|s| format!("{}{s}", " ".repeat(n)))
        .collect::<Vec<_>>()
        .join("\n")
}
fn dedup(v: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    v.retain(|s| seen.insert(s.clone()));
}
fn relative(from: &str, to: &str, module: bool) -> String {
    let mut from = from.split('/').collect::<Vec<_>>();
    from.pop();
    let to = to.split('/').collect::<Vec<_>>();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut parts = vec![".."; from.len() - common];
    parts.extend_from_slice(&to[common..]);
    let mut value = parts.join("/");
    if module && let Some(i) = value.rfind('.') {
        value.truncate(i);
    }
    if !value.starts_with('.') {
        value = format!("./{value}");
    }
    value
}
