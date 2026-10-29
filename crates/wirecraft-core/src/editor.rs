//! Compiler-backed body context. No editor framework or language-server dependency.
//! C17 bodies are analyzed in the typed C17 unit; Rust bodies in the Rust template output.
use crate::{
    Document, Error, Result,
    analysis::{Relation, relation},
    c17::C17,
    generation::{GeneratedProject, SourceMap, VirtualFile},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub struct BodyContext {
    /// LSP language of the generated unit: `c` or `rust`.
    pub language: String,
    pub file: VirtualFile,
    pub companions: Vec<VirtualFile>,
    pub source: SourceMap,
    pub header: String,
    pub symbols: Vec<Value>,
    pub type_signatures: BTreeMap<String, String>,
    pub excluded_library_prefixes: BTreeSet<String>,
    /// Generated struct names of modeled types and library instances, by entity id.
    pub declarations: BTreeMap<String, String>,
    /// Declaration texts of the libraries lent to this body, by entity id.
    pub library_sources: Vec<(String, String)>,
}
impl BodyContext {
    pub fn project(&self, name: &str) -> GeneratedProject {
        GeneratedProject {
            name: name.into(),
            files: std::iter::once(self.file.clone())
                .chain(self.companions.iter().cloned())
                .collect(),
            capability: if self.language == "rust" {
                "rust-editor-context".into()
            } else {
                "c17-editor-context".into()
            },
        }
    }
}
pub fn body_context(
    document: &Document,
    node_id: &str,
    section: &str,
    draft: Option<&str>,
) -> Result<BodyContext> {
    document.node(node_id)?;
    match compiler_language(document, node_id, section) {
        Some("c") => c17_body_context(document, node_id, section, draft),
        Some(_) => rust_body_context(document, node_id, section, draft),
        None => Err(Error::new(
            "unsupported",
            "Compiler body context currently supports C17 and Rust Declaration and Instantiation sections",
        )),
    }
}
/// The generated-program language (`c` or `rust`) whose compiler and language server
/// analyze this body, if any: Declaration and Instantiation sections of C17 or Rust
/// entities.
pub fn compiler_language(
    document: &Document,
    node_id: &str,
    section: &str,
) -> Option<&'static str> {
    let node = document.nodes.get(node_id)?;
    if !["source", "initialization"].contains(&section)
        || node.is_link()
        || matches!(node.kind, crate::NodeKind::Type | crate::NodeKind::Note)
    {
        return None;
    }
    match (
        document.effective_text_language(node_id, section).as_str(),
        document.effective(node_id, "technologyId").as_str(),
    ) {
        ("c", "c-native") => Some("c"),
        ("rust", "rust") => Some("rust"),
        _ => None,
    }
}
fn c17_body_context(
    document: &Document,
    node_id: &str,
    section: &str,
    draft: Option<&str>,
) -> Result<BodyContext> {
    let mut candidate = document.clone();
    let node = candidate
        .nodes
        .get_mut(node_id)
        .ok_or_else(|| Error::new("not_found", "Editor entity does not exist"))?;
    if let Some(text) = draft {
        if section == "source" {
            node.text.source = text.into();
        } else {
            node.text.initialization = text.into();
        }
    }
    let project = C17::generate(&candidate, &candidate.root_node_id)?;
    let mut files = project.files.into_iter();
    let file = files
        .next()
        .ok_or_else(|| Error::new("unsupported", "Compiler produced no C unit"))?;
    let source = file
        .source_map
        .iter()
        .find(|m| m.node_id == node_id && m.section == section)
        .cloned()
        .ok_or_else(|| {
            Error::new(
                "unsupported",
                "Selected entity has no editable C body in the compiled unit",
            )
        })?;
    let header = file
        .content
        .lines()
        .take(source.generated_start_line - 1)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .find(|line| line.starts_with("static threadwork_error_t tw_"))
        .filter(|_| {
            crate::analysis::stereotype(&candidate, &candidate.nodes[node_id]) != "ServiceLibrary"
        })
        .unwrap_or("/* C service library declarations */")
        .to_string();
    let mut symbols = Vec::new();
    let mut excluded_library_prefixes = BTreeSet::new();
    for n in candidate.nodes.values() {
        let Some(link) = &n.link else {
            continue;
        };
        let name = crate::c17::identifier(&n.name);
        if relation(&candidate, n) == Some(Relation::Dependency) {
            if link.target_node_id != node_id {
                excluded_library_prefixes.insert(format!("{name}__"));
            } else if let Some(library) = candidate.nodes.get(&link.source_node_id) {
                let members = crate::c17::library_members(library)
                    .iter()
                    .map(|m| m.signature())
                    .collect::<Vec<_>>();
                symbols.push(json!({"label":name,"kind":6,"nodeId":library.id,"detail":format!("const {} *{name}",crate::c17::library_type(library)),"documentation":format!("Instance of library {} injected by dependency {}; call members with {name}->function(...).\n\n{}",library.name,n.name,members.join("\n"))}));
                // Former prefixed aliases are offered only to bodies that still use them.
                let body = &candidate.nodes[node_id];
                if format!("{}\n{}", body.text.source, body.text.initialization)
                    .contains(&format!("{name}__"))
                {
                    for function in crate::c17::library_functions(&candidate, n)? {
                        symbols.push(json!({"label":function.name,"kind":3,"detail":function.signature,"documentation":format!("Injected by dependency {}",n.name)}));
                    }
                }
            }
            continue;
        }
        if relation(&candidate, n) == Some(Relation::Source) {
            if link.target_node_id == node_id {
                let provider = candidate
                    .nodes
                    .get(&link.source_node_id)
                    .map(|n| n.name.as_str())
                    .unwrap_or(&link.source_node_id);
                // The generated header is authoritative for the facade type name.
                let ctype = regex::Regex::new(&format!(r"(\w+Capability) \*{name}\b"))
                    .ok()
                    .and_then(|r| r.captures(&header).map(|c| c[1].to_string()))
                    .unwrap_or_else(|| "source capability".into());
                symbols.push(json!({"label":name,"kind":6,"nodeId":link.source_node_id,"detail":format!("{ctype} *{name}; char *getSource(capability, parameters, parameter_count)"),"documentation":format!("Materializes the declaration of {provider}, replacing ${{name}} markers with threadwork_build_parameter values; the caller frees the result.")}));
            }
            continue;
        }
        if !matches!(
            relation(&candidate, n),
            Some(Relation::Principal | Relation::Error)
        ) {
            continue;
        }
        for (matches, direction) in [
            (link.target_node_id == node_id, "input"),
            (link.source_node_id == node_id, "output"),
        ] {
            if matches {
                let argument = if link.source_node_id == link.target_node_id {
                    format!("{name}_{}", if direction == "input" { "in" } else { "out" })
                } else {
                    name.clone()
                };
                let payload = candidate
                    .nodes
                    .get(&link.type_definition_id)
                    .map(|n| n.name.as_str())
                    .unwrap_or(if link.type_name.is_empty() {
                        "Any"
                    } else {
                        &link.type_name
                    });
                symbols.push(json!({"label":argument,"kind":6,"nodeId":n.id,"detail":format!("{direction} buffer of {payload}"),"documentation":format!("{}:{} → {}:{}",link.source_node_id,link.source_port_name,link.target_node_id,link.target_port_name)}));
            }
        }
    }
    // A reused alias prefix is available if any incoming dependency supplies it.
    for n in candidate.nodes.values().filter(|n| {
        n.link.as_ref().is_some_and(|l| l.target_node_id == node_id)
            && relation(&candidate, n) == Some(Relation::Dependency)
    }) {
        excluded_library_prefixes.remove(&format!("{}__", crate::c17::identifier(&n.name)));
    }
    let mut type_signatures = BTreeMap::new();
    for node in candidate
        .nodes
        .values()
        .filter(|n| n.kind == crate::NodeKind::Type)
    {
        let start = format!("typedef struct {} {{", node.name);
        let end = format!("}} {};", node.name);
        if let Some(a) = file.content.find(&start)
            && let Some(b) = file.content[a..].find(&end)
        {
            type_signatures.insert(
                node.name.clone(),
                file.content[a..a + b + end.len()].to_string(),
            );
        }
    }
    let mut declarations: BTreeMap<String, String> = candidate
        .nodes
        .values()
        .filter(|n| n.kind == crate::NodeKind::Type)
        .map(|n| (n.name.clone(), n.id.clone()))
        .collect();
    for library in libraries(&candidate) {
        declarations.insert(crate::c17::library_type(library), library.id.clone());
    }
    Ok(BodyContext {
        language: "c".into(),
        declarations,
        library_sources: lent_libraries(&candidate, node_id),
        type_signatures,
        file,
        companions: files.collect(),
        source,
        header,
        symbols,
        excluded_library_prefixes,
    })
}

/// The body inside the single-file Rust program, located by the compiler's
/// `wirecraft-source` markers. An empty body is generated as one blank line so that the
/// cursor of an empty editor still has a position in the program.
fn rust_body_context(
    document: &Document,
    node_id: &str,
    section: &str,
    draft: Option<&str>,
) -> Result<BodyContext> {
    let root = &document.root_node_id;
    if document.effective(root, "technologyId") != "rust" {
        return Err(Error::new(
            "unsupported",
            "Rust body context needs a design whose root technology is rust",
        ));
    }
    let mut candidate = document.clone();
    let node = candidate
        .nodes
        .get_mut(node_id)
        .ok_or_else(|| Error::new("not_found", "Editor entity does not exist"))?;
    let text = if section == "source" {
        &mut node.text.source
    } else {
        &mut node.text.initialization
    };
    if let Some(draft) = draft {
        *text = draft.into();
    }
    if text.is_empty() {
        *text = " ".into();
    }
    let project = crate::compiler::generate(&candidate, root, Default::default())?;
    let mut files = project.files;
    let index = files
        .iter()
        .position(|f| {
            f.path.ends_with(".rs")
                && f.source_map
                    .iter()
                    .any(|m| m.node_id == node_id && m.section == section)
        })
        .ok_or_else(|| {
            Error::new(
                "unsupported",
                "Selected entity has no editable Rust body in the generated program",
            )
        })?;
    let file = files.remove(index);
    let source = file
        .source_map
        .iter()
        .find(|m| m.node_id == node_id && m.section == section)
        .cloned()
        .unwrap();
    let library =
        crate::analysis::stereotype(&candidate, &candidate.nodes[node_id]) == "ServiceLibrary";
    // Processor and composite bodies sit in a one-line `fn init_…`/`fn run_…` header.
    let header = file
        .content
        .lines()
        .take(source.generated_start_line - 1)
        .filter(|line| line.starts_with("fn "))
        .last()
        .filter(|_| !library)
        .unwrap_or("// Rust service library items (file scope)")
        .to_string();
    let mut symbols = Vec::new();
    for (name, ty) in rust_parameters(&header) {
        let plain = name.trim_start_matches("r#");
        if name == "context" {
            symbols.push(json!({"label":name,"kind":6,"detail":format!("context: {ty}"),"documentation":"Per-node runtime context; keep state between ticks with context.state::<T>(\"key\")."}));
            continue;
        }
        // Self-links appear as `<link>_in`/`<link>_out`; other arguments are named after the link.
        let link = candidate.nodes.values().find(|n| {
            n.link.as_ref().is_some_and(|l| {
                (l.source_node_id == node_id || l.target_node_id == node_id)
                    && [
                        plain.to_string(),
                        format!("{plain}_"),
                        plain.trim_end_matches("_in").into(),
                        plain.trim_end_matches("_out").into(),
                    ]
                    .contains(&crate::c17::identifier(&n.name))
            })
        });
        let Some(link_node) = link else {
            continue;
        };
        let l = link_node.link.as_ref().unwrap();
        let entity = match relation(&candidate, link_node) {
            Some(Relation::Dependency | Relation::Source) => l.source_node_id.clone(),
            _ => link_node.id.clone(),
        };
        let documentation = match relation(&candidate, link_node) {
            Some(Relation::Dependency) => {
                let library = candidate
                    .nodes
                    .get(&l.source_node_id)
                    .map_or(l.source_node_id.as_str(), |n| n.name.as_str());
                format!(
                    "Instance of library {library} lent by dependency {}; call its functions with {name}.function(...).",
                    link_node.name
                )
            }
            Some(Relation::Source) => format!(
                "Materializes the declaration of {}: {name}.get_source(&parameters) replaces ${{name}} markers.",
                candidate
                    .nodes
                    .get(&l.source_node_id)
                    .map_or(l.source_node_id.as_str(), |n| n.name.as_str())
            ),
            _ => {
                // Buffers hold any value; only a modeled struct names the popped type.
                let modeled = candidate
                    .nodes
                    .get(&l.type_definition_id)
                    .map(|n| crate::c17::identifier(&n.name));
                let payload = modeled.clone().unwrap_or_else(|| {
                    if l.type_name.is_empty() {
                        "any value".into()
                    } else {
                        l.type_name.clone()
                    }
                });
                let input = l.target_node_id == node_id
                    && (l.source_node_id != node_id || plain.ends_with("_in"));
                format!(
                    "{} buffer of {payload}: {}\n\n{}:{} → {}:{}",
                    if input { "Input" } else { "Output" },
                    if input {
                        format!(
                            "pop::<{}>({name}) takes the next value",
                            modeled.as_deref().unwrap_or("T")
                        )
                    } else {
                        format!("push({name}, value) sends a value")
                    },
                    l.source_node_id,
                    l.source_port_name,
                    l.target_node_id,
                    l.target_port_name
                )
            }
        };
        symbols.push(json!({"label":name,"kind":6,"nodeId":entity,"detail":format!("{name}: {ty}"),"documentation":documentation}));
    }
    Ok(BodyContext {
        language: "rust".into(),
        file,
        companions: files,
        source,
        header,
        symbols,
        type_signatures: BTreeMap::new(),
        excluded_library_prefixes: BTreeSet::new(),
        declarations: candidate
            .nodes
            .values()
            .filter(|n| n.kind == crate::NodeKind::Type)
            .map(|n| (crate::c17::identifier(&n.name), n.id.clone()))
            .chain(libraries(&candidate).map(|n| (crate::libraries::symbols(n).0, n.id.clone())))
            .collect(),
        library_sources: lent_libraries(&candidate, node_id),
    })
}
/// `name: type` pairs of a one-line generated Rust function header.
fn rust_parameters(header: &str) -> Vec<(String, String)> {
    let Some(list) = header
        .split_once('(')
        .and_then(|(_, rest)| rest.rsplit_once(") ->"))
        .map(|(list, _)| list)
    else {
        return vec![];
    };
    // Generated parameter types contain no commas outside angle brackets.
    let mut parameters = vec![];
    let (mut depth, mut start) = (0i32, 0);
    for (i, c) in list
        .char_indices()
        .chain(std::iter::once((list.len(), ',')))
    {
        match c {
            '<' | '(' => depth += 1,
            '>' | ')' => depth -= 1,
            ',' if depth == 0 => {
                if let Some((name, ty)) = list[start..i].split_once(':') {
                    parameters.push((name.trim().to_string(), ty.trim().to_string()));
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    parameters
}
fn libraries(document: &Document) -> impl Iterator<Item = &crate::Node> {
    document
        .nodes
        .values()
        .filter(|n| !n.is_link() && crate::analysis::stereotype(document, n) == "ServiceLibrary")
}
/// Libraries reaching `node_id` through dependency links, with their declaration text.
fn lent_libraries(document: &Document, node_id: &str) -> Vec<(String, String)> {
    let mut lent = document
        .nodes
        .values()
        .filter(|n| relation(document, n) == Some(Relation::Dependency))
        .filter_map(|n| n.link.as_ref())
        .filter(|l| l.target_node_id == node_id)
        .filter_map(|l| document.nodes.get(&l.source_node_id))
        .map(|n| (n.id.clone(), n.text.source.clone()))
        .collect::<Vec<_>>();
    lent.dedup();
    lent
}
impl BodyContext {
    /// A model location for a position in the generated unit (0-based line, UTF-32
    /// character), in the shape of a mapped diagnostic (`source` is one-based):
    /// - `body`: inside an authored section (`current` when it is the edited one);
    /// - `entity`: a model argument, modeled type or library instance;
    /// - `generated`: runtime or other scaffolding, with the generated line's text.
    ///
    /// `identifier` is the word the request was made on. Library members reach bodies
    /// through generated instances (forwarding methods, function-pointer fields), so a
    /// definition there continues to the function in the lent library's text.
    pub fn locate(&self, line: usize, character: usize, identifier: &str) -> Value {
        let text = self.file.content.lines().nth(line).unwrap_or("").trim();
        let body = |node: &str, section: &str, line: usize, column: usize| json!({"kind":"body","current":node==self.source.node_id&&section==self.source.section,"source":{"nodeId":node,"section":section,"line":line+1,"column":column+1},"positionEncoding":"utf-32","text":text});
        if let Some(m) = self
            .file
            .source_map
            .iter()
            .find(|m| (m.generated_start_line..=m.generated_end_line).contains(&(line + 1)))
        {
            return body(
                &m.node_id,
                &m.section,
                line + 1 - m.generated_start_line + m.source_start_line - 1,
                character.saturating_sub(m.generated_column_offset),
            );
        }
        let entity = |id: &str| json!({"kind":"entity","source":{"nodeId":id,"section":"properties","line":1,"column":1},"positionEncoding":"utf-32","text":text});
        // A model argument is defined in the edited body's generated header; elsewhere
        // the same name is another symbol (a struct field, a local).
        if text == self.header.trim()
            && let Some(id) = self
                .symbols
                .iter()
                .find(|s| s["label"] == identifier)
                .and_then(|s| s["nodeId"].as_str())
        {
            return entity(id);
        }
        let word = |s: &str, name: &str| {
            regex::Regex::new(&format!(r"\b{}\b", regex::escape(name))).is_ok_and(|r| r.is_match(s))
        };
        if let Some(id) = self
            .declarations
            .iter()
            .find(|(name, _)| word(text, &format!("struct {name}")))
            .map(|(_, id)| id)
        {
            return entity(id);
        }
        if !identifier.is_empty() && word(text, identifier) {
            let definition = self.function_definition(identifier);
            for (library, source) in &self.library_sources {
                for (i, line) in source.lines().enumerate() {
                    if let Some(name) = definition.captures(line).and_then(|c| c.get(1)) {
                        let column = line[..name.start()].chars().count();
                        return json!({"kind":"body","current":false,"source":{"nodeId":library,"section":"source","line":i+1,"column":column+1},"positionEncoding":"utf-32","text":line.trim()});
                    }
                }
            }
        }
        json!({"kind":"generated","path":self.file.path,"line":line+1,"column":character+1,"text":text})
    }
    /// A line defining function `name`; capture 1 is the name.
    fn function_definition(&self, name: &str) -> regex::Regex {
        let name = regex::escape(name);
        regex::Regex::new(&if self.language == "rust" {
            format!(
                r#"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]*"\s+)?fn\s+({name})\b"#
            )
        } else {
            format!(r"^[A-Za-z_][\w \t\*]*?\b({name})\s*\([^;]*$")
        })
        .unwrap()
    }
    /// Generated positions (0-based line, UTF-32 character) that name the same library
    /// function as `identifier` under another symbol: instance members (Rust forwarding
    /// methods, C function-pointer fields) and the function in a lent library's text.
    /// References are requested at each, so that a library function and the calls
    /// through its instance are found together.
    pub fn related_positions(&self, identifier: &str) -> Vec<(usize, usize)> {
        if identifier.is_empty() {
            return vec![];
        }
        let name = regex::escape(identifier);
        let member = regex::Regex::new(&if self.language == "rust" {
            format!(r"\bfn\s+({name})\s*(?:<[^>]*>)?\s*\(\s*self\b")
        } else {
            format!(r"\(\s*\*\s*({name})\s*\)\s*\(")
        })
        .unwrap();
        let definition = self.function_definition(identifier);
        let mut positions = vec![];
        for (i, line) in self.file.content.lines().enumerate() {
            let map = self
                .file
                .source_map
                .iter()
                .find(|m| (m.generated_start_line..=m.generated_end_line).contains(&(i + 1)));
            let capture = match map {
                None => member.captures(line),
                Some(m) if self.library_sources.iter().any(|(id, _)| *id == m.node_id) => {
                    definition.captures(line)
                }
                Some(_) => None,
            };
            if let Some(name) = capture.and_then(|c| c.get(1)) {
                positions.push((i, line[..name.start()].chars().count()));
            }
        }
        positions
    }
}
