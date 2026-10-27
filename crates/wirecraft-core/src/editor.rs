//! Compiler-backed body context. No editor framework or language-server dependency.
use crate::{
    Document, Error, Result,
    analysis::{Relation, relation},
    c17::C17,
    generation::{GeneratedProject, SourceMap, VirtualFile},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
pub struct BodyContext {
    pub file: VirtualFile,
    pub companions: Vec<VirtualFile>,
    pub source: SourceMap,
    pub header: String,
    pub symbols: Vec<Value>,
    pub type_signatures: BTreeMap<String, String>,
    pub excluded_library_prefixes: BTreeSet<String>,
}
impl BodyContext {
    pub fn project(&self, name: &str) -> GeneratedProject {
        GeneratedProject {
            name: name.into(),
            files: std::iter::once(self.file.clone())
                .chain(self.companions.iter().cloned())
                .collect(),
            capability: "c17-editor-context".into(),
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
    if !["source", "initialization"].contains(&section)
        || document.effective_text_language(node_id, section) != "c"
        || document.effective(node_id, "technologyId") != "c-native"
    {
        return Err(Error::new(
            "unsupported",
            "Compiler body context currently supports C17 Declaration and Instantiation sections",
        ));
    }
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
                symbols.push(json!({"label":name,"kind":6,"detail":format!("const {} *{name}",crate::c17::library_type(library)),"documentation":format!("Instance of library {} injected by dependency {}; call members with {name}->function(...).\n\n{}",library.name,n.name,members.join("\n"))}));
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
                symbols.push(json!({"label":name,"kind":6,"detail":format!("{ctype} *{name}; char *getSource(capability, parameters, parameter_count)"),"documentation":format!("Materializes the declaration of {provider}, replacing ${{name}} markers with threadwork_build_parameter values; the caller frees the result.")}));
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
                symbols.push(json!({"label":argument,"kind":6,"detail":format!("{direction} buffer of {payload}"),"documentation":format!("{}:{} → {}:{}",link.source_node_id,link.source_port_name,link.target_node_id,link.target_port_name)}));
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
    Ok(BodyContext {
        type_signatures,
        file,
        companions: files.collect(),
        source,
        header,
        symbols,
        excluded_library_prefixes,
    })
}
