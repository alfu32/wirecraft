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

/// Write an SVG, PNG, or a real multipage vector PDF from a sheet plan.
/// Single-image formats reject multiple pages rather than dropping content.
pub fn pages(pages: &[String], path: &Path, format: &str) -> Result<()> {
    pages_with_mode(pages, path, format, false)
}
pub fn pages_with_mode(
    pages: &[String],
    path: &Path,
    format: &str,
    rasterized: bool,
) -> Result<()> {
    if pages.is_empty() {
        return Err(Error::new("invalid_input", "No export pages"));
    }
    if format != "pdf" && pages.len() > 1 {
        return Err(Error::new(
            "invalid_input",
            "Use PDF for multipage output, or disable splitting for SVG/PNG",
        ));
    }
    if format == "svg" {
        return store::atomic_write(path, pages[0].as_bytes());
    }
    if format == "png" {
        let mut options = resvg::usvg::Options::default();
        options.fontdb_mut().load_system_fonts();
        let tree = resvg::usvg::Tree::from_str(&pages[0], &options)
            .map_err(|e| Error::new("export", e.to_string()))?;
        let size = tree.size().to_int_size();
        if u64::from(size.width()) * u64::from(size.height()) > 40_000_000 {
            return Err(Error::new(
                "export",
                "Page exceeds 40 megapixels; use SVG/PDF",
            ));
        }
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
            .ok_or_else(|| Error::new("export", "Cannot allocate page"))?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );
        return store::atomic_write(
            path,
            &pixmap
                .encode_png()
                .map_err(|e| Error::new("export", e.to_string()))?,
        );
    }
    if format != "pdf" {
        return Err(Error::new("invalid_input", "Expected svg, png or pdf"));
    }
    use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref};
    let mut pdf = Pdf::new();
    let mut alloc = Ref::new(1);
    let catalog = alloc.bump();
    let tree_ref = alloc.bump();
    let refs = (0..pages.len()).map(|_| alloc.bump()).collect::<Vec<_>>();
    pdf.catalog(catalog).pages(tree_ref);
    pdf.pages(tree_ref)
        .kids(refs.iter().copied())
        .count(pages.len() as i32);
    let mut options = svg2pdf::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    for (svg, page_ref) in pages.iter().zip(refs) {
        let tree = svg2pdf::usvg::Tree::from_str(svg, &options)
            .map_err(|e| Error::new("export", e.to_string()))?;
        let (image_ref, chunk) = if rasterized {
            let size = tree.size().to_int_size();
            if u64::from(size.width()) * u64::from(size.height()) > 40_000_000 {
                return Err(Error::new(
                    "export",
                    "Raster PDF page exceeds 40 megapixels",
                ));
            }
            let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
                .ok_or_else(|| Error::new("export", "Cannot allocate raster page"))?;
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::identity(),
                &mut pixmap.as_mut(),
            );
            let rgb = pixmap
                .data()
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2]])
                .collect::<Vec<_>>();
            let image_ref = alloc.bump();
            let mut image = pdf.image_xobject(image_ref, &rgb);
            image
                .width(size.width() as i32)
                .height(size.height() as i32)
                .bits_per_component(8)
                .color_space()
                .device_rgb();
            image.finish();
            (image_ref, None)
        } else {
            let (chunk, image_ref) =
                svg2pdf::to_chunk(&tree, svg2pdf::ConversionOptions::default())
                    .map_err(|e| Error::new("export", e.to_string()))?;
            let mut map = std::collections::HashMap::new();
            let chunk = chunk.renumber(|old| *map.entry(old).or_insert_with(|| alloc.bump()));
            (map[&image_ref], Some(chunk))
        };
        let content_ref = alloc.bump();
        let width = tree.size().width() * 72. / 96.;
        let height = tree.size().height() * 72. / 96.;
        let mut page = pdf.page(page_ref);
        page.media_box(Rect::new(0., 0., width, height))
            .parent(tree_ref)
            .contents(content_ref);
        page.resources()
            .x_objects()
            .pair(Name(b"Diagram"), image_ref);
        page.finish();
        let mut content = Content::new();
        content
            .transform([width, 0., 0., height, 0., 0.])
            .x_object(Name(b"Diagram"));
        pdf.stream(content_ref, &content.finish());
        if let Some(chunk) = chunk {
            pdf.extend(&chunk);
        }
    }
    store::atomic_write(path, &pdf.finish())
}

/// Bounded preview rasterization stays off the GPUI thread and avoids full A0 textures.
pub fn thumbnail(svg: &str) -> Result<Vec<u8>> {
    let mut options = resvg::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_str(svg, &options)
        .map_err(|e| Error::new("export", e.to_string()))?;
    let scale = (1200. / tree.size().width().max(tree.size().height())).min(1.);
    let width = (tree.size().width() * scale).ceil().max(1.) as u32;
    let height = (tree.size().height() * scale).ceil().max(1.) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| Error::new("export", "Cannot allocate preview"))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|e| Error::new("export", e.to_string()))
}
