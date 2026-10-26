use crate::{
    Document, Error, Result,
    export::{escape, markdown},
    sheets::dimensions,
};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Options {
    pub format: String,
    pub header: bool,
    pub footer: bool,
    pub top_mm: f64,
    pub left_mm: f64,
    pub right_mm: f64,
    pub bottom_mm: f64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            format: "A4".into(),
            header: true,
            footer: true,
            top_mm: 15.,
            left_mm: 15.,
            right_mm: 15.,
            bottom_mm: 15.,
        }
    }
}
pub fn ids(d: &Document, parent: &str) -> Result<Vec<String>> {
    d.node(parent)?;
    let mut stack = vec![parent.to_string()];
    let mut ids = vec![];
    while let Some(id) = stack.pop() {
        let n = d.node(&id)?;
        ids.push(id);
        stack.extend(n.children.iter().rev().cloned());
    }
    Ok(ids)
}
pub fn markdown_document(d: &Document, parent: &str) -> Result<String> {
    let ids = ids(d, parent)?;
    let mut out = format!(
        "# {} Functional and Technical Specification\n\n## Contents\n\n",
        d.project_name()
    );
    for id in &ids {
        writeln!(out, "- {}", d.qualified_name(id)).unwrap();
    }
    out.push_str("\n## Composition\n\n```mermaid\n");
    out.push_str(&crate::export::mermaid(d, parent)?);
    out.push_str("```\n\n");
    out.push_str(&markdown(d, &ids)?);
    Ok(out)
}
pub fn html(d: &Document, parent: &str) -> Result<String> {
    let ids = ids(d, parent)?;
    let mut out = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{} Specification</title><style>body{{font:15px system-ui;max-width:1000px;margin:3rem auto;color:#243b53}}pre{{white-space:pre-wrap;background:#f0f4f8;padding:1rem}}article{{break-inside:avoid}}table{{border-collapse:collapse}}td,th{{border:1px solid #bcccdc;padding:.4rem}}</style></head><body><h1>{} Functional and Technical Specification</h1><nav><h2>Contents</h2><ul>",
        escape(d.project_name()),
        escape(d.project_name())
    );
    for (i, id) in ids.iter().enumerate() {
        write!(
            out,
            "<li><a href=\"#part-{i}\">{}</a></li>",
            escape(&d.qualified_name(id))
        )
        .unwrap();
    }
    out.push_str("</ul></nav><section><h2>Composition</h2>");
    out.push_str(&crate::export::drawing_svg(d, parent)?.replacen(
        "<svg ",
        "<svg style=\"max-width:100%;height:auto\" ",
        1,
    ));
    out.push_str("</section>");
    for (i, id) in ids.iter().enumerate() {
        let n = d.node(id)?;
        write!(out,"<article id=\"part-{i}\"><h2>{}</h2><p>{} · {} · {}</p><table><tr><th>Status</th><th>Responsible</th><th>Assignee</th></tr><tr><td>{}</td><td>{}</td><td>{}</td></tr></table>",escape(&n.name),escape(crate::analysis::stereotype(d,n)),escape(&d.effective(id,"languageId")),escape(&d.effective(id,"technologyId")),escape(n.status.as_deref().unwrap_or("")),escape(&d.effective(id,"responsible")),escape(n.assignee.as_deref().unwrap_or(""))).unwrap();
        for (label, text) in [
            ("Specification", &n.text.specification),
            ("Declaration", &n.text.source),
            ("Instantiation", &n.text.initialization),
            ("Tests", &n.text.tests),
            ("Usage Instructions", &n.text.ai_instructions),
        ] {
            if !text.is_empty() {
                write!(out, "<h3>{label}</h3><pre>{}</pre>", escape(text)).unwrap();
            }
        }
        out.push_str("</article>");
    }
    out.push_str("</body></html>\n");
    Ok(out)
}
pub fn pages(d: &Document, parent: &str, options: &Options) -> Result<Vec<String>> {
    if options.format.ends_with("roll") || options.format == "Auto" {
        return Err(Error::new(
            "invalid_input",
            "Documentation needs a fixed page format",
        ));
    }
    let (w, h) = dimensions(&options.format)?;
    let margins = [
        options.top_mm,
        options.left_mm,
        options.right_mm,
        options.bottom_mm,
    ];
    if margins
        .iter()
        .any(|v| !v.is_finite() || !(0.0..=100.0).contains(v))
        || w - options.left_mm - options.right_mm < 40.
        || h - options.top_mm - options.bottom_mm < 40.
    {
        return Err(Error::new("invalid_input", "Invalid documentation margins"));
    }
    let ids = ids(d, parent)?;
    let columns = ((w - options.left_mm - options.right_mm) / 1.65).floor() as usize;
    let lines_per_page = ((h - options.top_mm - options.bottom_mm - 15.) / 4.5).floor() as usize;
    let wrap = |text: &str| {
        let mut lines = Vec::new();
        for line in text.lines() {
            let chars = line.chars().collect::<Vec<_>>();
            if chars.is_empty() {
                lines.push(String::new());
            } else {
                lines.extend(chars.chunks(columns).map(|c| c.iter().collect::<String>()));
            }
        }
        lines
    };
    let mut cover = format!(
        "# {} Functional and Technical Specification\n\n## Contents\n\n",
        d.project_name()
    );
    for id in &ids {
        writeln!(cover, "- {}", d.qualified_name(id)).unwrap();
    }
    let contents = wrap(&cover);
    let dossier = markdown(d, &ids)?;
    let dossier = dossier
        .split_once("\n\n")
        .map(|(_, body)| body)
        .unwrap_or(&dossier);
    let body = wrap(dossier);
    let total = contents.len().div_ceil(lines_per_page) + 1 + body.len().div_ceil(lines_per_page);
    if total > 256 {
        return Err(Error::new("export", "Documentation exceeds 256 pages"));
    }
    let start = || {
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\"><rect width=\"{w}\" height=\"{h}\" fill=\"white\"/><g font-family=\"DejaVu Sans Mono\" font-size=\"2.7\" fill=\"#243b53\">"
        );
        if options.header {
            write!(out,"<text x=\"{}\" y=\"{}\" font-family=\"DejaVu Sans\" font-size=\"3\">{} · Revision {} {}</text>",options.left_mm,options.top_mm,escape(d.project_name()),escape(d.master_revision["name"].as_str().unwrap_or("")),escape(d.master_revision["date"].as_str().unwrap_or(""))).unwrap();
        }
        out
    };
    let finish = |mut out: String, index: usize| {
        if options.footer {
            write!(
                out,
                "<text x=\"{}\" y=\"{}\">Wirecraft · {} / {total}</text>",
                options.left_mm,
                h - options.bottom_mm,
                index + 1
            )
            .unwrap();
        }
        out.push_str("</g></svg>");
        out
    };
    let text_page = |lines: &[String]| {
        let mut out = start();
        for (i, line) in lines.iter().enumerate() {
            let (text, size, weight, family) = if let Some(s) = line.strip_prefix("# ") {
                (s, 4.2, "600", "DejaVu Sans")
            } else if let Some(s) = line.strip_prefix("## ") {
                (s, 3.8, "600", "DejaVu Sans")
            } else if let Some(s) = line.strip_prefix("### ") {
                (s, 3.3, "600", "DejaVu Sans")
            } else {
                (line.as_str(), 2.7, "400", "DejaVu Sans Mono")
            };
            if !line.starts_with("```") {
                write!(out,"<text x=\"{}\" y=\"{}\" font-size=\"{size}\" font-weight=\"{weight}\" font-family=\"{family}\" xml:space=\"preserve\">{}</text>",options.left_mm,options.top_mm+8.+i as f64*4.5,escape(text)).unwrap();
            }
        }
        out
    };
    let mut pages = Vec::new();
    for chunk in contents.chunks(lines_per_page) {
        pages.push(finish(text_page(chunk), pages.len()));
    }
    // Embed the shared drawing as nested SVG paths/text, never a preview PNG.
    let svg = crate::export::drawing_svg(d, parent)?;
    let opening = svg
        .find('>')
        .ok_or_else(|| Error::new("export", "Invalid composition SVG"))?;
    let view_box = svg[..opening]
        .split("viewBox=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .ok_or_else(|| Error::new("export", "Composition has no viewBox"))?;
    let mut figure = start();
    write!(figure,"<text x=\"{}\" y=\"{}\" font-family=\"DejaVu Sans\" font-size=\"4.2\" font-weight=\"600\">Composition · {}</text><svg x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"{view_box}\" preserveAspectRatio=\"xMidYMin meet\">{}",options.left_mm,options.top_mm+8.,escape(&d.node(parent)?.name),options.left_mm,options.top_mm+14.,w-options.left_mm-options.right_mm,h-options.top_mm-options.bottom_mm-23.,&svg[opening+1..]).unwrap();
    pages.push(finish(figure, pages.len()));
    for chunk in body.chunks(lines_per_page) {
        pages.push(finish(text_page(chunk), pages.len()));
    }
    Ok(pages)
}
