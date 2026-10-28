//! Technical-sheet pagination in millimeters; source uses four world units per mm.
use crate::{
    Document, Error, Result,
    export::{escape, plan_svg},
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
    pub overview: String,
    pub overview_width_mm: f64,
    pub overview_height_mm: f64,
    pub tiles: Vec<SheetTile>,
    /// Page contours in canvas world units, in the same order as `tiles`.
    #[serde(skip)]
    pub world_tiles: Vec<crate::viewport::Rect>,
    /// Title block and parts list contours in canvas world units.
    #[serde(skip)]
    pub world_blocks: Vec<SheetBlock>,
}
/// A fixed block of the technical sheet, such as the title block.
#[derive(Clone, Debug)]
pub struct SheetBlock {
    pub label: &'static str,
    pub rect: crate::viewport::Rect,
}
/// Millimetre rectangles of the parts list and title block on the assembled sheet.
struct BlockLayout {
    parts: [f64; 4],
    title: [f64; 4],
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
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetTile {
    pub row: usize,
    pub column: usize,
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}
pub fn plan(d: &Document, parent: &str, options: &SheetOptions) -> Result<SheetPlan> {
    use std::fmt::Write;
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
    let diagram = plan_svg(d, parent)?;
    let attribute = |key: &str| {
        diagram
            .split(&format!("{key}=\""))
            .nth(1)
            .and_then(|s| s.split('"').next())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.)
    };
    let unit_scale = 4. * f64::from(options.scale);
    let content_w = attribute("width") / unit_scale;
    let content_h = attribute("height") / unit_scale;
    let format = if options.format == "Auto" {
        FORMATS
            .iter()
            .filter(|f| **f != "Auto" && !f.contains("roll"))
            .filter_map(|f| dimensions(f).ok().map(|(w, h)| (*f, w, h)))
            .filter(|(_, w, h)| *w >= content_w + 25. && *h >= content_h + 10.)
            .min_by(|a, b| (a.1 * a.2).total_cmp(&(b.1 * b.2)))
            .map(|(f, _, _)| f)
            .unwrap_or("A0-roll")
    } else {
        &options.format
    };
    let (width, mut height) = dimensions(format)?;
    if format.ends_with("roll") {
        height = (content_h + 10.).clamp(100., 1500.);
    }
    let clipped = (!options.split || format.ends_with("roll"))
        && (content_w + 25. > width || content_h + 10. > height);
    if options.format == "Auto" && clipped {
        return Err(Error::new(
            "export",
            "Drawing exceeds available Auto sheet formats. Increase the scale denominator or choose a fixed split format.",
        ));
    }
    let overlap = if options.split && !format.ends_with("roll") {
        options.overlap_mm
    } else {
        0.
    };
    if overlap >= (width - 25.).min(height - 10.) {
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
    let tiled = options.split && !format.ends_with("roll");
    let columns = if tiled {
        count(content_w + 25., width)
    } else {
        1
    };
    let rows = if tiled {
        count(content_h + 10., height)
    } else {
        1
    };
    if rows.saturating_mul(columns) > 256 {
        return Err(Error::new(
            "export",
            "Sheet plan exceeds 256 pages; choose a smaller scale",
        ));
    }
    let total_w = width + (columns - 1) as f64 * (width - overlap);
    let total_h = height + (rows - 1) as f64 * (height - overlap);
    let drawing_w = total_w - 25.;
    let drawing_h = total_h - 10.;
    let x = 20. + (drawing_w - content_w) / 2.;
    let y = 5. + (drawing_h - content_h) / 2.;
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
    // The nested drawing maps its viewBox origin to (x, y) mm at `unit_scale` world units per mm.
    let view_box = diagram
        .split("viewBox=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .map(|s| {
            s.split(' ')
                .filter_map(|v| v.parse::<f64>().ok())
                .collect::<Vec<_>>()
        })
        .filter(|v| v.len() == 4)
        .unwrap_or_else(|| vec![0., 0., 1., 1.]);
    let tiles = (0..rows)
        .flat_map(|row| {
            (0..columns).map(move |column| SheetTile {
                row,
                column,
                x_mm: column as f64 * (width - overlap),
                y_mm: row as f64 * (height - overlap),
                width_mm: width,
                height_mm: height,
            })
        })
        .collect::<Vec<_>>();
    let (technical, blocks) = technical_block(
        d,
        parent,
        [total_w, total_h],
        format,
        options.scale,
        [1, tiles.len()],
    )?;
    let mut composition = format!(
        "<defs><clipPath id=\"drawingClip\"><rect x=\"20\" y=\"5\" width=\"{drawing_w}\" height=\"{drawing_h}\"/></clipPath></defs><g id=\"technical-sheet\">{technical}<rect x=\"20\" y=\"5\" width=\"{drawing_w}\" height=\"{drawing_h}\" fill=\"none\" stroke=\"#777777\" stroke-width=\".25\"/></g><g id=\"plan-drawing\" clip-path=\"url(#drawingClip)\">{nested}</g>"
    );
    if clipped {
        write!(composition,"<text x=\"22\" y=\"{}\" font-family=\"DejaVu Sans\" font-size=\"3\" fill=\"#bd3333\">Drawing exceeds this fixed sheet: enable split or increase scale denominator.</text>",total_h-8.).unwrap();
    }
    let mut contours = String::from(
        "<g id=\"page-contours\" fill=\"none\" stroke=\"#aaaaaa\" stroke-width=\".15\">",
    );
    let mut crosses = std::collections::BTreeSet::new();
    for tile in &tiles {
        write!(contours,"<rect data-row=\"{}\" data-column=\"{}\" x=\"{}\" y=\"{}\" width=\"{width}\" height=\"{height}\"/>",tile.row,tile.column,tile.x_mm,tile.y_mm).unwrap();
        if overlap > 0. {
            let mut add = |x: f64, y: f64| {
                crosses.insert(((x * 1000.).round() as i64, (y * 1000.).round() as i64));
            };
            for fraction in [0.25, 0.75] {
                if tile.row > 0 {
                    add(tile.x_mm + width * fraction, tile.y_mm + overlap / 2.);
                }
                if tile.row + 1 < rows {
                    add(
                        tile.x_mm + width * fraction,
                        tile.y_mm + height - overlap / 2.,
                    );
                }
                if tile.column > 0 {
                    add(tile.x_mm + overlap / 2., tile.y_mm + height * fraction);
                }
                if tile.column + 1 < columns {
                    add(
                        tile.x_mm + width - overlap / 2.,
                        tile.y_mm + height * fraction,
                    );
                }
            }
        }
    }
    contours.push_str("</g>");
    let mut alignment = String::from(
        "<g id=\"alignment-marks\" fill=\"none\" stroke=\"#555555\" stroke-width=\".15\">",
    );
    for (x, y) in crosses {
        let x = x as f64 / 1000.;
        let y = y as f64 / 1000.;
        write!(
            alignment,
            "<path d=\"M{} {y}h6M{x} {}v6\"/>",
            x - 3.,
            y - 3.
        )
        .unwrap();
    }
    alignment.push_str("</g>");
    if (composition.len() + alignment.len()).saturating_mul(tiles.len()) > 32 * 1024 * 1024 {
        return Err(Error::new(
            "export",
            "Sheet plan exceeds 32 MiB; choose a smaller scale",
        ));
    }
    let overview = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{total_w}mm\" height=\"{total_h}mm\" viewBox=\"0 0 {total_w} {total_h}\"><rect width=\"{total_w}\" height=\"{total_h}\" fill=\"white\"/>{composition}{contours}{alignment}</svg>"
    );
    let pages=tiles.iter().map(|tile|format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}mm\" height=\"{height}mm\" viewBox=\"{} {} {width} {height}\"><rect x=\"{}\" y=\"{}\" width=\"{width}\" height=\"{height}\" fill=\"white\"/>{composition}{alignment}</svg>",tile.x_mm,tile.y_mm,tile.x_mm,tile.y_mm)).collect();
    let world = |[left, top, w, h]: [f64; 4]| crate::viewport::Rect {
        x: view_box[0] + (left - x) * unit_scale,
        y: view_box[1] + (top - y) * unit_scale,
        width: w * unit_scale,
        height: h * unit_scale,
    };
    let world_tiles = tiles
        .iter()
        .map(|t| world([t.x_mm, t.y_mm, t.width_mm, t.height_mm]))
        .collect();
    let world_blocks = vec![
        SheetBlock {
            label: "Parts list",
            rect: world(blocks.parts),
        },
        SheetBlock {
            label: "Title block",
            rect: world(blocks.title),
        },
    ];
    Ok(SheetPlan {
        world_tiles,
        world_blocks,
        format: format.into(),
        width_mm: width,
        height_mm: height,
        rows,
        columns,
        clipped,
        pages,
        overview,
        overview_width_mm: total_w,
        overview_height_mm: total_h,
        tiles,
    })
}
fn technical_block(
    d: &Document,
    parent: &str,
    dimensions: [f64; 2],
    format: &str,
    scale: u32,
    pagination: [usize; 2],
) -> Result<(String, BlockLayout)> {
    let [width, height] = dimensions;
    let [page, pages] = pagination;
    use std::fmt::Write;
    let drawing = crate::drawing::Drawing::new(d, parent);
    let mut ids = drawing.shapes.keys().cloned().collect::<Vec<_>>();
    ids.sort_by_key(|id| (d.nodes[id].name.to_lowercase(), id.clone()));
    let labels = [
        "No.",
        "Part",
        "Kind",
        "Modified",
        "Rev.",
        "Responsible",
        "Signature",
    ];
    let mut widths = labels.map(|s| text_width(s) + 2.);
    for (row, id) in ids.iter().enumerate() {
        let node = d.node(id)?;
        for (column, value) in [
            (row + 1).to_string(),
            node.name.clone(),
            crate::analysis::stereotype(d, node).into(),
            parts_date(node.modified["date"].as_str().unwrap_or("")),
            d.effective_revision(id)
                .and_then(|v| v["name"].as_str())
                .unwrap_or("")
                .into(),
            d.effective(id, "responsible"),
            String::new(),
        ]
        .into_iter()
        .enumerate()
        {
            widths[column] = widths[column].max(text_width(&value) + 2.);
        }
    }
    for (w, max) in widths.iter_mut().zip([9., 45., 38., 32., 12., 38., 27.5]) {
        *w = w.min(max);
    }
    widths[6] = 27.5;
    let table_w = widths.iter().sum::<f64>().min(width - 25.);
    let x = width - 5. - table_w;
    let row_h = 5.;
    let max_rows = (((height - 54.) / row_h) as usize).saturating_sub(2).max(1);
    let rows = ids.len().min(max_rows);
    let table_h = (rows + 2) as f64 * row_h;
    let columns = widths;
    let total = columns.iter().sum::<f64>();
    let mut offsets = vec![0.];
    for c in columns {
        offsets.push(offsets.last().unwrap() + c / total * table_w);
    }
    let mut out = format!(
        "<g font-family=\"DejaVu Sans\" fill=\"#243b53\" stroke=\"#637889\" stroke-width=\".15\"><rect x=\"{x}\" y=\"5\" width=\"{table_w}\" height=\"{table_h}\" fill=\"white\"/><g stroke=\"none\" font-size=\"2.5\"><text x=\"{}\" y=\"8.5\">PARTS LIST</text></g>",
        x + 2.
    );
    for offset in &offsets[1..offsets.len() - 1] {
        write!(out, "<path d=\"M{} 10v{}\"/>", x + offset, table_h - 5.).unwrap();
    }
    for row in 1..rows + 2 {
        write!(
            out,
            "<path d=\"M{x} {}h{table_w}\"/>",
            5. + row as f64 * row_h
        )
        .unwrap();
    }
    out.push_str("<g stroke=\"none\" font-size=\"2.2\">");
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
            "<text x=\"{}\" y=\"13.5\">{title}</text>",
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
        let modified = parts_date(n.modified["date"].as_str().unwrap_or(""));
        let values = [
            (row + 1).to_string(),
            n.name.clone(),
            crate::analysis::stereotype(d, n).into(),
            modified,
            revision.into(),
            if responsible == "none" {
                String::new()
            } else {
                responsible
            },
            String::new(),
        ];
        for (i, value) in values.into_iter().enumerate() {
            let limit = ((offsets[i + 1] - offsets[i] - 1.5) / 1.1).max(1.) as usize;
            let display = value.chars().take(limit).collect::<String>();
            write!(
                out,
                "<text x=\"{}\" y=\"{}\">{}</text>",
                x + offsets[i] + 0.8,
                18.5 + row as f64 * row_h,
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
    let by = height - 41.;
    let revision = d.master_revision["name"].as_str().unwrap_or("");
    let date = d.master_revision["date"].as_str().unwrap_or("");
    write!(out,"<g id=\"title-block\"><rect x=\"{bx}\" y=\"{by}\" width=\"{block_w}\" height=\"36\" fill=\"white\"/>").unwrap();
    for row in 1..5 {
        write!(
            out,
            "<path d=\"M{bx} {}h{block_w}\"/>",
            by + row as f64 * 7.2
        )
        .unwrap();
    }
    for offset in [45., 112.] {
        write!(
            out,
            "<path d=\"M{} {by}v36\"/>",
            bx + offset * block_w / 180.
        )
        .unwrap();
    }
    write!(out,"<g stroke=\"none\" font-size=\"2.5\"><text x=\"{}\" y=\"{}\">Project</text><text x=\"{}\" y=\"{}\">{}</text><text x=\"{}\" y=\"{}\">Format: {format}</text><text x=\"{}\" y=\"{}\">Revision</text><text x=\"{}\" y=\"{}\">{} · {}</text><text x=\"{}\" y=\"{}\">Scale 1:{scale}</text><text x=\"{}\" y=\"{}\">Wirecraft {} · {pages} sheets</text></g></g></g>",bx+2.,by+5.,bx+47.*block_w/180.,by+5.,escape(d.project_name()),bx+114.*block_w/180.,by+5.,bx+2.,by+12.2,bx+47.*block_w/180.,by+12.2,escape(revision),escape(date),bx+114.*block_w/180.,by+12.2,bx+2.,by+33.5,env!("CARGO_PKG_VERSION")).unwrap();
    let _ = page;
    out.push_str("<g id=\"folding-marks\" fill=\"none\" stroke=\"#999999\" stroke-width=\".2\">");
    let mut offset = 210.;
    while offset < width {
        write!(out, "<path d=\"M{} {}v-5\"/>", width - offset, height).unwrap();
        offset += 210.;
    }
    offset = 297.;
    while offset < height {
        write!(out, "<path d=\"M{width} {}h-5\"/>", height - offset).unwrap();
        offset += 297.;
    }
    out.push_str("</g>");
    Ok((
        out,
        BlockLayout {
            parts: [x, 5., table_w, table_h],
            title: [bx, by, block_w, 36.],
        },
    ))
}
fn text_width(value: &str) -> f64 {
    value
        .chars()
        .map(|c| {
            if "il.,:;!| '".contains(c) {
                0.65
            } else if "MW@".contains(c) {
                1.9
            } else {
                1.2
            }
        })
        .sum()
}
fn parts_date(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|v| v.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|_| value.into())
}
