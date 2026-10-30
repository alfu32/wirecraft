//! Compiler-backed body context. No editor framework or language-server dependency.
//! C17 bodies are analyzed in the typed C17 unit; Rust, Go, Kotlin, JavaScript (Node.js,
//! systemJS) and PHP bodies in the single-file output of their template compiler.
use crate::{
    Document, Error, Result,
    analysis::{Relation, relation},
    c17::C17,
    generation::{GeneratedProject, SourceMap, VirtualFile},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub struct BodyContext {
    /// LSP language of the generated unit: `c`, `rust`, `go`, `kotlin`, `javascript` or
    /// `php`.
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
    /// Declaration texts of every service library, by entity id.
    pub libraries: BTreeMap<String, String>,
    /// `(consumer, argument, library)` for each dependency link: the consumer's bodies
    /// reach the library's functions through the argument named after the link.
    pub library_arguments: Vec<(String, String, String)>,
}
impl BodyContext {
    pub fn project(&self, name: &str) -> GeneratedProject {
        GeneratedProject {
            name: name.into(),
            files: std::iter::once(self.file.clone())
                .chain(self.companions.iter().cloned())
                .collect(),
            capability: if self.language == "c" {
                "c17-editor-context".into()
            } else {
                format!("{}-editor-context", self.language)
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
    match body_language(document, node_id, section) {
        Some("c") => c17_body_context(document, node_id, section, draft),
        Some(language) => template_body_context(document, node_id, section, draft, language),
        None => Err(Error::new(
            "unsupported",
            "Compiler body context supports Declaration and Instantiation sections of C17, Rust, Go, Kotlin, JavaScript and PHP entities",
        )),
    }
}
/// The generated-program language (`c` or `rust`) whose compiler and language server
/// also check, complete, color and describe this body: Declaration and Instantiation
/// sections of C17 or Rust entities.
pub fn compiler_language(
    document: &Document,
    node_id: &str,
    section: &str,
) -> Option<&'static str> {
    body_language(document, node_id, section).filter(|l| ["c", "rust"].contains(l))
}
/// The generated-program language of a Declaration or Instantiation body, for every
/// technology with a body context: `c`, `rust`, `go`, `kotlin`, `javascript` (Node.js
/// and systemJS) or `php`. Definitions and references are served for all of them.
pub fn body_language(document: &Document, node_id: &str, section: &str) -> Option<&'static str> {
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
        ("go", "go") => Some("go"),
        ("kotlin", "kotlin-jvm") => Some("kotlin"),
        ("javascript", "nodejs" | "systemjs" | "quickjs") => Some("javascript"),
        ("php", "php") => Some("php"),
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
        libraries: all_libraries(&candidate),
        library_arguments: library_arguments(&candidate),
        type_signatures,
        file,
        companions: files.collect(),
        source,
        header,
        symbols,
        excluded_library_prefixes,
    })
}

/// Conventions of one template language's single-file program.
struct Dialect {
    extension: &'static str,
    /// Keyword of the one-line `init_…`/`run_…` function headers around bodies.
    function: &'static str,
    /// Header shown for service libraries, whose items are at file scope.
    scope: &'static str,
    /// Member access through a library argument, as documented to the author.
    member: &'static str,
}
fn dialect(language: &str) -> Dialect {
    let (extension, function, scope, member) = match language {
        "rust" => (
            "rs",
            "fn ",
            "// Rust service library items (file scope)",
            ".",
        ),
        "go" => (
            "go",
            "func ",
            "// Go service library items (file scope)",
            ".",
        ),
        "kotlin" => (
            "kt",
            "fun ",
            "// Kotlin service library items (file scope)",
            ".",
        ),
        "php" => (
            "php",
            "function ",
            "// PHP service library items (file scope)",
            "->",
        ),
        _ => (
            "js",
            "function ",
            "// JavaScript service library items (file scope)",
            ".",
        ),
    };
    Dialect {
        extension,
        function,
        scope,
        member,
    }
}
/// The body inside the single-file program of a template compiler, located by the
/// compiler's `wirecraft-source` markers. An empty body is generated as one blank line so
/// that the cursor of an empty editor still has a position in the program.
fn template_body_context(
    document: &Document,
    node_id: &str,
    section: &str,
    draft: Option<&str>,
    language: &str,
) -> Result<BodyContext> {
    let root = &document.root_node_id;
    let technology = document.effective(node_id, "technologyId");
    if document.effective(root, "technologyId") != technology {
        return Err(Error::new(
            "unsupported",
            format!("Body context needs a design whose root technology is {technology}"),
        ));
    }
    let rust = language == "rust";
    let dialect = dialect(language);
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
            f.path.ends_with(&format!(".{}", dialect.extension))
                && f.source_map
                    .iter()
                    .any(|m| m.node_id == node_id && m.section == section)
        })
        .ok_or_else(|| {
            Error::new(
                "unsupported",
                "Selected entity has no editable body in the generated program",
            )
        })?;
    let file = files.remove(index);
    match language {
        // A build script would make the Kotlin server resolve the classpath with Gradle;
        // the single file needs only the standard library.
        "kotlin" => files.retain(|f| !f.path.ends_with(".gradle.kts")),
        // gopls loads packages through a module.
        "go" if !files.iter().any(|f| f.path == "go.mod") => files.push(VirtualFile {
            path: "go.mod".into(),
            content: "module wirecraft/editor\n\ngo 1.21\n".into(),
            binary_content: None,
            source_map: vec![],
        }),
        _ => {}
    }
    let source = file
        .source_map
        .iter()
        .find(|m| m.node_id == node_id && m.section == section)
        .cloned()
        .unwrap();
    let library =
        crate::analysis::stereotype(&candidate, &candidate.nodes[node_id]) == "ServiceLibrary";
    // Processor and composite bodies sit in a one-line `init_…`/`run_…` header.
    let header = file
        .content
        .lines()
        .take(source.generated_start_line - 1)
        .filter(|line| line.starts_with(dialect.function))
        .last()
        .filter(|_| !library)
        .unwrap_or(dialect.scope)
        .to_string();
    let mut symbols = Vec::new();
    for (name, ty) in parameters(language, &header) {
        let plain = name.trim_start_matches("r#");
        // PHP arguments are variables; the identifier a request is made on has no `$`.
        let shown = if language == "php" {
            format!("${name}")
        } else {
            name.clone()
        };
        let detail = if ty.is_empty() {
            shown.clone()
        } else if language == "go" || language == "php" {
            format!("{shown} {ty}")
        } else {
            format!("{shown}: {ty}")
        };
        if name == "context" {
            symbols.push(json!({"label":name,"kind":6,"detail":detail,"documentation":if rust {"Per-node runtime context; keep state between ticks with context.state::<T>(\"key\")."} else {"Per-node runtime context shared with the generated runtime."}}));
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
        let provider = candidate
            .nodes
            .get(&l.source_node_id)
            .map_or(l.source_node_id.as_str(), |n| n.name.as_str());
        let documentation = match relation(&candidate, link_node) {
            Some(Relation::Dependency) => format!(
                "Instance of library {provider} lent by dependency {}; call its functions with {shown}{}function(...).",
                link_node.name, dialect.member
            ),
            Some(Relation::Source) if rust => format!(
                "Materializes the declaration of {provider}: {name}.get_source(&parameters) replaces ${{name}} markers."
            ),
            Some(Relation::Source) => {
                format!("Materializes the declaration of {provider}, replacing ${{name}} markers.")
            }
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
                let usage = if !rust {
                    String::new()
                } else if input {
                    format!(
                        ": pop::<{}>({name}) takes the next value",
                        modeled.as_deref().unwrap_or("T")
                    )
                } else {
                    format!(": push({name}, value) sends a value")
                };
                format!(
                    "{} buffer of {payload}{usage}\n\n{}:{} → {}:{}",
                    if input { "Input" } else { "Output" },
                    l.source_node_id,
                    l.source_port_name,
                    l.target_node_id,
                    l.target_port_name
                )
            }
        };
        symbols.push(json!({"label":name,"kind":6,"nodeId":entity,"detail":detail,"documentation":documentation}));
    }
    Ok(BodyContext {
        language: language.into(),
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
            .chain(libraries(&candidate).flat_map(|n| {
                let (ty, value) = crate::libraries::symbols(n);
                [(ty, n.id.clone()), (value, n.id.clone())]
            }))
            .collect(),
        library_sources: lent_libraries(&candidate, node_id),
        libraries: all_libraries(&candidate),
        library_arguments: library_arguments(&candidate),
    })
}
/// `(name, type)` pairs of a one-line generated function header: `name: type` (Rust,
/// Kotlin), `name type` (Go), `name = default` (JavaScript) or `type &$name` (PHP).
fn parameters(language: &str, header: &str) -> Vec<(String, String)> {
    let Some(open) = header.find('(') else {
        return vec![];
    };
    // Generated parameter types contain no commas outside brackets.
    let mut parts = vec![];
    let (mut depth, mut start) = (0i32, open + 1);
    for (i, c) in header[open + 1..]
        .char_indices()
        .map(|(i, c)| (i + open + 1, c))
    {
        match c {
            '>' if header[..i].ends_with('-') => {}
            '<' | '(' | '[' | '{' => depth += 1,
            ')' if depth == 0 => {
                parts.push(&header[start..i]);
                break;
            }
            '>' | ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&header[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .filter_map(|part| {
            let (name, ty) = match language {
                "go" => part.split_once(char::is_whitespace)?,
                "javascript" => (part.split('=').next()?, ""),
                "php" => {
                    let part = part.split('=').next()?.trim();
                    let (ty, name) = part.rsplit_once(char::is_whitespace).unwrap_or(("", part));
                    (name.trim_start_matches(['&', '.']).strip_prefix('$')?, ty)
                }
                _ => part.split_once(':')?,
            };
            Some((name.trim().to_string(), ty.trim().to_string()))
        })
        .collect()
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
fn all_libraries(document: &Document) -> BTreeMap<String, String> {
    libraries(document)
        .map(|n| (n.id.clone(), n.text.source.clone()))
        .collect()
}
fn library_arguments(document: &Document) -> Vec<(String, String, String)> {
    document
        .nodes
        .values()
        .filter(|n| relation(document, n) == Some(Relation::Dependency))
        .filter_map(|n| {
            let l = n.link.as_ref()?;
            Some((
                l.target_node_id.clone(),
                crate::c17::identifier(&n.name),
                l.source_node_id.clone(),
            ))
        })
        .collect()
}
/// The identifier around a body offset (start and end byte offsets); a PHP variable's
/// `$` is not part of it.
pub fn identifier_at(text: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(text.len());
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let start = text[..offset]
        .char_indices()
        .rev()
        .take_while(|(_, c)| word(*c))
        .last()
        .map_or(offset, |(i, _)| i);
    let end = offset
        + text[offset..]
            .find(|c: char| !word(c))
            .unwrap_or(text.len() - offset);
    (start, end)
}
fn regex(pattern: &str) -> regex::Regex {
    regex::Regex::new(pattern).unwrap()
}
impl BodyContext {
    /// The authored-body location of a generated position (0-based line, UTF-32
    /// character), when it is inside a mapped section.
    fn mapped(&self, line: usize, character: usize) -> Option<Value> {
        let m = self
            .file
            .source_map
            .iter()
            .find(|m| (m.generated_start_line..=m.generated_end_line).contains(&(line + 1)))?;
        let text = self.file.content.lines().nth(line).unwrap_or("").trim();
        Some(
            json!({"kind":"body","current":m.node_id==self.source.node_id&&m.section==self.source.section,"source":{"nodeId":m.node_id,"section":m.section,"line":line+1-m.generated_start_line+m.source_start_line,"column":character.saturating_sub(m.generated_column_offset)+1},"positionEncoding":"utf-32","text":text}),
        )
    }
    fn entity(id: &str, text: &str) -> Value {
        json!({"kind":"entity","source":{"nodeId":id,"section":"properties","line":1,"column":1},"positionEncoding":"utf-32","text":text})
    }
    /// A model location for a position in the generated unit (0-based line, UTF-32
    /// character), in the shape of a mapped diagnostic (`source` is one-based):
    /// - `body`: inside an authored section (`current` when it is the edited one);
    /// - `entity`: a model argument, modeled type or library instance;
    /// - `generated`: runtime or other scaffolding, with the generated line's text.
    ///
    /// `identifier` is the word the request was made on. Library members reach bodies
    /// through generated instances (forwarding methods, function-pointer fields, object
    /// properties), so a definition there continues to the function in the library.
    pub fn locate(&self, line: usize, character: usize, identifier: &str) -> Value {
        if let Some(body) = self.mapped(line, character) {
            return body;
        }
        let text = self.file.content.lines().nth(line).unwrap_or("").trim();
        // A model argument is defined in the edited body's generated header; elsewhere
        // the same name is another symbol (a struct field, a local).
        if text == self.header.trim()
            && let Some(id) = self
                .symbols
                .iter()
                .find(|s| s["label"] == identifier)
                .and_then(|s| s["nodeId"].as_str())
        {
            return Self::entity(id, text);
        }
        if let Some(id) = self
            .declarations
            .iter()
            .find(|(name, _)| self.declares(text, name))
            .map(|(_, id)| id)
        {
            return Self::entity(id, text);
        }
        let word =
            |s: &str, name: &str| regex(&format!(r"\b{}\b", regex::escape(name))).is_match(s);
        if !identifier.is_empty() && word(text, identifier) {
            for (library, _) in &self.library_sources {
                if let Some(definition) = self.library_definition(library, identifier) {
                    return definition;
                }
            }
        }
        json!({"kind":"generated","path":self.file.path,"line":line+1,"column":character+1,"text":text})
    }
    /// Whether a generated line declares the modeled type or library instance `name`.
    fn declares(&self, line: &str, name: &str) -> bool {
        let name = regex::escape(name);
        regex(&match self.language.as_str() {
            "go" => format!(r"\btype\s+{name}\b"),
            "kotlin" => format!(r"\b(?:class|object|typealias)\s+{name}\b"),
            "javascript" => {
                format!(r"(?:@typedef\s*\{{[^}}]*\}}\s*|\b(?:const|let|var|class)\s+){name}\b")
            }
            "php" => format!(r"(?:\bclass\s+{name}\b|\${name}\s*=)"),
            _ => format!(r"\bstruct\s+{name}\b"),
        })
        .is_match(line)
    }
    /// A line defining function `name`; capture 1 is the name.
    fn function_definition(&self, name: &str) -> regex::Regex {
        let name = regex::escape(name);
        regex(&match self.language.as_str() {
            "rust" => format!(
                r#"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]*"\s+)?fn\s+({name})\b"#
            ),
            "go" => format!(r"^\s*func\s+(?:\([^)]*\)\s*)?({name})\s*[\[(]"),
            "kotlin" => format!(
                r"^\s*(?:(?:private|public|internal|protected|inline|suspend|tailrec|operator|infix)\s+)*fun\s+(?:<[^>]*>\s*)?({name})\s*\("
            ),
            "javascript" => format!(
                r"^\s*(?:export\s+)?(?:async\s+)?(?:function\s*\*?\s*({name})\s*\(|(?:const|let|var)\s+({name})\s*=\s*(?:async\s+)?(?:function\b|\([^)]*\)\s*=>|\w+\s*=>))"
            ),
            "php" => format!(
                r"^\s*(?:(?:public|private|protected|static|final|abstract)\s+)*function\s+&?\s*({name})\s*\("
            ),
            _ => format!(r"^[A-Za-z_][\w \t\*]*?\b({name})\s*\([^;]*$"),
        })
    }
    /// Lines of an authored section as `(generated line, column offset, text)`: from the
    /// generated unit when it renders the section, else the stored text.
    fn section_lines(&self, node: &str, section: &str) -> Vec<(Option<usize>, String)> {
        let lines: Vec<&str> = self.file.content.lines().collect();
        if let Some(m) = self
            .file
            .source_map
            .iter()
            .find(|m| m.node_id == node && m.section == section)
        {
            return (m.generated_start_line - 1..m.generated_end_line)
                .filter_map(|i| Some((Some(i), lines.get(i)?.to_string())))
                .collect();
        }
        self.libraries
            .get(node)
            .filter(|_| section == "source")
            .map(|text| text.lines().map(|l| (None, l.to_string())).collect())
            .unwrap_or_default()
    }
    /// The location of a capture in a section line from `section_lines`.
    fn section_location(
        &self,
        node: &str,
        index: usize,
        generated: Option<usize>,
        line: &str,
        start: usize,
    ) -> Value {
        let column = line[..start].chars().count();
        generated
            .and_then(|g| self.mapped(g, column))
            .unwrap_or_else(|| json!({"kind":"body","current":false,"source":{"nodeId":node,"section":"source","line":index+1,"column":column+1},"positionEncoding":"utf-32","text":line.trim()}))
    }
    /// Where library `library` defines function `name`.
    fn library_definition(&self, library: &str, name: &str) -> Option<Value> {
        let definition = self.function_definition(name);
        self.section_lines(library, "source")
            .iter()
            .enumerate()
            .find_map(|(i, (generated, line))| {
                let name = definition.captures(line)?.iter().skip(1).flatten().next()?;
                Some(self.section_location(library, i, *generated, line, name.start()))
            })
    }
    /// The library whose function the identifier at `[start, end)` of the edited body
    /// names: a call through a lent library's argument (`math.f`, `math->f`,
    /// `$math->f`), or a function the edited library itself defines. `None` for other
    /// member accesses.
    fn library_member(&self, text: &str, start: usize, end: usize) -> Option<String> {
        let name = &text[start..end];
        let before = text[..start].trim_end();
        let receiver = ["?.", "->", "::", "."]
            .iter()
            .find_map(|access| before.strip_suffix(access))
            .map(|rest| {
                let rest = rest.trim_end();
                let (s, e) = identifier_at(rest, rest.len());
                &rest[s..e]
            });
        match receiver {
            Some(receiver) => self
                .library_arguments
                .iter()
                .find(|(consumer, argument, _)| {
                    *consumer == self.source.node_id && argument == receiver
                })
                .map(|(_, _, library)| library.clone()),
            None => self
                .libraries
                .contains_key(&self.source.node_id)
                .then(|| self.source.node_id.clone())
                .filter(|library| self.library_definition(library, name).is_some()),
        }
    }
    /// Definitions found from the model alone, without a language server: a library
    /// function called through a lent instance, a model argument of the edited body, a
    /// modeled type or a library instance named in it.
    pub fn model_definitions(&self, text: &str, offset: usize) -> Vec<Value> {
        let (start, end) = identifier_at(text, offset);
        let name = &text[start..end];
        if name.is_empty() {
            return vec![];
        }
        let line = text[..start].rsplit('\n').next().unwrap_or("");
        let line = format!("{line}{}", text[start..].split('\n').next().unwrap_or(""));
        let before = text[..start].trim_end();
        if ["?.", "->", "::", "."].iter().any(|a| before.ends_with(a)) {
            return self
                .library_member(text, start, end)
                .and_then(|library| self.library_definition(&library, name))
                .into_iter()
                .collect();
        }
        if let Some(library) = self.library_member(text, start, end) {
            return self
                .library_definition(&library, name)
                .into_iter()
                .collect();
        }
        self.symbols
            .iter()
            .find(|s| s["label"] == name)
            .and_then(|s| s["nodeId"].as_str())
            .or_else(|| self.declarations.get(name).map(String::as_str))
            .map(|id| Self::entity(id, line.trim()))
            .into_iter()
            .collect()
    }
    /// References found from the model alone: for a library function, its definition,
    /// the calls inside its library and every call through an argument lent by a
    /// dependency link, in all consumers' bodies.
    pub fn model_references(&self, text: &str, offset: usize) -> Vec<Value> {
        let (start, end) = identifier_at(text, offset);
        let name = &text[start..end];
        let Some(library) = (!name.is_empty())
            .then(|| self.library_member(text, start, end))
            .flatten()
        else {
            return vec![];
        };
        let escaped = regex::escape(name);
        let definition = self.function_definition(name);
        let call = regex(&format!(r"(?:^|[^\w.$>:])({escaped})\s*\("));
        let mut locations = vec![];
        for (i, (generated, line)) in self.section_lines(&library, "source").iter().enumerate() {
            let capture = definition
                .captures(line)
                .and_then(|c| c.iter().skip(1).flatten().next())
                .or_else(|| call.captures(line).and_then(|c| c.get(1)));
            if let Some(capture) = capture {
                locations.push(self.section_location(
                    &library,
                    i,
                    *generated,
                    line,
                    capture.start(),
                ));
            }
        }
        for (consumer, argument, _) in self
            .library_arguments
            .iter()
            .filter(|(_, _, l)| *l == library)
        {
            let call = regex(&format!(
                r"(?:^|[^\w$])\$?{}\s*(?:\?\.|\.|->|::)\s*({escaped})\b",
                regex::escape(argument)
            ));
            for section in ["initialization", "source"] {
                for (i, (generated, line)) in
                    self.section_lines(consumer, section).iter().enumerate()
                {
                    for capture in call.captures_iter(line).filter_map(|c| c.get(1)) {
                        if let Some(generated) = generated {
                            let column = line[..capture.start()].chars().count();
                            locations.extend(self.mapped(*generated, column));
                        } else {
                            locations.push(self.section_location(
                                consumer,
                                i,
                                None,
                                line,
                                capture.start(),
                            ));
                        }
                    }
                }
            }
        }
        locations
    }
    /// Generated positions (0-based line, UTF-32 character) that name the same library
    /// function as `identifier` under another symbol: instance members (Rust forwarding
    /// methods, C and Go function fields, Kotlin properties, JavaScript object entries,
    /// PHP forwarding methods) and the function in a lent library's text. References
    /// are requested at each, so that a library function and the calls through its
    /// instance are found together.
    pub fn related_positions(&self, identifier: &str) -> Vec<(usize, usize)> {
        if identifier.is_empty() {
            return vec![];
        }
        let name = regex::escape(identifier);
        let member = regex(&match self.language.as_str() {
            "rust" => format!(r"\bfn\s+({name})\s*(?:<[^>]*>)?\s*\(\s*self\b"),
            "go" => format!(r"^\s*({name})\s+func\s*\("),
            "kotlin" => format!(r"^\s*val\s+({name})\s*="),
            "javascript" => format!(r"Object\.freeze\(\{{[^}}]*?\b({name})\b"),
            "php" => format!(r"\bpublic function ({name})\(mixed \.\.\."),
            _ => format!(r"\(\s*\*\s*({name})\s*\)\s*\("),
        });
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
            if let Some(name) = capture.and_then(|c| c.iter().skip(1).flatten().next()) {
                positions.push((i, line[..name.start()].chars().count()));
            }
        }
        positions
    }
}
