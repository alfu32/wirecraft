//! Wirecraft's 24-unit, single-weight vector vocabulary. No raster/font dependency.
use gpui::*;
pub fn icon(name: &'static str) -> AnyElement {
    canvas(
        |bounds, _, _| bounds,
        move |bounds, _, window, _| {
            let lines: &[&[[f32; 2]]] = match name {
                "new" => &[
                    &[
                        [6., 3.],
                        [15., 3.],
                        [19., 7.],
                        [19., 21.],
                        [5., 21.],
                        [5., 3.],
                    ],
                    &[[14., 3.], [14., 8.], [19., 8.]],
                    &[[8., 14.], [16., 14.]],
                    &[[12., 10.], [12., 18.]],
                ],
                "open" => &[&[
                    [3., 19.],
                    [3., 6.],
                    [9., 6.],
                    [11., 9.],
                    [21., 9.],
                    [18., 19.],
                    [3., 19.],
                ]],
                "save" => &[
                    &[
                        [4., 3.],
                        [17., 3.],
                        [21., 7.],
                        [21., 21.],
                        [3., 21.],
                        [3., 3.],
                        [4., 3.],
                    ],
                    &[[7., 3.], [7., 9.], [16., 9.], [16., 3.]],
                    &[[7., 21.], [7., 14.], [17., 14.], [17., 21.]],
                ],
                "undo" => &[
                    &[[8., 5.], [3., 10.], [8., 15.]],
                    &[[3., 10.], [15., 10.], [20., 13.], [20., 19.]],
                ],
                "redo" => &[
                    &[[16., 5.], [21., 10.], [16., 15.]],
                    &[[21., 10.], [9., 10.], [4., 13.], [4., 19.]],
                ],
                "select" => &[&[[5., 3.], [19., 13.], [12., 14.], [9., 21.], [5., 3.]]],
                "node" => &[
                    &[[4., 5.], [20., 5.], [20., 19.], [4., 19.], [4., 5.]],
                    &[[8., 9.], [16., 9.]],
                    &[[8., 13.], [13., 13.]],
                ],
                "type" => &[
                    &[
                        [12., 3.],
                        [21., 8.],
                        [21., 17.],
                        [12., 22.],
                        [3., 17.],
                        [3., 8.],
                        [12., 3.],
                        [12., 22.],
                    ],
                    &[[3., 8.], [12., 13.], [21., 8.]],
                ],
                "port" => &[
                    &[
                        [12., 3.],
                        [18.4, 5.6],
                        [21., 12.],
                        [18.4, 18.4],
                        [12., 21.],
                        [5.6, 18.4],
                        [3., 12.],
                        [5.6, 5.6],
                        [12., 3.],
                    ],
                    &[[10., 8.], [14., 12.], [10., 16.]],
                ],
                "link" => &[
                    &[[3., 6.], [8., 6.], [16., 18.], [21., 18.]],
                    &[[17., 14.], [21., 18.], [17., 22.]],
                ],
                "fit" => &[
                    &[[3., 9.], [3., 3.], [9., 3.]],
                    &[[15., 3.], [21., 3.], [21., 9.]],
                    &[[21., 15.], [21., 21.], [15., 21.]],
                    &[[9., 21.], [3., 21.], [3., 15.]],
                ],
                "copy" => &[
                    &[[8., 8.], [21., 8.], [21., 21.], [8., 21.], [8., 8.]],
                    &[[16., 4.], [3., 4.], [3., 17.]],
                ],
                "cut" => &[
                    &[[4., 4.], [20., 20.]],
                    &[[4., 20.], [20., 4.]],
                    &[[4., 4.], [3., 7.], [6., 9.], [9., 8.]],
                    &[[4., 20.], [3., 17.], [6., 15.], [9., 16.]],
                ],
                "paste" => &[
                    &[
                        [8., 5.],
                        [4., 5.],
                        [4., 21.],
                        [20., 21.],
                        [20., 5.],
                        [16., 5.],
                    ],
                    &[[8., 3.], [16., 3.], [16., 8.], [8., 8.], [8., 3.]],
                ],
                "sheet" => &[
                    &[[4., 3.], [20., 3.], [20., 21.], [4., 21.], [4., 3.]],
                    &[[7., 7.], [17., 7.]],
                    &[[7., 11.], [17., 11.]],
                    &[[12., 15.], [17., 15.], [17., 18.], [12., 18.], [12., 15.]],
                ],
                "code" => &[
                    &[[8., 6.], [2., 12.], [8., 18.]],
                    &[[16., 6.], [22., 12.], [16., 18.]],
                    &[[14., 3.], [10., 21.]],
                ],
                "board" => &[
                    &[[3., 4.], [21., 4.], [21., 21.], [3., 21.], [3., 4.]],
                    &[[9., 4.], [9., 21.]],
                    &[[15., 4.], [15., 21.]],
                    &[[5., 8.], [7., 8.]],
                    &[[11., 12.], [13., 12.]],
                    &[[17., 8.], [19., 8.]],
                ],
                "analysis" => &[
                    &[[3., 3.], [3., 21.], [22., 21.]],
                    &[[7., 17.], [11., 11.], [15., 14.], [21., 5.]],
                ],
                "settings" => &[
                    &[[3., 6.], [21., 6.]],
                    &[[3., 12.], [21., 12.]],
                    &[[3., 18.], [21., 18.]],
                    &[[8., 3.], [8., 9.]],
                    &[[16., 9.], [16., 15.]],
                    &[[10., 15.], [10., 21.]],
                ],
                "server" => &[
                    &[[3., 4.], [21., 4.], [21., 11.], [3., 11.], [3., 4.]],
                    &[[3., 14.], [21., 14.], [21., 21.], [3., 21.], [3., 14.]],
                    &[[6., 7.], [9., 7.]],
                    &[[6., 17.], [9., 17.]],
                ],
                "search" => &[
                    &[
                        [10., 3.],
                        [15., 5.],
                        [17., 10.],
                        [15., 15.],
                        [10., 17.],
                        [5., 15.],
                        [3., 10.],
                        [5., 5.],
                        [10., 3.],
                    ],
                    &[[15., 15.], [22., 22.]],
                ],
                "up" => &[
                    &[[5., 12.], [12., 5.], [19., 12.]],
                    &[[12., 5.], [12., 21.]],
                ],
                "close" => &[&[[6., 6.], [18., 18.]], &[[6., 18.], [18., 6.]]],
                "check" => &[&[[4., 12.], [9., 18.], [21., 5.]]],
                "spark" => &[&[
                    [12., 2.],
                    [15., 9.],
                    [22., 12.],
                    [15., 15.],
                    [12., 22.],
                    [9., 15.],
                    [2., 12.],
                    [9., 9.],
                    [12., 2.],
                ]],
                _ => &[
                    &[[5., 4.], [19., 4.], [19., 20.], [5., 20.], [5., 4.]],
                    &[[8., 9.], [16., 9.]],
                    &[[8., 14.], [16., 14.]],
                ],
            };
            let mut path = PathBuilder::stroke(px(1.5));
            for line in lines {
                for (i, p) in line.iter().enumerate() {
                    let point = bounds.origin
                        + point(
                            bounds.size.width * p[0] / 24.,
                            bounds.size.height * p[1] / 24.,
                        );
                    if i == 0 {
                        path.move_to(point);
                    } else {
                        path.line_to(point);
                    }
                }
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, crate::chrome::color(0x9babb9));
            }
        },
    )
    .size(px(16.))
    .flex_none()
    .into_any_element()
}
pub fn for_button(id: &str) -> &'static str {
    match id {
        "new" => "new",
        "open" | "import-archetype" => "open",
        "save" | "save-as" | "save-archetype" => "save",
        "undo" => "undo",
        "redo" => "redo",
        "select" => "select",
        "node" | "Designer" => "node",
        "type" | "Archetypes" => "type",
        "link" => "link",
        "port" => "port",
        "fit" => "fit",
        "up" => "up",
        "copy-graph" | "copy-fiches" | "copy-mcp" => "copy",
        "cut-graph" => "cut",
        "paste-graph" => "paste",
        "Export" | "sheet" => "sheet",
        "Editor" | "Tools" => "code",
        "Analysis" => "analysis",
        "Project Management" => "board",
        "MCP" => "server",
        "apply" => "check",
        _ => "",
    }
}

/// Embedded controls use the same stroke vocabulary, including editor search and color picker.
pub struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        let data = match path {
            "icons/check.svg" => "M4 12l5 6L21 5",
            "icons/close.svg" | "icons/window-close.svg" => "M6 6l12 12M6 18L18 6",
            "icons/window-minimize.svg" => "M5 17h14",
            "icons/window-maximize.svg" => "M5 5h14v14H5z",
            "icons/window-restore.svg" => "M8 5h11v11M5 8h11v11H5z",
            "icons/chevron-down.svg" => "M5 9l7 7 7-7",
            "icons/chevron-up.svg" => "M5 15l7-7 7 7",
            "icons/chevron-left.svg" => "M15 5l-7 7 7 7",
            "icons/chevron-right.svg" => "M9 5l7 7-7 7",
            "icons/arrow-down.svg" => "M12 3v18M5 14l7 7 7-7",
            "icons/arrow-up.svg" => "M12 21V3M5 10l7-7 7 7",
            "icons/search.svg" => "M15 15l7 7M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0",
            "icons/case-sensitive.svg" => "M3 18L8 5l5 13M5 13h6M16 10h5v8h-5v-4h5",
            "icons/regex.svg" => "M5 15h1v1H5zM15 3v12M9 6l12 6M9 12l12-6",
            "icons/whole-word.svg" => "M3 7v12h18V7M7 7v7h4V7M15 7v7h3V7",
            "icons/replace.svg" | "icons/replace-all.svg" => {
                "M4 5h14l-4-4M20 19H6l4 4M6 10h3v4H6zM15 10h3v4h-3z"
            }
            "icons/palette.svg" => {
                "M12 3a9 9 0 1 0 0 18c3 0 0-5 3-5h3c5 0 4-13-6-13zM7 9h1M12 6h1M17 9h1"
            }
            "icons/loader-circle.svg" | "icons/loader.svg" => "M20 12a8 8 0 1 1-8-8",
            _ => return Ok(None),
        };
        Ok(Some(std::borrow::Cow::Owned(format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"{data}\" fill=\"none\" stroke=\"black\" stroke-width=\"1.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/></svg>").into_bytes())))
    }
    fn list(&self, _: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(vec![])
    }
}
