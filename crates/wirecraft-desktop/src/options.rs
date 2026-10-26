//! Editable settings are staged until Save; persistence is an application service.
use super::*;
use wirecraft_runtime::config::Settings;
pub struct OptionsEditor {
    pub saved: Settings,
    pub fields: Vec<OptionField>,
    pub tab: &'static str,
}
pub struct OptionField {
    pub path: String,
    pub label: String,
    pub input: Entity<InputState>,
    numeric: bool,
}
impl OptionsEditor {
    pub fn new(saved: Settings, w: &mut Window, cx: &mut Context<Wirecraft>) -> Self {
        let value = serde_json::to_value(&saved).unwrap();
        let mut definitions = vec![
            (
                "/appearance/theme".to_string(),
                "Application theme (dark / light)".to_string(),
            ),
            (
                "/appearance/designerFont".into(),
                "Flow Designer font".into(),
            ),
            ("/appearance/editorFont".into(), "Code editor font".into()),
            (
                "/appearance/compositeTitleSize".into(),
                "Composite title target screen size".into(),
            ),
            (
                "/appearance/referenceWidth".into(),
                "Composite reference viewport width".into(),
            ),
            (
                "/appearance/referenceHeight".into(),
                "Composite reference viewport height".into(),
            ),
            (
                "/editorIndentSpaces".into(),
                "Code editor indent spaces".into(),
            ),
            (
                "/appearance/wheelRows".into(),
                "Mouse-wheel scroll rows".into(),
            ),
            ("/clangd/executable".into(), "clangd executable".into()),
            ("/cCompiler".into(), "C compiler executable".into()),
            ("/cppCompiler".into(), "C++ compiler executable".into()),
            (
                "/inference/provider".into(),
                "Active provider (clipboard / openai / anthropic / ollama)".into(),
            ),
            ("/inference/endpoint".into(), "Endpoint URL".into()),
            ("/inference/model".into(), "Model".into()),
            (
                "/inference/tokenEnv".into(),
                "API token environment variable".into(),
            ),
        ];
        definitions.extend(
            saved
                .appearance
                .colors
                .keys()
                .map(|key| (format!("/appearance/colors/{key}"), key.clone())),
        );
        let fields = definitions
            .into_iter()
            .map(|(path, label)| {
                let val = value.pointer(&path).unwrap();
                let numeric = val.is_number();
                let text = val
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| val.to_string());
                let input = cx.new(|cx| {
                    let mut s = InputState::new(w, cx);
                    s.set_value(text, w, cx);
                    s
                });
                OptionField {
                    path,
                    label,
                    input,
                    numeric,
                }
            })
            .collect();
        Self {
            saved,
            fields,
            tab: "General",
        }
    }
    pub fn value(&self, cx: &App) -> Result<Value, String> {
        let mut value = serde_json::to_value(&self.saved).map_err(|e| e.to_string())?;
        for f in &self.fields {
            let text = f.input.read(cx).value().to_string();
            let val = if f.numeric {
                serde_json::from_str::<Value>(&text)
                    .map_err(|_| format!("{} must be a number", f.label))?
            } else {
                json!(text)
            };
            *value
                .pointer_mut(&f.path)
                .ok_or_else(|| format!("Missing option {}", f.path))? = val;
        }
        let settings: Settings =
            serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        settings.validate().map_err(|e| e.to_string())?;
        Ok(value)
    }
}
impl Wirecraft {
    pub(super) fn apply_settings(&mut self, value: Value, w: &mut Window, cx: &mut Context<Self>) {
        if let Ok(settings) = serde_json::from_value::<Settings>(value) {
            self.options = OptionsEditor::new(settings.clone(), w, cx);
            Theme::change(
                if settings.appearance.theme == "light" {
                    ThemeMode::Light
                } else {
                    ThemeMode::Dark
                },
                Some(w),
                cx,
            );
            let text = self.editor.read(cx).value();
            let position = self.editor.read(cx).cursor_position();
            let language = self
                .selected
                .first()
                .map(|id| {
                    self.snapshot
                        .document
                        .effective_text_language(id, &self.section)
                })
                .unwrap_or_default();
            self.editor = cx.new(|cx| {
                let mut s = InputState::new(w, cx)
                    .code_editor(language)
                    .tab_size(TabSize {
                        tab_size: settings.editor_indent_spaces,
                        hard_tabs: false,
                    });
                s.set_value(text, w, cx);
                s.set_cursor_position(position, w, cx);
                s
            });
            cx.subscribe(&self.editor, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.persist_ui(cx);
                }
            })
            .detach();
            Theme::global_mut(cx).mono_font_family = settings.appearance.editor_font.clone().into();
            self.rebuild();
            cx.notify();
        }
    }
    pub(super) fn render_options(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut tabs = div().flex().gap_2().p_3();
        for label in ["General", "Flow Designer colors", "Inference Providers"] {
            tabs = tabs.child(
                button(label, label)
                    .when(self.options.tab == label, |b| b.bg(rgb(0x255452)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.options.tab = label;
                        cx.notify();
                    })),
            );
        }
        let mut form = div()
            .id("options-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_4()
            .flex()
            .flex_col()
            .gap_3();
        for f in &self.options.fields {
            let colors = f.path.starts_with("/appearance/colors/");
            let inference = f.path.starts_with("/inference/");
            if (self.options.tab == "General" && (colors || inference))
                || (self.options.tab == "Flow Designer colors" && !colors)
                || (self.options.tab == "Inference Providers" && !inference)
            {
                continue;
            }
            form = form.child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(div().w(px(310.)).flex_none().child(f.label.clone()))
                    .child(div().w(px(380.)).child(Input::new(&f.input))),
            );
        }
        if self.options.tab == "Inference Providers" {
            form=form.child(div().p_3().max_w(px(720.)).text_color(rgb(0x92a8b8)).child("Clipboard is the default. Network providers run only when an AI command is invoked. Store the token in an environment variable and enter its name here; Wirecraft does not import Threadworks credentials. Browser authorization was unimplemented in the reference application."));
        }
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(div().p_4().text_lg().child("Options"))
            .child(tabs)
            .child(form)
            .child(
                div()
                    .p_3()
                    .flex()
                    .gap_2()
                    .child(button("save-options", "Save options").on_click(cx.listener(
                        |this, _, _, cx| match this.options.value(cx) {
                            Ok(value) => this.client.send("settings.replace", value),
                            Err(e) => {
                                this.status = e;
                                cx.notify();
                            }
                        },
                    )))
                    .child(
                        button("cancel-options", "Cancel changes").on_click(cx.listener(
                            |this, _, w, cx| {
                                let saved = this.options.saved.clone();
                                this.apply_settings(serde_json::to_value(saved).unwrap(), w, cx);
                            },
                        )),
                    )
                    .child(
                        button("reset-options", "Restore defaults").on_click(cx.listener(
                            |this, _, w, cx| {
                                this.options = OptionsEditor::new(Settings::default(), w, cx);
                                cx.notify();
                            },
                        )),
                    ),
            )
            .into_any_element()
    }
    pub(super) fn render_mcp(&self, cx: &mut Context<Self>) -> AnyElement {
        let config = json!({"mcpServers":{"wirecraft":{"url":format!("{}/mcp",self.endpoint),"headers":{"Authorization":format!("Bearer {}",self.token)}}}});
        let mut log = div()
            .id("mcp-access-log")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_3()
            .bg(rgb(0x10161e));
        for (i, row) in self.mcp_status["log"]
            .as_array()
            .into_iter()
            .flatten()
            .rev()
            .enumerate()
        {
            log = log.child(
                div()
                    .id(i)
                    .text_xs()
                    .font_family("DejaVu Sans Mono")
                    .child(format!(
                        "{}  {} → {}  {}ms",
                        row["at"].as_str().unwrap_or(""),
                        row["method"].as_str().unwrap_or(""),
                        row["status"],
                        row["elapsedMs"]
                    )),
            );
        }
        div().flex().flex_col().flex_1().min_h_0().p_4().gap_3().child(div().text_lg().text_color(rgb(0x70d5c6)).child(if self.mcp_status["enabled"]==false{"MCP SERVER: OFF"}else{"MCP SERVER: ON"}))
            .child(div().child(format!("MCP endpoint: {}/mcp",self.endpoint))).child(div().child(format!("Health endpoint: {}/health",self.endpoint)))
            .child(div().flex().gap_2().child(button("start-mcp","Start").on_click(cx.listener(|this,_,_,_|this.client.send("mcp.set_enabled",json!({"enabled":true}))))).child(button("stop-mcp","Stop").on_click(cx.listener(|this,_,_,_|this.client.send("mcp.set_enabled",json!({"enabled":false}))))).child(button("health-mcp","Check health / Refresh log").on_click(cx.listener(|this,_,_,_|{this.tool("system.health");this.tool("mcp.status");}))).child(button("copy-mcp","Copy client configuration").on_click(move|_,_,cx|cx.write_to_clipboard(ClipboardItem::new_string(serde_json::to_string_pretty(&config).unwrap())))))
            .child(div().text_color(rgb(0x91a5b7)).child("HTTP access log · latest 256 requests · bodies and credentials are never logged")).child(log)
            .child(div().text_xs().child("Stop disables MCP requests while the desktop remains connected to its backend. Closing this desktop stops its owned backend and tools. The original Threadworks server runs independently.")).into_any_element()
    }
}
