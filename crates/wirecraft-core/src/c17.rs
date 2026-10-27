//! C17 topology compiler. Editable bodies stay separate from runtime/graph assembly.
use crate::{
    Document, Error, Node, NodeKind, Result,
    analysis::{Relation, relation, stereotype},
    generation::{GeneratedProject, SourceMap, VirtualFile},
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub struct C17;
struct Wire {
    argument: String,
    allocation: String,
    source: String,
    target: String,
    parent: String,
    ctype: String,
}
/// Typed source-capability facade, exposed only to the consuming entity.
struct SourceCapability {
    argument: String,
    dependency: String,
    capability_type: String,
    provider: String,
    target: String,
}
/// Generated function parameter: name, declared C type and caller expression.
struct Argument {
    name: String,
    ctype: String,
    value: String,
}
struct Emitter {
    source: String,
    maps: Vec<SourceMap>,
    next_line: usize,
}
impl Emitter {
    fn new() -> Self {
        Self {
            source: String::new(),
            maps: vec![],
            next_line: 1,
        }
    }
    fn line(&mut self, text: &str) {
        self.next_line += text.as_bytes().iter().filter(|&&b| b == 10).count() + 1;
        self.source.push_str(text);
        self.source.push('\n');
    }
    fn authored(&mut self, node: &Node, section: &str, text: &str, indent: &str) {
        let start = self.next_line;
        for line in text.split('\n') {
            self.line(&format!("{indent}{line}"));
        }
        self.maps.push(SourceMap {
            node_id: node.id.clone(),
            section: section.into(),
            generated_start_line: start,
            generated_end_line: start + text.split('\n').count() - 1,
            source_start_line: 1,
            generated_column_offset: indent.chars().count(),
        });
    }
}
impl C17 {
    pub fn generate(document: &Document, root: &str) -> Result<GeneratedProject> {
        document.ensure_valid()?;
        document.node(root)?;
        let scope = document.descendants(root);
        let mut symbols = BTreeMap::new();
        for (i, n) in document.nodes.values().filter(|n| !n.is_link()).enumerate() {
            symbols.insert(
                n.id.clone(),
                format!("{}_{}", identifier(&n.name).to_lowercase(), i + 1),
            );
        }
        let mut libraries = BTreeSet::new();
        let mut executables = BTreeSet::new();
        for id in &scope {
            let n = document.node(id)?;
            if document.effective(id, "technologyId") == "file-export" {
                if !n.children.is_empty() || n.is_link() {
                    return Err(unsupported(
                        n,
                        "C file exports must be standalone leaf files",
                    ));
                }
                continue;
            }
            if !["c-native", "none"].contains(&document.effective(id, "technologyId").as_str()) {
                return Err(unsupported(
                    n,
                    "Cross-technology C compilation requires an explicit FFI/aggregate adapter",
                ));
            }
            if n.is_link() || matches!(n.kind, NodeKind::Note) {
                continue;
            }
            if !["none", "single-file"]
                .contains(&document.effective(id, "fileLayoutStrategyId").as_str())
            {
                return Err(unsupported(n, "C17 supports only single-file layout"));
            }
            if n.name.starts_with('@') {
                return Err(unsupported(
                    n,
                    "Graph template overrides are not yet migrated",
                ));
            }
            if n.is_link() || matches!(n.kind, NodeKind::Type | NodeKind::Note) {
                continue;
            }
            if stereotype(document, n) == "ServiceLibrary" {
                libraries.insert(id.clone());
            } else {
                if n.text.source.lines().any(|l| {
                    l.trim_start().starts_with("#include")
                        || l.trim_start().starts_with("# include")
                }) {
                    return Err(unsupported(
                        n,
                        "Include directives belong in a service-library declaration",
                    ));
                }
                executables.insert(id.clone());
            }
        }
        if !executables.contains(root) {
            return Err(Error::new(
                "invalid_input",
                "Compile an executable processing entity or composite",
            ));
        }
        let mut wires = BTreeMap::new();
        let mut allocated = BTreeSet::new();
        let mut payloads: BTreeMap<String, (String, String)> = BTreeMap::new();
        let mut capabilities = vec![];
        let mut sources = BTreeMap::new();
        for id in &scope {
            let n = document.node(id)?;
            let Some(link) = &n.link else {
                continue;
            };
            if !scope.contains(&link.source_node_id) || !scope.contains(&link.target_node_id) {
                return Err(unsupported(
                    n,
                    "Selected C scope crosses a link boundary; compile its enclosing composite",
                ));
            }
            match relation(document, n) {
                Some(Relation::TypeUsage) => continue,
                Some(Relation::Dependency) => {
                    if !libraries.contains(&link.source_node_id) {
                        return Err(unsupported(
                            n,
                            "Library links must originate in a service library",
                        ));
                    }
                    capabilities.push(n);
                    continue;
                }
                Some(Relation::Runnable) => {
                    return Err(unsupported(
                        n,
                        "C17 cannot expose a type-safe runtime-compiled runnable capability; use src or an explicit toolchain adapter",
                    ));
                }
                Some(Relation::Source) => {
                    let provider = document.node(&link.source_node_id)?;
                    if provider.is_link()
                        || matches!(provider.kind, NodeKind::Type | NodeKind::Note)
                    {
                        return Err(unsupported(
                            n,
                            "Source capabilities must originate in an entity with a declaration",
                        ));
                    }
                    if !executables.contains(&link.target_node_id) {
                        return Err(unsupported(
                            n,
                            "Source capabilities must be consumed by an executable C entity",
                        ));
                    }
                    let argument = identifier(&n.name);
                    if reserved(&argument) {
                        return Err(unsupported(n, "Link argument is a reserved C identifier"));
                    }
                    // Same index as the baseline dependency symbol: position among the
                    // provider's outgoing library/source/runnable capability links.
                    let position = provider
                        .outgoing_links
                        .iter()
                        .filter(|id| {
                            document.nodes.get(*id).is_some_and(|n| {
                                matches!(
                                    relation(document, n),
                                    Some(
                                        Relation::Dependency
                                            | Relation::Source
                                            | Relation::Runnable
                                    )
                                )
                            })
                        })
                        .position(|id| id == &n.id)
                        .unwrap_or(0)
                        + 1;
                    let mut dependency = format!("{argument}{position}");
                    if !allocated.insert(dependency.clone()) {
                        dependency = format!("{dependency}_{}", identifier(&n.id));
                        allocated.insert(dependency.clone());
                    }
                    let mut capability_type = dependency.clone();
                    capability_type[..1].make_ascii_uppercase();
                    sources.insert(
                        id.clone(),
                        SourceCapability {
                            argument,
                            capability_type: format!("{capability_type}Capability"),
                            dependency,
                            provider: provider.id.clone(),
                            target: link.target_node_id.clone(),
                        },
                    );
                    continue;
                }
                _ => {}
            }
            if !executables.contains(&link.source_node_id)
                || !executables.contains(&link.target_node_id)
            {
                return Err(unsupported(
                    n,
                    "Data links must connect executable C entities",
                ));
            }
            let ctype = if let Some(expression) = &link.type_expression {
                expression_type(document, expression)?
            } else if !link.type_definition_id.trim().is_empty() {
                type_name(document, &link.type_definition_id)?
            } else if !link.type_name.trim().is_empty()
                || !link.payload_definition.trim().is_empty()
            {
                if !valid_identifier(&link.type_name) || link.payload_definition.trim().is_empty() {
                    return Err(unsupported(
                        n,
                        "A wire type requires a valid typeName and payloadDefinition",
                    ));
                }
                let normalized = link
                    .payload_definition
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                if let Some((previous, _)) = payloads.get(&link.type_name) {
                    if previous != &normalized {
                        return Err(unsupported(n, "Conflicting wire payload definitions"));
                    }
                } else {
                    payloads.insert(link.type_name.clone(), (normalized, id.clone()));
                }
                link.type_name.clone()
            } else {
                "void *".into()
            };
            let argument = identifier(&n.name);
            if reserved(&argument) {
                return Err(unsupported(n, "Link argument is a reserved C identifier"));
            }
            let source = document.node(&link.source_node_id)?;
            let position = source
                .outgoing_links
                .iter()
                .filter(|id| {
                    document.nodes.get(*id).is_some_and(|n| {
                        matches!(
                            relation(document, n),
                            Some(Relation::Principal | Relation::Error)
                        )
                    })
                })
                .position(|id| id == &n.id)
                .unwrap_or(0)
                + 1;
            let mut allocation = format!("{argument}{position}");
            if !allocated.insert(allocation.clone()) {
                allocation = format!("{allocation}_{}", identifier(&n.id));
                allocated.insert(allocation.clone());
            }
            wires.insert(
                id.clone(),
                Wire {
                    argument,
                    allocation,
                    source: link.source_node_id.clone(),
                    target: link.target_node_id.clone(),
                    parent: n.parent_id.clone().unwrap_or(root.into()),
                    ctype,
                },
            );
        }
        let mut out = Emitter::new();
        out.line("/* Generated by Wirecraft C17; authored source is mapped separately. */\n#include <stddef.h>\n#include <stdbool.h>\n#include <stdint.h>\n#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include <signal.h>\n#include <unistd.h>");
        out.line(include_str!("../resources/c_runtime.c"));
        for (_, id) in payloads.values() {
            let n = document.node(id)?;
            out.authored(
                n,
                "link.payloadDefinition",
                &n.link.as_ref().unwrap().payload_definition,
                "",
            );
        }
        emit_types(document, &scope, &mut out)?;
        for id in library_order(&libraries, &capabilities) {
            let n = document.node(&id)?;
            out.authored(n, "source", &n.text.source, "");
        }
        // One exporter struct and one shared instance per library that is depended upon;
        // each dependency link lends that instance to its consumer as a named argument.
        let mut injected = vec![];
        let mut exported = BTreeSet::new();
        for n in &capabilities {
            let link = n.link.as_ref().unwrap();
            let library = document.node(&link.source_node_id)?;
            // Library-to-library links share file scope; only executables receive instances.
            if executables.contains(&link.target_node_id) {
                if exported.insert(library.id.clone()) {
                    out.line(&library_exporter(library));
                }
                let argument = identifier(&n.name);
                if reserved(&argument) {
                    return Err(unsupported(n, "Link argument is a reserved C identifier"));
                }
                injected.push((
                    link.target_node_id.clone(),
                    Argument {
                        name: argument,
                        ctype: format!("const {} *", library_type(library)),
                        value: format!("&{}", library_instance(library)),
                    },
                ));
            }
        }
        // Compatibility: bodies written for the former `link__function` aliases still compile.
        let bodies = scope
            .iter()
            .filter_map(|id| document.nodes.get(id))
            .filter(|n| !n.is_link())
            .map(|n| format!("{}\n{}", n.text.source, n.text.initialization))
            .collect::<Vec<_>>();
        let mut aliases = BTreeSet::new();
        for n in &capabilities {
            let prefix = format!("{}__", identifier(&n.name));
            if !bodies.iter().any(|body| body.contains(&prefix)) {
                continue;
            }
            for function in library_functions(document, n)? {
                if aliases.insert(function.name) {
                    out.line(&function.declaration);
                }
            }
        }
        for capability in sources.values() {
            injected.push((
                capability.target.clone(),
                Argument {
                    name: capability.argument.clone(),
                    ctype: format!("{} *", capability.capability_type),
                    value: format!("&{}", capability.dependency),
                },
            ));
        }
        for (id, capability) in &sources {
            let n = document.node(id)?;
            let provider = document.node(&capability.provider)?;
            out.line(&format!(
                "/* Source capability {} from {} */\ntypedef struct {cap} {{\n    const char *template_source;\n    char *(*getSource)(\n        const struct {cap} *capability,\n        const threadwork_build_parameter *parameters,\n        size_t parameter_count);\n}} {cap};\n\nstatic char *{dep}_get_source(\n    const {cap} *capability,\n    const threadwork_build_parameter *parameters,\n    size_t parameter_count)\n{{\n    return threadwork_materialize_source(capability->template_source, parameters, parameter_count);\n}}\n\nstatic {cap} {dep} = {{\n    .template_source = {literal},\n    .getSource = {dep}_get_source\n}};",
                comment(&n.name),
                comment(&provider.name),
                cap = capability.capability_type,
                dep = capability.dependency,
                literal = string_literal(&provider.text.source),
            ));
        }
        for (id, wire) in &wires {
            let n = document.node(id)?;
            out.line(&format!("/* Transport {} */", comment(&n.name)));
            for side in ["a", "b"] {
                out.line(&format!(
                    "static threadwork_buffer {}_{side}_port = {{ .element_size = sizeof({}) }};",
                    wire.allocation, wire.ctype
                ));
            }
            out.line(&format!("static threadwork_error_t transport_{}(threadwork_buffer *a, threadwork_buffer *b) {{ return threadwork_buffer_transport(a,b); }}",wire.allocation));
        }
        for id in &executables {
            let arguments = arguments(id, &wires, &injected)?;
            for phase in ["init", "run"] {
                out.line(&format!(
                    "static threadwork_error_t tw_{phase}_{}({});",
                    symbols[id],
                    parameters(&arguments)
                ));
            }
        }
        // Bodies may be emitted in ID order because every generated function has a prototype.
        for id in &executables {
            let n = document.node(id)?;
            let args = arguments(id, &wires, &injected)?;
            for (phase, section, text) in [
                ("init", "initialization", &n.text.initialization),
                ("run", "source", &n.text.source),
            ] {
                out.line(&format!("/** Threadwork entity {} @phase {phase} */\nstatic threadwork_error_t tw_{phase}_{}({})\n{{\n    (void)context;",comment(&n.name),symbols[id],parameters(&args)));
                for argument in &args {
                    out.line(&format!("    (void){};", argument.name));
                }
                out.authored(n, section, text, "    ");
                for child in &n.children {
                    if executables.contains(child) {
                        let child_args = arguments(child, &wires, &injected)?;
                        let call_args = child_args
                            .iter()
                            .map(|argument| format!(", {}", argument.value))
                            .collect::<String>();
                        out.line(&format!("    if (tw_{phase}_{}(context{call_args}) != THREADWORK_OK) return THREADWORK_ERROR;",symbols[child]));
                    }
                }
                if phase == "run" {
                    for w in wires.values().filter(|w| w.parent == *id) {
                        out.line(&format!("    if (threadwork_buffer_count(&{a}_a_port)>0U) {{\n        if (transport_{a}(&{a}_a_port,&{a}_b_port)!=THREADWORK_OK) return THREADWORK_ERROR;\n        if (threadwork_runner__record_transit(&threadwork_runner)!=THREADWORK_OK) return THREADWORK_ERROR;\n    }}",a=w.allocation));
                    }
                }
                out.line("    return THREADWORK_OK;\n}");
            }
        }
        let root_args = arguments(root, &wires, &injected)?;
        let root_args = root_args
            .iter()
            .map(|argument| format!(", {}", argument.value))
            .collect::<String>();
        out.line(&format!("int main(void) {{\n    threadwork_context context;\n    threadwork_error_t status=threadwork_runner__init(&threadwork_runner);\n    int running=1, recent_transit=0;\n    if(status!=THREADWORK_OK) return EXIT_FAILURE;\n    if(threadwork_context_init(&context)!=THREADWORK_OK) return EXIT_FAILURE;\n    status=threadwork_runner__install_shutdown_signal_handlers(&threadwork_runner);\n    if(status==THREADWORK_OK) status=tw_init_{symbol}(&context{root_args});\n    while(status==THREADWORK_OK) {{\n        status=threadwork_runner__is_running(&threadwork_runner,&running);\n        if(status!=THREADWORK_OK || !running) break;\n        status=tw_run_{symbol}(&context{root_args});\n    }}\n    if(status==THREADWORK_OK) {{\n        status=threadwork_runner__begin_shutdown_drain(&threadwork_runner,10U);\n        while(status==THREADWORK_OK) {{\n            status=threadwork_runner__has_recent_transit(&threadwork_runner,&recent_transit);\n            if(status!=THREADWORK_OK || !recent_transit) break;\n            status=tw_run_{symbol}(&context{root_args});\n        }}\n    }}",symbol=symbols[root]));
        for w in wires.values() {
            for side in ["a", "b"] {
                out.line(&format!("    while({a}_{side}_port.queue.head!=NULL) {{ threadwork_packet *p={a}_{side}_port.queue.head; {a}_{side}_port.queue.head=p->next; threadwork_packet_destroy(p); }}",a=w.allocation));
            }
        }
        out.line("    threadwork_context_destroy(&context);\n    (void)threadwork_runner__destroy(&threadwork_runner);\n    return status==THREADWORK_OK ? EXIT_SUCCESS : EXIT_FAILURE;\n}");
        let name = identifier(document.node(root)?.name.as_str());
        let mut files = vec![VirtualFile {
            path: format!("{name}.c"),
            content: out.source,
            binary_content: None,
            source_map: out.maps,
        }];
        files.extend(crate::generation::file_exports(document, root)?);
        let mut paths = BTreeSet::new();
        for file in &files {
            if !paths.insert(&file.path) {
                return Err(Error::new(
                    "invalid_input",
                    format!("Duplicate generated path '{}'", file.path),
                ));
            }
        }
        Ok(GeneratedProject {
            name: document.project_name().into(),
            capability: "c17-topology".into(),
            files,
        })
    }
}
fn arguments(
    id: &str,
    wires: &BTreeMap<String, Wire>,
    injected: &[(String, Argument)],
) -> Result<Vec<Argument>> {
    let mut args = vec![];
    let mut used = BTreeSet::new();
    for w in wires.values() {
        for (matches, side) in [(w.target == id, "b"), (w.source == id, "a")] {
            if matches {
                let name = if w.source == w.target {
                    format!("{}_{}", w.argument, if side == "b" { "in" } else { "out" })
                } else {
                    w.argument.clone()
                };
                if !used.insert(name.clone()) {
                    return Err(Error::new(
                        "invalid_input",
                        format!("Duplicate C argument '{name}'; use distinct link names"),
                    ));
                }
                args.push(Argument {
                    name,
                    ctype: "threadwork_buffer *".into(),
                    value: format!("&{}_{}_port", w.allocation, side),
                });
            }
        }
    }
    // Only consumers receive library instances and source capabilities; providers keep
    // their own signatures.
    for (_, argument) in injected.iter().filter(|(target, _)| target == id) {
        if !used.insert(argument.name.clone()) {
            return Err(Error::new(
                "invalid_input",
                format!(
                    "Duplicate C argument '{}'; use distinct link names",
                    argument.name
                ),
            ));
        }
        args.push(Argument {
            name: argument.name.clone(),
            ctype: argument.ctype.clone(),
            value: argument.value.clone(),
        });
    }
    Ok(args)
}
/// Libraries in dependency order: a library linked into another library is emitted first,
/// so its includes and declarations precede their use. Ties and cycles keep ID order.
fn library_order(libraries: &BTreeSet<String>, capabilities: &[&Node]) -> Vec<String> {
    let providers = |id: &str| {
        capabilities
            .iter()
            .filter_map(|n| n.link.as_ref())
            .filter(|l| l.target_node_id == id && libraries.contains(&l.source_node_id))
            .map(|l| l.source_node_id.clone())
            .collect::<BTreeSet<_>>()
    };
    let mut ordered = vec![];
    let mut visiting = BTreeSet::new();
    fn visit(
        id: &str,
        providers: &dyn Fn(&str) -> BTreeSet<String>,
        visiting: &mut BTreeSet<String>,
        ordered: &mut Vec<String>,
    ) {
        if ordered.iter().any(|o| o == id) || !visiting.insert(id.to_string()) {
            return;
        }
        for provider in providers(id) {
            visit(&provider, providers, visiting, ordered);
        }
        ordered.push(id.to_string());
    }
    for id in libraries {
        visit(id, &providers, &mut visiting, &mut ordered);
    }
    ordered
}
fn parameters(args: &[Argument]) -> String {
    format!(
        "threadwork_context *context{}",
        args.iter()
            .map(|argument| format!(", {}{}", argument.ctype, argument.name))
            .collect::<String>()
    )
}
/// Encodes authored text as adjacent C string literals, one per source line.
/// `?` is escaped so trigraph sequences cannot alter the materialized source.
fn string_literal(text: &str) -> String {
    if text.is_empty() {
        return "\"\"".into();
    }
    let mut pieces = vec![];
    for line in text.split_inclusive('\n') {
        let mut piece = String::from("\"");
        for c in line.chars() {
            match c {
                '\\' => piece.push_str("\\\\"),
                '"' => piece.push_str("\\\""),
                '?' => piece.push_str("\\?"),
                '\n' => piece.push_str("\\n"),
                '\r' => piece.push_str("\\r"),
                '\t' => piece.push_str("\\t"),
                c if c.is_ascii_control() => piece.push_str(&format!("\\{:03o}", c as u32)),
                c => piece.push(c),
            }
        }
        piece.push('"');
        pieces.push(piece);
    }
    pieces.join("\n        ")
}
fn emit_types(d: &Document, scope: &BTreeSet<String>, out: &mut Emitter) -> Result<()> {
    let types: BTreeSet<String> = scope
        .iter()
        .filter(|id| d.nodes[*id].kind == NodeKind::Type)
        .cloned()
        .collect();
    let mut emitted = BTreeSet::new();
    let mut pending = types.clone();
    for id in &types {
        let n = d.node(id)?;
        if !valid_identifier(&n.name) || reserved(&n.name) {
            return Err(unsupported(n, "Invalid C type identifier"));
        }
        out.line(&format!("struct {};", n.name));
    }
    while !pending.is_empty() {
        let before = pending.len();
        for id in pending.clone() {
            let n = d.node(&id)?;
            let definition = n.type_definition.as_ref().unwrap();
            if definition["qualifier"].as_str().unwrap_or("object") != "object" {
                return Err(unsupported(
                    n,
                    "Parameterized collection type emission is pending",
                ));
            }
            let fields = definition["fields"].as_array().cloned().unwrap_or_default();
            let blocked = fields.iter().any(|f| {
                !f["isReference"].as_bool().unwrap_or(false)
                    && f["typeId"]
                        .as_str()
                        .is_some_and(|id| types.contains(id) && !emitted.contains(id))
            });
            if blocked {
                continue;
            }
            out.line(&format!("typedef struct {} {{", n.name));
            if fields.is_empty() {
                out.line("    unsigned char _wirecraft_empty;");
            }
            for f in fields {
                let field = f["name"].as_str().unwrap_or("");
                if !valid_identifier(field) || reserved(field) {
                    return Err(unsupported(n, "Invalid C type field identifier"));
                }
                let type_id = f["typeId"].as_str().unwrap_or("string");
                let mut field_type = type_name(d, type_id)?;
                if let Some(node) = d.nodes.get(type_id)
                    && node.kind == NodeKind::Type
                {
                    field_type = format!("struct {}", node.name);
                }
                if f.get("typeExpression").is_some_and(|v| !v.is_null()) {
                    return Err(unsupported(
                        n,
                        "Explicit generic type expressions are pending",
                    ));
                }
                out.line(&format!(
                    "    {field_type} {}{field};",
                    if f["isReference"].as_bool().unwrap_or(false) {
                        "*"
                    } else {
                        ""
                    }
                ));
            }
            out.line(&format!("}} {};", n.name));
            emitted.insert(id.clone());
            pending.remove(&id);
        }
        if pending.len() == before {
            return Err(Error::new(
                "invalid_graph",
                "C value types contain a recursive value dependency; use reference fields",
            ));
        }
    }
    Ok(())
}
// Collections use the same erased runtime value as the source C runner.
fn expression_type(d: &Document, expression: &Value) -> Result<String> {
    match expression["constructorId"].as_str().unwrap_or("") {
        "" => type_name(d, expression["typeId"].as_str().unwrap_or("")),
        "array" => {
            let args = expression["arguments"]
                .as_array()
                .filter(|v| v.len() == 1)
                .ok_or_else(|| Error::new("invalid_input", "Array type requires one argument"))?;
            expression_type(d, &args[0])?;
            Ok("ThreadworkArray".into())
        }
        other => Err(Error::new(
            "unsupported",
            format!("C type constructor {other} is not supported"),
        )),
    }
}
fn type_name(d: &Document, id: &str) -> Result<String> {
    if let Some(n) = d.nodes.get(id) {
        if n.kind == NodeKind::Type && valid_identifier(&n.name) {
            return Ok(n.name.clone());
        }
        return Err(unsupported(n, "Wire refers to a non-C type entity"));
    }
    let value = match id {
        "boolean" => "bool",
        "number" => "double",
        "date" => "int64_t",
        "string" => "char *",
        "array" => "ThreadworkArray",
        s => s,
    };
    if valid_identifier(value)
        || [
            "signed char",
            "unsigned char",
            "unsigned short",
            "unsigned int",
            "unsigned long",
            "long long",
            "unsigned long long",
            "long double",
            "char *",
            "const char *",
            "void *",
        ]
        .contains(&value)
    {
        Ok(value.into())
    } else {
        Err(Error::new(
            "invalid_input",
            format!("Invalid C type '{id}'"),
        ))
    }
}
fn unsupported(n: &Node, message: &str) -> Error {
    Error::new("unsupported", format!("{} [{}]: {message}", n.name, n.id))
}
fn comment(s: &str) -> String {
    s.replace("*/", "* /").replace(['\n', '\r'], " ")
}
fn valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
pub(crate) fn identifier(value: &str) -> String {
    if valid_identifier(value.trim()) {
        return value.trim().into();
    }
    let mut result = String::new();
    for c in value.trim().chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            result.push(c);
        } else if !result.ends_with('_') {
            result.push('_');
        }
    }
    let mut result = result.trim_matches('_').to_string();
    if result.is_empty() {
        result = "value".into();
    }
    if result.starts_with(|c: char| c.is_ascii_digit()) {
        result.insert(0, '_');
    }
    result
}
fn reserved(s: &str) -> bool {
    [
        "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else",
        "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long", "register",
        "restrict", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef",
        "union", "unsigned", "void", "volatile", "while", "_Atomic", "_Bool",
    ]
    .contains(&s)
}

pub(crate) struct LibraryFunction {
    pub name: String,
    pub declaration: String,
    pub signature: String,
}
/// A function a C service library exports through its instance struct.
pub(crate) struct LibraryMember {
    pub name: String,
    pub result: String,
    pub parameters: String,
}
impl LibraryMember {
    pub fn signature(&self) -> String {
        format!("{} {}({})", self.result, self.name, self.parameters)
    }
}
/// `lib_math` → `Lib_mathLibrary`, matching the capability type spelling.
pub(crate) fn library_type(library: &Node) -> String {
    let mut name = identifier(&library.name);
    name[..1].make_ascii_uppercase();
    format!("{name}Library")
}
/// The single shared instance of a library's exporter struct.
pub(crate) fn library_instance(library: &Node) -> String {
    format!("{}_library", identifier(&library.name))
}
/// Exporter struct and its one static instance, emitted after the library source.
pub(crate) fn library_exporter(library: &Node) -> String {
    let members = library_members(library);
    let ty = library_type(library);
    let mut text = format!(
        "/* Library {} instance: dependency links lend it to consumers by link name. */\ntypedef struct {ty} {{",
        comment(&library.name)
    );
    if members.is_empty() {
        text.push_str("\n    unsigned char _wirecraft_empty;");
    }
    for m in &members {
        text.push_str(&format!(
            "\n    {} (*{})({});",
            m.result, m.name, m.parameters
        ));
    }
    text.push_str(&format!(
        "\n}} {ty};\nstatic const {ty} {} = {{",
        library_instance(library)
    ));
    if members.is_empty() {
        text.push_str(" 0 ");
    }
    for m in &members {
        text.push_str(&format!("\n    .{name} = {name},", name = m.name));
    }
    if !members.is_empty() {
        text.push('\n');
    }
    text.push_str("};");
    text
}
/// Top-level function declarations and definitions in a library, plus the supported
/// standard-header functions it includes.
pub(crate) fn library_members(library: &Node) -> Vec<LibraryMember> {
    let source = &library.text.source;
    // Brace depth at every byte, so only file-scope declarations are exported.
    let mut depth = vec![0usize; source.len() + 1];
    let mut level = 0usize;
    for (i, b) in source.bytes().enumerate() {
        depth[i] = level;
        match b {
            b'{' => level += 1,
            b'}' => level = level.saturating_sub(1),
            _ => {}
        }
    }
    let mut members: Vec<LibraryMember> = vec![];
    let mut add = |result: &str, name: &str, parameters: &str| {
        if !members.iter().any(|m| m.name == name) {
            members.push(LibraryMember {
                name: name.into(),
                result: result.into(),
                parameters: parameters.into(),
            });
        }
    };
    let function_pattern=regex::Regex::new(r"(?m)(?:^|[;}])\s*((?:[A-Za-z_][A-Za-z0-9_]*\s+)+(?:\*+\s*)?)([A-Za-z_][A-Za-z0-9_]*)\s*\(([^;{}]*)\)\s*(?:\{|;)").unwrap();
    for capture in function_pattern.captures_iter(source) {
        let whole = capture.get(0).unwrap();
        if depth[capture.get(2).unwrap().start()] != 0
            || whole.as_str().trim_start().starts_with('#')
        {
            continue;
        }
        let words = capture[1].split_whitespace().collect::<Vec<_>>();
        if words
            .iter()
            .any(|w| ["return", "else", "typedef", "case", "goto", "sizeof", "do"].contains(w))
            || ["if", "while", "for", "switch"].contains(&&capture[2])
        {
            continue;
        }
        let result = words
            .into_iter()
            .filter(|w| !["static", "extern", "inline", "_Noreturn"].contains(w))
            .collect::<Vec<_>>()
            .join(" ");
        let parameters = capture[3].trim();
        add(
            &result,
            &capture[2],
            if parameters.is_empty() {
                "void"
            } else {
                parameters
            },
        );
    }
    for (header, functions) in [
        (
            "stdio.h",
            &[
                ("FILE *", "fopen", "const char *, const char *"),
                ("int", "fprintf", "FILE *, const char *, ..."),
                ("int", "fclose", "FILE *"),
            ][..],
        ),
        (
            "time.h",
            &[
                ("time_t", "time", "time_t *"),
                ("struct tm *", "localtime_r", "const time_t *, struct tm *"),
                (
                    "size_t",
                    "strftime",
                    "char *, size_t, const char *, const struct tm *",
                ),
            ][..],
        ),
        (
            "uuid/uuid.h",
            &[
                ("void", "uuid_generate_random", "uuid_t"),
                ("void", "uuid_unparse_lower", "const uuid_t, char *"),
            ][..],
        ),
    ] {
        // Match the directive itself: `<sys/time.h>` must not imply `<time.h>`.
        let directive = regex::Regex::new(&format!(
            r#"(?m)^\s*#\s*include\s*[<"]{}[>"]"#,
            regex::escape(header)
        ))
        .unwrap();
        if directive.is_match(source) {
            for (result, name, parameters) in functions {
                add(result, name, parameters);
            }
        }
    }
    members
}
/// Legacy `link__function` aliases, emitted only for bodies that still use them.
pub(crate) fn library_functions(document: &Document, n: &Node) -> Result<Vec<LibraryFunction>> {
    let link = n
        .link
        .as_ref()
        .ok_or_else(|| unsupported(n, "Library capability must be a link"))?;
    let library = document.node(&link.source_node_id)?;
    let prefix = identifier(&n.name);
    Ok(library_members(library)
        .into_iter()
        .map(|m| {
            let alias = format!("{prefix}__{}", m.name);
            LibraryFunction {
                declaration: format!("{} (*{alias})({}) = {};", m.result, m.parameters, m.name),
                signature: format!("{} {alias}({})", m.result, m.parameters),
                name: alias,
            }
        })
        .collect())
}
