//! Deterministic portable reports and vector diagrams. Rendering belongs to adapters.
use crate::{
    Document, NodeKind, Result,
    viewport::{Rect, route},
};
use std::{collections::BTreeMap, fmt::Write};
pub fn markdown(d: &Document, ids: &[String]) -> Result<String> {
    let mut out = format!("# {}\n\n", d.project_name());
    for id in ids {
        let n = d.node(id)?;
        writeln!(out,"## {}\n\n- ID: `{}`\n- Kind: {:?}\n- Role: {}\n- Path: {}\n- Language: {}\n- Technology: {}\n- Status: {}\n- Assignee: {}\n- Responsible: {}\n",n.name,n.id,n.kind,crate::analysis::stereotype(d,n),d.qualified_name(id),d.effective(id,"languageId"),d.effective(id,"technologyId"),n.status.as_deref().unwrap_or(""),n.assignee.as_deref().unwrap_or(""),d.effective(id,"responsible")).unwrap();
        for (title, text, language) in [
            ("Specification", &n.text.specification, "markdown"),
            (
                "Source",
                &n.text.source,
                d.effective(id, "languageId").as_str(),
            ),
            (
                "Initialization",
                &n.text.initialization,
                d.effective(id, "languageId").as_str(),
            ),
            ("Tests", &n.text.tests, n.text.tests_language_id.as_str()),
            ("Usage", &n.text.ai_instructions, "markdown"),
        ] {
            if !text.is_empty() {
                let fence = "`".repeat(
                    text.lines()
                        .map(|line| line.chars().take_while(|c| *c == '`').count())
                        .max()
                        .unwrap_or(0)
                        .max(2)
                        + 1,
                );
                writeln!(out, "### {title}\n\n{fence}{language}\n{text}\n{fence}\n").unwrap();
            }
        }
        if let Some(link) = &n.link {
            writeln!(
                out,
                "### Link contract\n\n```json\n{}\n```\n",
                serde_json::to_string_pretty(link)?
            )
            .unwrap();
        }
        for lid in n.incoming_links.iter().chain(n.outgoing_links.iter()) {
            if let Some(l) = d.nodes.get(lid).and_then(|n| n.link.as_ref()) {
                writeln!(
                    out,
                    "- Link `{lid}`: {}:{} → {}:{} ({}, {})",
                    l.source_node_id,
                    l.source_port_name,
                    l.target_node_id,
                    l.target_port_name,
                    l.interaction_kind,
                    l.type_name
                )
                .unwrap();
            }
        }
    }
    Ok(out)
}
fn escape(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\'' => "&apos;".into(),
            c => c.to_string(),
        })
        .collect()
}
/// Export the focused graph to a scalable drawing with title and parts list.
/// This is a free-size drawing, not yet the original ISO/poster sheet compositor.
pub fn svg(d: &Document, parent: &str) -> Result<String> {
    let p = d.node(parent)?;
    let mut shapes = BTreeMap::new();
    for id in &p.children {
        let n = d.node(id)?;
        if !n.is_link() {
            let r = &n.layout;
            shapes.insert(
                id.clone(),
                Rect {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: r.height,
                },
            );
        }
    }
    let left = shapes.values().map(|r| r.x).fold(0., f64::min) - 40.;
    let top = shapes.values().map(|r| r.y).fold(0., f64::min) - 70.;
    let right = shapes.values().map(|r| r.x + r.width).fold(400., f64::max) + 40.;
    let bottom = shapes.values().map(|r| r.y + r.height).fold(200., f64::max) + 40.;
    let width = (right - left).max(600.);
    let table_y = bottom + 20.;
    let height = table_y - top + (shapes.len() as f64 + 2.) * 24. + 50.;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"{left} {top} {width} {height}\"><title>{}</title><desc>Wirecraft focused diagram: {}</desc><defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"#243b53\"/></marker></defs><rect x=\"{left}\" y=\"{top}\" width=\"{width}\" height=\"{height}\" fill=\"white\"/><g font-family=\"DejaVu Sans\" fill=\"#102a43\"><text x=\"{}\" y=\"{}\" font-size=\"22\">{}</text>",
        escape(d.project_name()),
        escape(&d.qualified_name(parent)),
        left + 20.,
        top + 32.,
        escape(&d.qualified_name(parent))
    );
    for id in &p.children {
        let n = d.node(id)?;
        if let Some(link) = &n.link
            && let (Some(a), Some(b)) = (
                shapes.get(&link.source_node_id),
                shapes.get(&link.target_node_id),
            )
        {
            let points = route(*a, *b);
            let points_text = points
                .iter()
                .map(|p| format!("{},{}", p[0], p[1]))
                .collect::<Vec<_>>()
                .join(" ");
            write!(out,"<polyline points=\"{points_text}\" fill=\"none\" stroke=\"#243b53\" stroke-width=\"1.5\" marker-end=\"url(#arrow)\"/><text x=\"{}\" y=\"{}\" font-size=\"11\">{}</text>",points[1][0]+4.,points[1][1]-8.,escape(&n.name)).unwrap();
        }
    }
    for (id, r) in &shapes {
        let n = d.node(id)?;
        write!(out,"<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"{}\" stroke=\"#334e68\"/><text x=\"{}\" y=\"{}\" font-size=\"10\">{:?}</text><text x=\"{}\" y=\"{}\" font-size=\"14\">{}</text>",r.x,r.y,r.width,r.height,if n.kind==NodeKind::Type{"#e0f2f1"}else{"#f0f4f8"},r.x+8.,r.y+16.,n.kind,r.x+8.,r.y+38.,escape(&n.name)).unwrap();
    }
    write!(out,"<path d=\"M {} {table_y} H {}\" stroke=\"#334e68\"/><text x=\"{}\" y=\"{}\" font-size=\"12\">Parts · Name / Kind / Responsible / Revision</text>",left+20.,left+width-20.,left+20.,table_y+20.).unwrap();
    for (i, id) in shapes.keys().enumerate() {
        let n = d.node(id)?;
        let revision = n
            .revision
            .as_ref()
            .and_then(|v| v["name"].as_str())
            .unwrap_or("");
        write!(
            out,
            "<text x=\"{}\" y=\"{}\" font-size=\"11\">{} · {:?} · {} · {}</text>",
            left + 20.,
            table_y + 44. + i as f64 * 24.,
            escape(&n.name),
            n.kind,
            escape(&d.effective(id, "responsible")),
            escape(revision)
        )
        .unwrap();
    }
    out.push_str("</g></svg>\n");
    Ok(out)
}
