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
    out.push_str("</ul></nav>");
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
    let text = markdown_document(d, parent)?;
    let columns = ((w - options.left_mm - options.right_mm) / 1.65).floor() as usize;
    let lines_per_page = ((h - options.top_mm - options.bottom_mm - 15.) / 4.5).floor() as usize;
    let mut lines = Vec::new();
    for line in text.lines() {
        let chars = line.chars().collect::<Vec<_>>();
        if chars.is_empty() {
            lines.push(String::new());
        } else {
            lines.extend(chars.chunks(columns).map(|c| c.iter().collect::<String>()));
        }
    }
    if lines.len().div_ceil(lines_per_page) > 256 {
        return Err(Error::new("export", "Documentation exceeds 256 pages"));
    }
    let total = lines.len().div_ceil(lines_per_page);
    let mut pages = Vec::new();
    for (page, lines) in lines.chunks(lines_per_page).enumerate() {
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\"><rect width=\"{w}\" height=\"{h}\" fill=\"white\"/><g font-family=\"DejaVu Sans Mono\" font-size=\"2.7\" fill=\"#243b53\">"
        );
        if options.header {
            write!(out,"<text x=\"{}\" y=\"{}\" font-family=\"DejaVu Sans\" font-size=\"3.5\">{} · Functional and Technical Specification</text>",options.left_mm,options.top_mm,escape(d.project_name())).unwrap();
        }
        for (i, line) in lines.iter().enumerate() {
            write!(
                out,
                "<text x=\"{}\" y=\"{}\" xml:space=\"preserve\">{}</text>",
                options.left_mm,
                options.top_mm + 8. + i as f64 * 4.5,
                escape(line)
            )
            .unwrap();
        }
        if options.footer {
            write!(
                out,
                "<text x=\"{}\" y=\"{}\">Wirecraft · {} / {total}</text>",
                options.left_mm,
                h - options.bottom_mm,
                page + 1
            )
            .unwrap();
        }
        out.push_str("</g></svg>");
        pages.push(out);
    }
    Ok(pages)
}
