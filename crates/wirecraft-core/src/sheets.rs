//! Technical-sheet pagination in millimeters; source uses four world units per mm.
use crate::{
    Document, Error, Result,
    export::{escape, svg},
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
    let diagram = svg(d, parent)?;
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
            pages.push(format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}mm\" height=\"{height}mm\" viewBox=\"0 0 {width} {height}\"><rect width=\"{width}\" height=\"{height}\" fill=\"white\"/><defs><clipPath id=\"drawingClip\"><rect x=\"20\" y=\"5\" width=\"{usable_w}\" height=\"{usable_h}\"/></clipPath></defs><g clip-path=\"url(#drawingClip)\">{nested}</g><g font-family=\"DejaVu Sans\" font-size=\"3\" fill=\"#263c50\"><rect x=\"20\" y=\"5\" width=\"{usable_w}\" height=\"{usable_h}\" fill=\"none\" stroke=\"#526777\" stroke-width=\".25\"/><text x=\"22\" y=\"{}\">{} · {} · Scale 1:{} · Page {} / {} (row {}, column {})</text><text x=\"22\" y=\"{}\">Revision: {} · Wirecraft technical plan</text></g></svg>",height-23.,escape(d.project_name()),format,options.scale,pages.len()+1,rows*columns,row+1,col+1,height-17.,escape(d.master_revision["name"].as_str().unwrap_or(""))))
        }
    }
    Ok(SheetPlan {
        format: format.into(),
        width_mm: width,
        height_mm: height,
        rows,
        columns,
        pages,
    })
}
