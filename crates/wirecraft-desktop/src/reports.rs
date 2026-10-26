use super::*;
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportState {
    pub documentation: bool,
    pub rasterized: bool,
    pub printer: String,
    pub sheet: wirecraft_core::sheets::SheetOptions,
    pub document: wirecraft_core::documentation::Options,
}
impl Wirecraft {
    pub(super) fn export_arguments(&self) -> Value {
        json!({"nodeId":self.parent,"kind":if self.export_settings.documentation{"documentation"}else{"plan"},"options":if self.export_settings.documentation{serde_json::to_value(&self.export_settings.document).unwrap()}else{serde_json::to_value(&self.export_settings.sheet).unwrap()},"page":self.export_page,"generation":self.export_generation,"rasterized":self.export_settings.rasterized})
    }
    pub(super) fn refresh_preview(&mut self) {
        self.export_generation += 1;
        self.client.send("export.preview", self.export_arguments());
    }
    fn write_report(&mut self, format: &'static str, w: &mut Window, cx: &mut Context<Self>) {
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
                button(format, format.to_uppercase())
                    .on_click(cx.listener(move |this, _, w, cx| this.write_report(format, w, cx)))
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
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .pb_2()
            .child(
                button("printer-file", "Export to file")
                    .when(state.printer.is_empty(), |b| {
                        b.bg(crate::chrome::color(0x255452))
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.export_settings.printer.clear();
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
                    .child(name.clone())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.export_settings.printer = name.clone();
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
                    button("submit-print", "Print all pages").on_click(cx.listener(
                        |this, _, _, _| {
                            let mut arguments = this.export_arguments();
                            arguments["printer"] = json!(this.export_settings.printer);
                            this.client.send("print.submit", arguments);
                        },
                    )),
                )
            });
        div()
            .flex()
            .flex_col()
            .flex_1()
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
            .child(queues)
            .child(exports)
            .child(preview)
            .child(
                div()
                    .flex()
                    .gap_2()
                    .p_3()
                    .items_center()
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
