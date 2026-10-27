use serde_json::json;
use wirecraft_core::{
    Document, NodeKind,
    sheets::{self, SheetOptions},
};
use wirecraft_runtime::service::{Request, Service};
fn fixture() -> Document {
    let mut d = Document::new("Assembled plan");
    for (name, x, y) in [("Left", 2500., 1500.), ("Right", 5000., 2900.)] {
        let id = d.create("root", name, NodeKind::Processor).unwrap();
        d.patch(&id, &json!({"layout":{"x":x,"y":y}})).unwrap();
    }
    d
}
#[test]
fn whole_plan_has_one_technical_frame_grid_and_shared_overlap_marks() {
    let d = fixture();
    let opts = SheetOptions {
        format: "A4".into(),
        split: true,
        ..Default::default()
    };
    let p = sheets::plan(&d, "root", &opts).unwrap();
    assert!(p.columns > 1);
    assert!(p.rows > 1);
    assert_eq!(p.overview.matches("id=\"title-block\"").count(), 1);
    assert!(p.overview.contains("id=\"page-contours\""));
    assert!(p.overview.contains("id=\"alignment-marks\""));
    assert!(p.overview.contains("id=\"folding-marks\""));
    assert_eq!(p.tiles.len(), p.pages.len());
    assert_eq!(p.overview_width_mm, 210. + (p.columns - 1) as f64 * 205.);
    assert_eq!(p.overview_height_mm, 297. + (p.rows - 1) as f64 * 292.);
    for (i, t) in p.tiles.iter().enumerate() {
        assert_eq!(t.row, i / p.columns);
        assert_eq!(t.column, i % p.columns);
        assert!(p.pages[i].contains(&format!("viewBox=\"{} {} 210 297\"", t.x_mm, t.y_mm)));
    }
}
#[test]
fn graph_translation_does_not_create_blank_origin_pages_and_preview_is_explicit() {
    let d = fixture();
    let opts = SheetOptions {
        format: "A4".into(),
        split: true,
        ..Default::default()
    };
    let before = sheets::plan(&d, "root", &opts).unwrap();
    let mut shifted = d.clone();
    for n in shifted.nodes.values_mut() {
        n.layout.x += 10000.;
        n.layout.y += 20000.;
    }
    let after = sheets::plan(&shifted, "root", &opts).unwrap();
    assert_eq!((before.rows, before.columns), (after.rows, after.columns));
    let a = resvg::tiny_skia::Pixmap::decode_png(
        &wirecraft_runtime::export::thumbnail(&before.overview).unwrap(),
    )
    .unwrap();
    let b = resvg::tiny_skia::Pixmap::decode_png(
        &wirecraft_runtime::export::thumbnail(&after.overview).unwrap(),
    )
    .unwrap();
    assert_eq!((a.width(), a.height()), (b.width(), b.height()));
    let error = a
        .data()
        .iter()
        .zip(b.data())
        .map(|(a, b)| a.abs_diff(*b) as f64)
        .sum::<f64>()
        / (a.data().len() as f64 * 255.);
    assert!(
        error < 0.002,
        "translation changed rendered geometry (normalized difference {error})"
    );
    let mut s = Service::new(d, None);
    let overview = s
        .execute(Request::new(
            "export.preview",
            json!({"options":opts,"overview":true}),
        ))
        .unwrap();
    assert_eq!(overview["overview"], true);
    assert_eq!(overview["pages"], before.pages.len());
    assert!(overview["svg"].as_str().unwrap().contains("page-contours"));
    let page = s
        .execute(Request::new(
            "export.preview",
            json!({"options":opts,"page":1}),
        ))
        .unwrap();
    assert_eq!(page["overview"], false);
    assert!(!page["svg"].as_str().unwrap().contains("page-contours"));
}
