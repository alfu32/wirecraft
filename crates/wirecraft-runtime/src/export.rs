use std::path::Path;
use wirecraft_core::{Document, Error, Result, store};
pub fn diagram(document: &Document, parent: &str, path: &Path, format: &str) -> Result<()> {
    let svg = wirecraft_core::export::svg(document, parent)?;
    match format {
        "svg" => store::atomic_write(path, svg.as_bytes()),
        "png" => {
            let mut options = resvg::usvg::Options::default();
            options.fontdb_mut().load_system_fonts();
            let tree = resvg::usvg::Tree::from_str(&svg, &options)
                .map_err(|e| Error::new("export", e.to_string()))?;
            let size = tree.size().to_int_size();
            if u64::from(size.width()) * u64::from(size.height()) > 40_000_000 {
                return Err(Error::new(
                    "export",
                    "Diagram exceeds 40 megapixels; export SVG or PDF instead",
                ));
            }
            let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
                .ok_or_else(|| Error::new("export", "Cannot allocate drawing"))?;
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::identity(),
                &mut pixmap.as_mut(),
            );
            let bytes = pixmap
                .encode_png()
                .map_err(|e| Error::new("export", e.to_string()))?;
            store::atomic_write(path, &bytes)
        }
        "pdf" => {
            let mut options = svg2pdf::usvg::Options::default();
            options.fontdb_mut().load_system_fonts();
            let tree = svg2pdf::usvg::Tree::from_str(&svg, &options)
                .map_err(|e| Error::new("export", e.to_string()))?;
            let bytes = svg2pdf::to_pdf(
                &tree,
                svg2pdf::ConversionOptions::default(),
                svg2pdf::PageOptions::default(),
            )
            .map_err(|e| Error::new("export", e.to_string()))?;
            store::atomic_write(path, &bytes)
        }
        _ => Err(Error::new(
            "invalid_input",
            "Export format must be svg, png or pdf",
        )),
    }
}
