//! Clipboard assistance is the implemented provider in the Threadworks baseline.
use crate::{Document, Error, Result};
use serde_json::{Value, json};
pub fn prompt(d: &Document, ids: &[String], task: &str) -> Result<Value> {
    if ids.is_empty() {
        return Err(Error::new("invalid_input", "Select at least one component"));
    }
    for id in ids {
        d.node(id)?;
    }
    let (instruction, section) = match task {
        "source" => (
            "Generate the complete source for the selected editor target. Honor its specification, typed inputs/outputs, dependencies, usage instructions and effective language/technology. For ordinary processing nodes return only statements for the recurring body; the compiler owns signatures, parameters, initialization, transport and main. For C use pop(input_buffer, &item), push(output_buffer, &item) and threadwork_buffer_count(buffer); preserve compiler-provided argument names. Return THREADWORK_ERROR only on failure. For C service libraries return translation-unit declarations and implementations, including includes where needed. Return only source, without commentary or Markdown fences.",
            "source",
        ),
        "specification" => (
            "Rewrite only the authored specification as clear precise Markdown. Preserve stated behavior, constraints, edge cases, inputs, outputs, dependencies and acceptance criteria. Do not invent requirements or change source. Return only the revised specification.",
            "specification",
        ),
        "tests" => (
            "Generate representative test data or a test script in each component's effective Tests language. Derive cases from its specification, exact input/output types and dependencies. Include normal, boundary, malformed and dependency-failure cases. Use exact compiler buffer and service names where executable. Return only test data or the test script.",
            "tests",
        ),
        _ => {
            return Err(Error::new(
                "invalid_input",
                "AI task must be source, specification or tests",
            ));
        }
    };
    let dossier = crate::export::markdown(d, ids)?;
    let mut longest = 0;
    let mut run = 0;
    for c in dossier.chars() {
        if c == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    let fence = "`".repeat((longest + 1).max(3));
    let mut text = format!(
        "You are assisting with a Wirecraft / Threadworks component.\n\nTask: {instruction}\n\nComponent dossier:\n{fence}markdown\n{dossier}\n{fence}\n"
    );
    if let Ok(project) = crate::c17::C17::generate(d, &d.root_node_id) {
        text.push_str("\nCompiler-provided editable context (generated wrapper excerpts):\n");
        for file in &project.files {
            let lines = file.content.lines().collect::<Vec<_>>();
            for map in file
                .source_map
                .iter()
                .filter(|m| ids.contains(&m.node_id) && m.section == section)
            {
                let start = map.generated_start_line.saturating_sub(6);
                let end = (map.generated_end_line + 2).min(lines.len());
                if start < end {
                    text.push_str(&format!("\n{} / {}:\n", map.node_id, map.section));
                    text.push_str(&lines[start..end].join("\n"));
                    text.push('\n');
                }
            }
        }
    }
    Ok(json!({"provider":"clipboard","task":task,"section":section,"prompt":text,"nodeIds":ids}))
}
