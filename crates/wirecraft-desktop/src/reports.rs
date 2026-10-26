use super::*;
use std::collections::BTreeMap;
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportState {
    pub documentation: bool,
    pub rasterized: bool,
    pub printer: String,
    pub sheet: wirecraft_core::sheets::SheetOptions,
    pub document: wirecraft_core::documentation::Options,
    pub printer_profiles: BTreeMap<String, PrintProfile>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct PrintProfile {
    sheet: wirecraft_core::sheets::SheetOptions,
    document: wirecraft_core::documentation::Options,
    rasterized: bool,
}
impl Wirecraft {
    fn switch_printer(&mut self, name: String) {
        let state = &mut self.export_settings;
        state.printer_profiles.insert(
            state.printer.clone(),
            PrintProfile {
                sheet: state.sheet.clone(),
                document: state.document.clone(),
                rasterized: state.rasterized,
            },
        );
        let profile = state
            .printer_profiles
            .get(&name)
            .cloned()
            .unwrap_or_default();
        state.sheet = profile.sheet;
        state.document = profile.document;
        state.rasterized = profile.rasterized;
        state.printer = name;
        self.export_page = 0;
        self.refresh_preview();
    }

    pub(super) fn export_arguments(&self) -> Value {
        json!({"nodeId":self.parent,"kind":if self.export_settings.documentation{"documentation"}else{"plan"},"options":if self.export_settings.documentation{serde_json::to_value(&self.export_settings.document).unwrap()}else{serde_json::to_value(&self.export_settings.sheet).unwrap()},"page":self.export_page,"generation":self.export_generation,"rasterized":self.export_settings.rasterized,"sheetOptions":self.export_settings.sheet,"documentOptions":self.export_settings.document})
    }
    pub(super) fn refresh_preview(&mut self) {
        self.export_generation += 1;
        self.client.send("export.preview", self.export_arguments());
    }
    fn write_report(
        &mut self,
        format: &'static str,
        all: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let directory = self
            .snapshot
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let prompt = cx.prompt_for_new_path(
            &directory,
            Some(&format!(
                "{}-{}.{}",
                self.snapshot.document.project_name(),
                if self.export_settings.documentation {
                    "specification"
                } else {
                    "plan"
                },
                if format == "markdown" { "md" } else { format }
            )),
        );
        let mut arguments = self.export_arguments();
        arguments["format"] = json!(format);
        if all {
            arguments["kind"] = json!("all");
        }
        cx.spawn_in(w, async move |this, cx| {
            if let Ok(Ok(Some(path))) = prompt.await {
                arguments["path"] = json!(path);
                let _ =
                    this.update_in(cx, |this, _, _| this.client.send("export.write", arguments));
            }
        })
        .detach();
    }
    pub(super) fn render_export(&self, cx: &mut Context<Self>) -> AnyElement {
        let state = &self.export_settings;
        let mut controls = div().flex().flex_wrap().items_center().gap_2().p_3();
        let doc = state.documentation;
        let format = if doc {
            &state.document.format
        } else {
            &state.sheet.format
        };
        controls = controls.child(
            button("sheet-format", format!("Format: {format} ▾")).on_click(cx.listener(
                |this, _, _, cx| {
                    this.menu = Some(
                        if this.export_settings.documentation {
                            "Document format"
                        } else {
                            "Sheet format"
                        }
                        .into(),
                    );
                    cx.notify();
                },
            )),
        );
        if !doc {
            controls = controls
                .child(
                    button("sheet-scale", format!("Scale: 1:{} ▾", state.sheet.scale)).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.menu = Some("Sheet scale".into());
                            cx.notify();
                        }),
                    ),
                )
                .child(
                    button(
                        "sheet-split",
                        if state.sheet.split {
                            "☑ Split fixed sheets across pages"
                        } else {
                            "☐ Split fixed sheets across pages"
                        },
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.export_settings.sheet.split = !this.export_settings.sheet.split;
                        this.export_page = 0;
                        this.refresh_preview();
                        this.persist_ui(cx);
                        cx.notify();
                    })),
                )
                .child(div().child(format!("Overlap: {} mm", state.sheet.overlap_mm)))
                .child(
                    button("overlap-less", "−").on_click(cx.listener(|this, _, _, cx| {
                        this.export_settings.sheet.overlap_mm =
                            (this.export_settings.sheet.overlap_mm - 1.).max(0.);
                        this.refresh_preview();
                        this.persist_ui(cx);
                    })),
                )
                .child(
                    button("overlap-more", "+").on_click(cx.listener(|this, _, _, cx| {
                        this.export_settings.sheet.overlap_mm =
                            (this.export_settings.sheet.overlap_mm + 1.).min(100.);
                        this.refresh_preview();
                        this.persist_ui(cx);
                    })),
                );
        } else {
            controls = controls
                .child(
                    button(
                        "document-header",
                        if state.document.header {
                            "☑ Header"
                        } else {
                            "☐ Header"
                        },
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.export_settings.document.header =
                            !this.export_settings.document.header;
                        this.refresh_preview();
                        this.persist_ui(cx);
                    })),
                )
                .child(
                    button(
                        "document-footer",
                        if state.document.footer {
                            "☑ Footer"
                        } else {
                            "☐ Footer"
                        },
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.export_settings.document.footer =
                            !this.export_settings.document.footer;
                        this.refresh_preview();
                        this.persist_ui(cx);
                    })),
                );
            for (key, label, value) in [
                ("top", "Top", state.document.top_mm),
                ("left", "Left", state.document.left_mm),
                ("right", "Right", state.document.right_mm),
                ("bottom", "Bottom", state.document.bottom_mm),
            ] {
                controls =
                    controls.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(format!("{label}: {value}mm"))
                            .child(button(key, "−").on_click(
                                cx.listener(move |this, _, _, cx| this.margin(key, -1., cx)),
                            ))
                            .child(button(label, "+").on_click(
                                cx.listener(move |this, _, _, cx| this.margin(key, 1., cx)),
                            )),
                    );
            }
        }
        let formats = if doc {
            vec!["pdf", "html", "markdown"]
        } else {
            vec!["pdf", "svg", "png", "mermaid"]
        };
        controls = controls.child(
            button(
                "pdf-mode",
                if state.rasterized {
                    "PDF: Rasterized"
                } else {
                    "PDF: Searchable vector"
                },
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.export_settings.rasterized = !this.export_settings.rasterized;
                this.persist_ui(cx);
                cx.notify();
            })),
        );
        let exports = div()
            .flex()
            .gap_2()
            .px_3()
            .pb_3()
            .children(formats.into_iter().map(|format| {
                button(format, format.to_uppercase()).on_click(
                    cx.listener(move |this, _, w, cx| this.write_report(format, false, w, cx)),
                )
            }));
        let mut preview = div()
            .id("sheet-preview")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_4()
            .bg(crate::chrome::color(0x303943))
            .flex()
            .justify_center();
        if let Some(image) = &self.export_image {
            preview = preview.child(
                img(image.clone())
                    .id("export-preview-image")
                    .w(px(900.))
                    .h(px(700.))
                    .flex_none()
                    .max_w(px(1000.))
                    .object_fit(ObjectFit::Contain)
                    .with_loading(||div().p_4().child("Rendering preview…").into_any_element())
                    .with_fallback(||div().p_4().child("Preview decoding failed. Export remains available; see application log.").into_any_element()),
            );
        } else {
            preview = preview.child("Choose settings and refresh the preview.");
        }
        let queues = div()
            .w(px(235.)).flex_none().flex().flex_col().gap_3().p_3()
            .bg(crate::chrome::color(0x191f28)).border_r_1().border_color(crate::chrome::color(0x29323d))
            .child(div().text_color(crate::chrome::color(0x70d5c6)).child("PRINTERS"))
            .child(
                button("printer-file", "Export to file")
                    .when(state.printer.is_empty(), |b| {
                        b.bg(crate::chrome::color(0x255452))
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.switch_printer(String::new());
                        this.persist_ui(cx);
                        cx.notify();
                    })),
            )
            .children(self.printers.iter().enumerate().map(|(i, p)| {
                let name = p["name"].as_str().unwrap_or("").to_string();
                div()
                    .id(("printer", i))
                    .p_2()
                    .rounded_sm()
                    .bg(crate::chrome::color(if state.printer == name {
                        0x255452
                    } else {
                        0x242c36
                    }))
                    .cursor_pointer()
                    .whitespace_nowrap().text_ellipsis().overflow_hidden().child(name.clone())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.switch_printer(name.clone());
                        this.persist_ui(cx);
                        cx.notify();
                    }))
            }))
            .child(
                button("refresh-printers", "Refresh printers")
                    .on_click(cx.listener(|this, _, _, _| this.tool("printers.list"))),
            )
            .when(!state.printer.is_empty(), |b| {
                b.child(
                    button("submit-print", "Print current section").on_click(cx.listener(
                        |this, _, _, _| {
                            let mut arguments = this.export_arguments();
                            arguments["printer"] = json!(this.export_settings.printer);
                            this.client.send("print.submit", arguments);
                        },
                    )),
                )
            })
            .when(!state.printer.is_empty(),|b|b.child(button("print-all","Print plan + documentation").on_click(cx.listener(|this,_,_,_|{let mut arguments=this.export_arguments();arguments["printer"]=json!(this.export_settings.printer);arguments["kind"]=json!("all");this.client.send("print.submit",arguments);}))))
            .child(div().flex_1()).child(div().text_xs().text_color(crate::chrome::color(0x91a5b7)).child("File export is always available. Print submits only to the selected CUPS queue."));
        let body =
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .p_3()
                        .child(
                            button("plan-tab", "Plan")
                                .when(!doc, |b| b.bg(crate::chrome::color(0x255452)))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.export_settings.documentation = false;
                                    this.export_page = 0;
                                    this.refresh_preview();
                                    this.persist_ui(cx);
                                    cx.notify();
                                })),
                        )
                        .child(
                            button("documentation-tab", "Documentation")
                                .when(doc, |b| b.bg(crate::chrome::color(0x255452)))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.export_settings.documentation = true;
                                    this.export_page = 0;
                                    this.refresh_preview();
                                    this.persist_ui(cx);
                                    cx.notify();
                                })),
                        )
                        .child(div().flex_1())
                        .child(
                            button("refresh-preview", "Refresh preview")
                                .on_click(cx.listener(|this, _, _, _| this.refresh_preview())),
                        ),
                )
                .child(controls)
                .child(exports)
                .child(preview)
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .p_3()
                        .items_center()
                        .flex_wrap()
                        .child(
                            button("pdf-all", "PDF All · Plan + Documentation").on_click(
                                cx.listener(|this, _, w, cx| this.write_report("pdf", true, w, cx)),
                            ),
                        )
                        .child(button("reset-print-settings", "Reset settings").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.export_settings.sheet = Default::default();
                                this.export_settings.document = Default::default();
                                this.export_settings.rasterized = false;
                                this.export_page = 0;
                                this.refresh_preview();
                                this.persist_ui(cx);
                                cx.notify();
                            }),
                        ))
                        .child(
                            button("remove-print-settings", "Remove stored settings").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.export_settings
                                        .printer_profiles
                                        .remove(&this.export_settings.printer);
                                    this.export_settings.sheet = Default::default();
                                    this.export_settings.document = Default::default();
                                    this.refresh_preview();
                                    this.persist_ui(cx);
                                    cx.notify();
                                }),
                            ),
                        )
                        .child(
                            button("previous-page", "Previous page").on_click(cx.listener(
                                |this, _, _, _| {
                                    if this.export_page > 0 {
                                        this.export_page -= 1;
                                        this.refresh_preview();
                                    }
                                },
                            )),
                        )
                        .child(div().child(format!(
                            "Page {} / {} · {}",
                            self.export_page + 1,
                            self.export_pages,
                            self.export_summary
                        )))
                        .child(button("next-page", "Next page").on_click(cx.listener(
                            |this, _, _, _| {
                                if this.export_page + 1 < this.export_pages {
                                    this.export_page += 1;
                                    this.refresh_preview();
                                }
                            },
                        ))),
                )
                .into_any_element();
        div()
            .flex()
            .flex_1()
            .min_h_0()
            .child(queues)
            .child(body)
            .into_any_element()
    }
    fn margin(&mut self, key: &str, delta: f64, cx: &mut Context<Self>) {
        let options = &mut self.export_settings.document;
        let value = match key {
            "top" => &mut options.top_mm,
            "left" => &mut options.left_mm,
            "right" => &mut options.right_mm,
            _ => &mut options.bottom_mm,
        };
        *value = (*value + delta).clamp(0., 100.);
        self.refresh_preview();
        self.persist_ui(cx);
    }
}

pub(super) fn preview_summary(value: &Value) -> String {
    if value["kind"] == "documentation" {
        return "Documentation".into();
    }
    if value["kind"] == "all" {
        return format!(
            "{} plan pages · {} documentation pages",
            value["planPages"].as_u64().unwrap_or(0),
            value["documentationPages"].as_u64().unwrap_or(0)
        );
    }
    format!(
        "{} · {:.0} × {:.0} mm · {} × {} sheets{}",
        value["format"].as_str().unwrap_or("Plan"),
        value["widthMm"].as_f64().unwrap_or(0.),
        value["heightMm"].as_f64().unwrap_or(0.),
        value["columns"].as_u64().unwrap_or(1),
        value["rows"].as_u64().unwrap_or(1),
        if value["clipped"] == true {
            " · Drawing clipped"
        } else {
            ""
        }
    )
}
