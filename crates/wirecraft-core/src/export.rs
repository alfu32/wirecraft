//! Deterministic portable reports and vector diagrams. Rendering belongs to adapters.
use crate::{Document, Result};
use std::fmt::Write;
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
pub(crate) fn escape(s: &str) -> String {
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
/// Portable visible diagram. Composite visibility matches the desktop projection.
pub fn svg(d: &Document, parent: &str) -> Result<String> {
    svg_impl(d, parent, true, false)
}
pub fn drawing_svg(d: &Document, parent: &str) -> Result<String> {
    svg_impl(d, parent, false, false)
}
/// Tight, transparent graph for a technical plan, independent of its world origin.
pub fn plan_svg(d: &Document, parent: &str) -> Result<String> {
    svg_impl(d, parent, false, true)
}
fn svg_impl(d: &Document, parent: &str, include_parts: bool, technical: bool) -> Result<String> {
    d.node(parent)?;
    let drawing = crate::drawing::Drawing::new(d, parent);
    let mut left = drawing.shapes.values().map(|s| s.rect.x).fold(0., f64::min) - 40.;
    let mut top = drawing.shapes.values().map(|s| s.rect.y).fold(0., f64::min) - 70.;
    let right = drawing
        .shapes
        .values()
        .map(|s| s.rect)
        .chain(drawing.annotations.values().map(|a| a.rect))
        .map(|r| r.x + r.width)
        .fold(400., f64::max)
        + 40.;
    let bottom = drawing
        .shapes
        .values()
        .map(|s| s.rect)
        .chain(drawing.annotations.values().map(|a| a.rect))
        .map(|r| r.y + r.height)
        .fold(200., f64::max)
        + 40.;
    let mut width = (right - left).max(600.);
    let table_y = bottom + 20.;
    let mut height = if include_parts {
        table_y - top + (drawing.shapes.len() as f64 + 2.) * 24. + 70.
    } else {
        bottom - top + 20.
    };
    if technical {
        use crate::viewport::Rect;
        let bounds = drawing
            .shapes
            .values()
            .map(|s| s.rect)
            .chain(drawing.annotations.values().map(|a| a.rect))
            .chain(
                drawing
                    .edges
                    .values()
                    .flat_map(|e| e.points.iter())
                    .map(|p| Rect {
                        x: p[0],
                        y: p[1],
                        width: 0.,
                        height: 0.,
                    }),
            )
            .reduce(Rect::union)
            .unwrap_or(Rect {
                x: 0.,
                y: 0.,
                width: 400.,
                height: 200.,
            });
        left = bounds.x - 20.;
        top = bounds.y - 25.;
        width = bounds.width + 40.;
        height = bounds.height + 50.;
    }
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"{left} {top} {width} {height}\"><title>{}</title><defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\"><path d=\"M0 0L10 5L0 10z\" fill=\"#334e68\"/></marker></defs><rect x=\"{left}\" y=\"{top}\" width=\"{width}\" height=\"{height}\" fill=\"white\"/><g font-family=\"DejaVu Sans\" fill=\"#102a43\"><text x=\"{}\" y=\"{}\" font-size=\"22\">{}</text>",
        escape(d.project_name()),
        left + 20.,
        top + 32.,
        escape(&d.qualified_name(parent))
    );
    if technical {
        out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"{left} {top} {width} {height}\"><title>{}</title><defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\"><path d=\"M0 0L10 5L0 10z\" fill=\"#334e68\"/></marker></defs><g font-family=\"DejaVu Sans\" fill=\"#102a43\">",
            escape(d.project_name())
        );
    }
    let mut shapes = drawing.shapes.iter().collect::<Vec<_>>();
    shapes.sort_by_key(|(_, s)| s.depth);
    for (id, s) in &shapes {
        let r = s.rect;
        let n = d.node(id)?;
        let (fill, stroke) = match s.role.as_str() {
            "Type" => ("#e0f2f1", "#00897b"),
            "ServiceLibrary" => ("#f1f2ff", "#626bd5"),
            "CompilerTemplate" => ("#fff4dc", "#aa6a00"),
            "ErrorHandler" => ("#fff2f2", "#bd5555"),
            _ => ("#f4f7fa", "#334e68"),
        };
        write!(out,"<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"0\" fill=\"{fill}\" stroke=\"{stroke}\" {}/><text x=\"{}\" y=\"{}\" font-size=\"14\">{}</text><text x=\"{}\" y=\"{}\" font-size=\"10\">{}</text>",r.x,r.y,r.width,r.height,if s.composite&&s.expanded{"stroke-dasharray=\"8 4\""}else{""},r.x+8.,r.y+20.,escape(&format!("{} {}",n.name,n.name_detail)),r.x+8.,r.y+37.,escape(&s.role)).unwrap();
        if s.diagnostics > 0 {
            write!(out,"<circle cx=\"{}\" cy=\"{}\" r=\"7\" fill=\"#d94f54\"/><text x=\"{}\" y=\"{}\" fill=\"white\" text-anchor=\"middle\" font-size=\"10\">!</text>",r.x+r.width-11.,r.y+11.,r.x+r.width-11.,r.y+14.).unwrap();
        }
        for (i, field) in s.fields.iter().enumerate() {
            if 52. + i as f64 * 15. > r.height {
                break;
            }
            write!(
                out,
                "<text x=\"{}\" y=\"{}\" fill=\"#00897b\" font-size=\"10\">{}</text>",
                r.x + 8.,
                r.y + 52. + i as f64 * 15.,
                escape(field)
            )
            .unwrap();
        }
    }
    for edge in drawing.edges.values().filter(|e| !e.annotation_only) {
        if let Some(error) = &edge.route_error {
            let r = drawing.shapes[&edge.source].rect;
            write!(out,"<text class=\"routing-error\" x=\"{}\" y=\"{}\" font-size=\"10\" fill=\"#bd5555\">{}</text>",r.x,r.y-8.,escape(error)).unwrap();
        }

        let points = edge
            .points
            .iter()
            .map(|p| format!("{},{}", p[0], p[1]))
            .collect::<Vec<_>>()
            .join(" ");
        let color = match edge.relation {
            Some(crate::analysis::Relation::Error) => "#bd5555",
            Some(crate::analysis::Relation::Dependency) => "#b88435",
            Some(crate::analysis::Relation::TypeUsage) => "#00897b",
            _ => "#334e68",
        };
        write!(out,"<polyline points=\"{points}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.5\" {}/>",if edge.dashed{"stroke-dasharray=\"8 5\""}else{""}).unwrap();
        for p in &edge.crossings {
            if let Some(s) = edge
                .points
                .windows(2)
                .find(|s| crate::viewport::distance_to_segment(*p, s[0], s[1]) < 0.1)
            {
                let dx = s[1][0] - s[0][0];
                let dy = s[1][1] - s[0][1];
                let len = dx.hypot(dy);
                let (ux, uy) = (dx / len, dy / len);
                let a = [p[0] - ux * 6., p[1] - uy * 6.];
                let b = [p[0] + ux * 6., p[1] + uy * 6.];
                write!(out,"<path d=\"M{} {}L{} {}\" stroke=\"white\" stroke-width=\"6\"/><path class=\"channel-bridge\" d=\"M{} {}C{} {},{} {},{} {}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.5\"/>",a[0],a[1],b[0],b[1],a[0],a[1],a[0]-uy*9.,a[1]+ux*9.,b[0]-uy*9.,b[1]+ux*9.,b[0],b[1]).unwrap();
            }
        }
        let (output, input) = crate::drawing::port_glyphs(&edge.points);
        for (class, points, filled) in [("output-port", output, false), ("input-port", input, true)]
        {
            if points.is_empty() {
                continue;
            }
            let points = points
                .iter()
                .map(|p| format!("{},{}", p[0], p[1]))
                .collect::<Vec<_>>()
                .join(" ");
            write!(
                out,
                "<polygon class=\"{class}\" points=\"{points}\" stroke=\"{color}\" fill=\"{}\"/>",
                if filled { color } else { "white" }
            )
            .unwrap();
        }
        if edge.points.len() >= 2 {
            for (i, j, text) in [
                (0, 1, &edge.source_label),
                (
                    edge.points.len() - 1,
                    edge.points.len() - 2,
                    &edge.target_label,
                ),
            ] {
                let p = edge.points[i];
                let right = edge.points[j][0] >= p[0];
                let split = text.rfind(':').unwrap_or(text.len());
                let name_len = if split < text.len() { split + 1 } else { split };
                write!(out,"<text x=\"{}\" y=\"{}\" font-size=\"9\" text-anchor=\"{}\"><tspan fill=\"#334e68\">{}</tspan><tspan fill=\"#00897b\">{}</tspan></text>",p[0]+if right{16.}else{-16.},p[1]-6.,if right{"start"}else{"end"},escape(&text[..name_len]),escape(&text[name_len..])).unwrap();
            }
        }
    }
    for port in &drawing.boundary_ports {
        let owner = drawing.shapes[&port.owner].rect;
        let right = port.point[0] > owner.x + owner.width / 2.;
        write!(out,"<circle class=\"composite-port\" cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"#00897b\"/><text x=\"{}\" y=\"{}\" text-anchor=\"{}\" font-size=\"9\">{}</text>",port.point[0],port.point[1],port.point[0]+if right{9.}else{-9.},port.point[1]-6.,if right{"start"}else{"end"},escape(&port.label)).unwrap();
    }

    for annotation in drawing.annotations.values() {
        let r = annotation.rect;
        let owner = drawing.shapes[&annotation.owner].rect;
        let color = if annotation.color_key == "typeStroke" {
            "#00897b"
        } else {
            "#b88435"
        };
        if annotation.flag {
            let y = r.y + 18.;
            write!(out,"<path d=\"M{} {}V{y}h{}\" fill=\"none\" stroke=\"{color}\"/><circle cx=\"{}\" cy=\"{y}\" r=\"3\" fill=\"{color}\"/><text x=\"{}\" y=\"{}\" font-size=\"10\" fill=\"{color}\">{}</text>",r.x,owner.y,r.width,r.x,r.x+6.,y-4.,escape(&annotation.label)).unwrap();
        } else {
            write!(out,"<path d=\"M{} {}H{}\" stroke=\"{color}\"/><circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"{color}\"/><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\" stroke=\"{color}\"/><text x=\"{}\" y=\"{}\" font-size=\"10\" fill=\"{color}\">{}</text>",owner.x+owner.width,r.y+r.height/2.,r.x,owner.x+owner.width,r.y+r.height/2.,r.x,r.y,r.width,r.height,r.x+5.,r.y+16.,escape(&annotation.label)).unwrap();
        }
    }
    if include_parts {
        write!(out,"<path d=\"M{} {table_y}H{}\" stroke=\"#334e68\"/><text x=\"{}\" y=\"{}\" font-size=\"12\">Parts · Name / Role / Responsible / Revision</text>",left+20.,right-20.,left+20.,table_y+20.).unwrap();
        for (i, (id, s)) in shapes.iter().enumerate() {
            let n = d.node(id)?;
            write!(
                out,
                "<text x=\"{}\" y=\"{}\" font-size=\"11\">{} · {} · {} · {}</text>",
                left + 20.,
                table_y + 44. + i as f64 * 24.,
                escape(&n.name),
                escape(&s.role),
                escape(&d.effective(id, "responsible")),
                escape(
                    n.revision
                        .as_ref()
                        .and_then(|r| r["name"].as_str())
                        .unwrap_or("")
                )
            )
            .unwrap();
        }
    }
    out.push_str("</g></svg>\n");
    Ok(out)
}
pub fn mermaid(d: &Document, parent: &str) -> Result<String> {
    d.node(parent)?;
    let drawing = crate::drawing::Drawing::new(d, parent);
    let mut out = String::from("flowchart LR\n");
    let keys = drawing
        .shapes
        .keys()
        .enumerate()
        .map(|(i, id)| (id.clone(), format!("n{i}")))
        .collect::<std::collections::BTreeMap<_, _>>();
    let clean = |s: &str| {
        s.replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace(['\n', '\r'], " ")
    };
    for (id, s) in &drawing.shapes {
        writeln!(out, "  {}[\"{} ({})\"]", keys[id], clean(&s.name), s.role).unwrap();
    }
    // Resolve collapsed endpoints through the same projection by matching edge endpoints.
    for (id, edge) in &drawing.edges {
        let l = d.node(id)?.link.as_ref().unwrap();
        let projected = |id: &str| {
            let mut node = d.nodes.get(id);
            while let Some(n) = node {
                if keys.contains_key(&n.id) {
                    return Some(&keys[&n.id]);
                }
                node = n.parent_id.as_ref().and_then(|id| d.nodes.get(id));
            }
            None
        };
        if let (Some(a), Some(b)) = (projected(&l.source_node_id), projected(&l.target_node_id)) {
            writeln!(out, "  {a} -->|\"{}\"| {b}", clean(&edge.name)).unwrap();
        }
    }
    Ok(out)
}
