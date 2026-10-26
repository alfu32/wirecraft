//! Technical-sheet pagination in millimeters; source uses four world units per mm.
use crate::{
    Document, Error, Result,
    export::{drawing_svg, escape},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct SheetOptions {
    pub format: String,
    pub scale: u32,
    pub split: bool,
    pub overlap_mm: f64,
}
impl Default for SheetOptions {
    fn default() -> Self {
        Self {
            format: "Auto".into(),
            scale: 1,
            split: false,
            overlap_mm: 5.,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetPlan {
    pub format: String,
    pub width_mm: f64,
    pub height_mm: f64,
    pub rows: usize,
    pub clipped: bool,
    pub columns: usize,
    pub pages: Vec<String>,
}
pub const FORMATS: &[&str] = &[
    "Auto",
    "A4",
    "A3",
    "A2",
    "A1",
    "A0",
    "A4-landscape",
    "A3-landscape",
    "A2-landscape",
    "A1-landscape",
    "A0-landscape",
    "A4-roll",
    "A3-roll",
    "A2-roll",
    "A1-roll",
    "A0-roll",
];
pub fn dimensions(format: &str) -> Result<(f64, f64)> {
    let base = format.split('-').next().unwrap_or("");
    let (mut w, mut h) = match base {
        "A4" => (210., 297.),
        "A3" => (297., 420.),
        "A2" => (420., 594.),
        "A1" => (594., 841.),
        "A0" => (841., 1189.),
        _ => return Err(Error::new("invalid_input", "Unknown sheet format")),
    };
    if format.ends_with("landscape") {
        std::mem::swap(&mut w, &mut h);
    }
    if format.ends_with("roll") {
        h = 1500.;
    }
    Ok((w, h))
}
pub fn plan(d: &Document, parent: &str, options: &SheetOptions) -> Result<SheetPlan> {
    if !FORMATS.contains(&options.format.as_str())
        || ![1, 2, 5, 10, 20, 50, 100, 200, 500, 1000].contains(&options.scale)
        || !options.overlap_mm.is_finite()
        || !(0.0..=100.0).contains(&options.overlap_mm)
    {
        return Err(Error::new(
            "invalid_input",
            "Invalid sheet format, scale or overlap",
        ));
    }
    let diagram = drawing_svg(d, parent)?;
    let attribute = |key: &str| {
        diagram
            .split(&format!("{key}=\""))
            .nth(1)
            .and_then(|s| s.split('"').next())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.)
    };
    let scale = 4. * f64::from(options.scale);
    let content_w = attribute("width") / scale;
    let content_h = attribute("height") / scale;
    let format = if options.format == "Auto" {
        FORMATS
            .iter()
            .filter(|f| **f != "Auto" && !f.contains("roll"))
            .filter_map(|f| dimensions(f).ok().map(|(w, h)| (*f, w, h)))
            .filter(|(_, w, h)| *w >= content_w + 25. && *h >= content_h + 35.)
            .min_by(|a, b| (a.1 * a.2).total_cmp(&(b.1 * b.2)))
            .map(|(f, _, _)| f)
            .unwrap_or("A0-roll")
    } else {
        &options.format
    };
    let (width, mut height) = dimensions(format)?;
    if format.ends_with("roll") {
        height = (content_h + 35.).clamp(100., 1500.);
    }
    let usable_w = width - 25.;
    let usable_h = height - 35.;
    let clipped = (!options.split || format.ends_with("roll"))
        && (content_w > usable_w || content_h > usable_h);
    if options.format == "Auto" && clipped {
        return Err(Error::new(
            "export",
            "Drawing exceeds available Auto sheet formats. Increase the scale denominator or choose a fixed split format.",
        ));
    }
    let overlap = if options.split {
        options.overlap_mm
    } else {
        0.
    };
    if overlap >= usable_w.min(usable_h) {
        return Err(Error::new(
            "invalid_input",
            "Overlap must be smaller than printable area",
        ));
    }
    let count = |needed: f64, available: f64| {
        if needed <= available {
            1
        } else {
            1 + ((needed - available) / (available - overlap)).ceil() as usize
        }
    };
    let columns = if options.split && !format.ends_with("roll") {
        count(content_w, usable_w)
    } else {
        1
    };
    let rows = if options.split && !format.ends_with("roll") {
        count(content_h, usable_h)
    } else {
        1
    };
    if rows.saturating_mul(columns) > 256 {
        return Err(Error::new(
            "export",
            "Sheet plan exceeds 256 pages; choose a smaller scale",
        ));
    }
    if diagram.len().saturating_mul(rows * columns) > 32 * 1024 * 1024 {
        return Err(Error::new(
            "export",
            "Sheet plan exceeds 32 MiB; choose a smaller scale",
        ));
    }
    let mut pages = Vec::new();
    for row in 0..rows {
        for col in 0..columns {
            let x = 20. - col as f64 * (usable_w - overlap);
            let y = 5. - row as f64 * (usable_h - overlap);
            // Nested SVG handles its original viewBox and preserves vector text.
            let nested = diagram
                .replacen("<svg ", &format!("<svg x=\"{x}\" y=\"{y}\" "), 1)
                .replacen(
                    &format!("width=\"{}\"", attribute("width")),
                    &format!("width=\"{content_w}\""),
                    1,
                )
                .replacen(
                    &format!("height=\"{}\"", attribute("height")),
                    &format!("height=\"{content_h}\""),
                    1,
                );
            let technical = technical_block(
                d,
                parent,
                [width, height],
                format,
                options.scale,
                [pages.len() + 1, rows * columns],
            )?;
            let warning = if clipped {
                format!(
                    "<text x=\"22\" y=\"{}\" font-family=\"DejaVu Sans\" font-size=\"3\" fill=\"#bd3333\">Drawing exceeds this fixed sheet: enable split or increase scale denominator.</text>",
                    height - 9.
                )
            } else {
                String::new()
            };
            pages.push(format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}mm\" height=\"{height}mm\" viewBox=\"0 0 {width} {height}\"><rect width=\"{width}\" height=\"{height}\" fill=\"white\"/><defs><clipPath id=\"drawingClip\"><rect x=\"20\" y=\"5\" width=\"{usable_w}\" height=\"{usable_h}\"/></clipPath></defs><g clip-path=\"url(#drawingClip)\">{nested}</g><g font-family=\"DejaVu Sans\" font-size=\"3\" fill=\"#263c50\"><rect x=\"20\" y=\"5\" width=\"{usable_w}\" height=\"{usable_h}\" fill=\"none\" stroke=\"#526777\" stroke-width=\".25\"/><text x=\"22\" y=\"{}\">{} · {} · Scale 1:{} · Page {} / {} (row {}, column {})</text><text x=\"22\" y=\"{}\">Revision: {} · Wirecraft technical plan</text></g>{technical}{warning}</svg>",height-23.,escape(d.project_name()),format,options.scale,pages.len()+1,rows*columns,row+1,col+1,height-17.,escape(d.master_revision["name"].as_str().unwrap_or(""))))
        }
    }
    Ok(SheetPlan {
        format: format.into(),
        width_mm: width,
        height_mm: height,
        rows,
        clipped,
        columns,
        pages,
    })
}

fn technical_block(
    d: &Document,
    parent: &str,
    dimensions: [f64; 2],
    format: &str,
    scale: u32,
    pagination: [usize; 2],
) -> Result<String> {
    let [width, height] = dimensions;
    let [page, pages] = pagination;
    use std::fmt::Write;
    let drawing = crate::drawing::Drawing::new(d, parent);
    let mut ids = drawing.shapes.keys().cloned().collect::<Vec<_>>();
    ids.sort_by_key(|id| (d.nodes[id].name.to_lowercase(), id.clone()));
    let table_w = (width - 30.).min(180.);
    let x = width - 5. - table_w;
    let row_h = 4.;
    let max_rows = ((height - 80.) / row_h).max(1.) as usize;
    let rows = ids.len().min(max_rows);
    let table_h = (rows + 2) as f64 * row_h;
    let columns = [4., 26., 22., 31., 9., 25., 16.];
    let total = columns.iter().sum::<f64>();
    let mut offsets = vec![0.];
    for c in columns {
        offsets.push(offsets.last().unwrap() + c / total * table_w);
    }
    let mut out = format!(
        "<g font-family=\"DejaVu Sans\" fill=\"#243b53\" stroke=\"#637889\" stroke-width=\".15\"><rect x=\"{x}\" y=\"5\" width=\"{table_w}\" height=\"{table_h}\" fill=\"white\"/><g stroke=\"none\" font-size=\"2.1\"><text x=\"{}\" y=\"8\">PARTS LIST</text></g>",
        x + 2.
    );
    for offset in &offsets[1..offsets.len() - 1] {
        write!(out, "<path d=\"M{} 9v{}\"/>", x + offset, table_h - 4.).unwrap();
    }
    for row in 1..rows + 2 {
        write!(
            out,
            "<path d=\"M{x} {}h{table_w}\"/>",
            5. + row as f64 * row_h
        )
        .unwrap();
    }
    out.push_str("<g stroke=\"none\" font-size=\"1.6\">");
    for (i, title) in [
        "No.",
        "Part",
        "Kind",
        "Modified",
        "Rev.",
        "Responsible",
        "Signature",
    ]
    .into_iter()
    .enumerate()
    {
        write!(
            out,
            "<text x=\"{}\" y=\"12\">{title}</text>",
            x + offsets[i] + 0.8
        )
        .unwrap();
    }
    for (row, id) in ids.iter().take(max_rows).enumerate() {
        let n = d.node(id)?;
        let revision = d
            .effective_revision(id)
            .and_then(|r| r["name"].as_str())
            .unwrap_or("");
        let responsible = d.effective(id, "responsible");
        let modified = n.modified["date"].as_str().unwrap_or("");
        let values = [
            (row + 1).to_string(),
            n.name.clone(),
            crate::analysis::stereotype(d, n).into(),
            modified.into(),
            revision.into(),
            if responsible == "none" {
                String::new()
            } else {
                responsible
            },
            String::new(),
        ];
        for (i, value) in values.into_iter().enumerate() {
            let limit = ((offsets[i + 1] - offsets[i] - 1.5) / 0.85).max(1.) as usize;
            let display = value.chars().take(limit).collect::<String>();
            write!(
                out,
                "<text x=\"{}\" y=\"{}\">{}</text>",
                x + offsets[i] + 0.8,
                16. + row as f64 * row_h,
                escape(&display)
            )
            .unwrap();
        }
    }
    if ids.len() > max_rows {
        write!(
            out,
            "<text x=\"{}\" y=\"{}\">+{} more; see complete documentation</text>",
            x + 2.,
            table_h + 8.,
            ids.len() - max_rows
        )
        .unwrap();
    }
    out.push_str("</g>");
    let block_w = (width - 25.).min(180.);
    let bx = width - 5. - block_w;
    let by = height - 37.;
    let revision = d.master_revision["name"].as_str().unwrap_or("");
    let date = d.master_revision["date"].as_str().unwrap_or("");
    write!(out,"<rect x=\"{bx}\" y=\"{by}\" width=\"{block_w}\" height=\"32\" fill=\"white\"/><path d=\"M{bx} {}h{block_w}M{bx} {}h{block_w}\"/><g stroke=\"none\"><text x=\"{}\" y=\"{}\" font-size=\"4\">{}</text><text x=\"{}\" y=\"{}\" font-size=\"2.5\">{}</text><text x=\"{}\" y=\"{}\" font-size=\"2.5\">Revision: {} · {}</text><text x=\"{}\" y=\"{}\" font-size=\"2.5\">Format: {format} · Scale 1:{scale} · Sheet {page}/{pages} · Wirecraft</text></g></g>",by+15.,by+24.,bx+3.,by+6.,escape(d.project_name()),bx+3.,by+12.,escape(&d.qualified_name(parent)),bx+3.,by+21.,escape(revision),escape(date),bx+3.,by+29.).unwrap();
    let mut mark = 210.;
    while mark < width - 5. {
        write!(
            out,
            "<path d=\"M{mark} 1v3M{mark} {}v3\" stroke=\"#637889\" stroke-width=\".2\"/>",
            height - 4.
        )
        .unwrap();
        mark += 185.;
    }
    Ok(out)
}
