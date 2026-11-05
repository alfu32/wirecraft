//! Home screen: new projects by technology, Open, recent files and examples. It opens when
//! the application starts without a file, and from the title bar's home button.
use super::*;
use gpui_component::scroll::ScrollableElement;
use wirecraft_core::thumbnail::{Thumbnail, ThumbnailShape};

const RECENT_LIMIT: usize = 12;
/// Technologies a new project can start with: title, description, language, technology.
const NEW_PROJECTS: [(&str, &str, &str, &str); 7] = [
    ("C17", "A native C program", "c", "c-native"),
    ("Rust", "A Cargo project", "rust", "rust"),
    ("Go", "A Go module", "go", "go"),
    (
        "Kotlin",
        "A Kotlin project for the JVM",
        "kotlin",
        "kotlin-jvm",
    ),
    ("Node.js", "JavaScript on Node.js", "javascript", "nodejs"),
    ("PHP", "A PHP project", "php", "php"),
    (
        "systemJS",
        "JavaScript modules, run on Bun",
        "javascript",
        "systemjs",
    ),
];
struct RecentEntry {
    path: String,
    name: String,
    exists: bool,
    seconds_ago: Option<u64>,
    thumbnail: Option<Arc<Thumbnail>>,
}
struct ExampleEntry {
    id: String,
    title: String,
    description: String,
    thumbnail: Option<Arc<Thumbnail>>,
}
pub(super) struct Home {
    pub(super) open: bool,
    settings: bool,
    pub(super) recent: Vec<String>,
    /// The project path last considered for the recent list.
    noted: Option<PathBuf>,
    examples_dir: PathBuf,
    recent_entries: Vec<RecentEntry>,
    examples: Vec<ExampleEntry>,
}
impl Home {
    pub(super) fn new(open: bool, examples_dir: PathBuf) -> Self {
        Self {
            open,
            settings: false,
            recent: vec![],
            noted: None,
            examples_dir,
            recent_entries: vec![],
            examples: vec![],
        }
    }
    pub(super) fn receive(&mut self, value: &Value) {
        let thumbnail = |entry: &Value| {
            serde_json::from_value::<Thumbnail>(entry["thumbnail"].clone())
                .ok()
                .map(Arc::new)
        };
        let text = |entry: &Value, key: &str| entry[key].as_str().unwrap_or("").to_string();
        let list = |key: &str| value[key].as_array().cloned().unwrap_or_default();
        self.recent_entries = list("recent")
            .iter()
            .map(|e| RecentEntry {
                path: text(e, "path"),
                name: text(e, "name"),
                exists: e["exists"] == true,
                seconds_ago: e["secondsAgo"].as_u64(),
                thumbnail: thumbnail(e),
            })
            .collect();
        self.examples = list("examples")
            .iter()
            .map(|e| ExampleEntry {
                id: text(e, "id"),
                title: text(e, "title"),
                description: text(e, "description"),
                thumbnail: thumbnail(e),
            })
            .collect();
    }
}
/// Human-readable file age for the recent list.
fn age_text(seconds: u64) -> String {
    let plural =
        |value: u64, unit: &str| format!("{value} {unit}{} ago", if value == 1 { "" } else { "s" });
    match seconds {
        0..60 => "Just now".into(),
        60..3600 => plural(seconds / 60, "minute"),
        3600..86_400 => plural(seconds / 3600, "hour"),
        86_400..2_592_000 => plural(seconds / 86_400, "day"),
        2_592_000..31_536_000 => plural(seconds / 2_592_000, "month"),
        _ => plural(seconds / 31_536_000, "year"),
    }
}
fn paint_thumbnail(t: &Thumbnail, b: Bounds<Pixels>, window: &mut Window) {
    const MARGIN: f32 = 6.;
    if t.width <= 0. || t.height <= 0. {
        return;
    }
    let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
    let scale = ((w - 2. * MARGIN) / t.width as f32)
        .min((h - 2. * MARGIN) / t.height as f32)
        .min(0.6);
    let x = f32::from(b.origin.x) + (w - t.width as f32 * scale) / 2.;
    let y = f32::from(b.origin.y) + (h - t.height as f32 * scale) / 2.;
    let at = |p: [f64; 2]| point(px(x + p[0] as f32 * scale), px(y + p[1] as f32 * scale));
    for shape in &t.boxes {
        let (fill, border) = match shape.shape {
            ThumbnailShape::Composite => (0x1c242e, 0x40505c),
            ThumbnailShape::Entity => (0x253f45, 0x70d5c6),
            ThumbnailShape::Type => (0x2d3b48, 0x9bb0c3),
            ThumbnailShape::Port => (0x16303f, 0x75d4d6),
        };
        let size = size(
            px(shape.rect[2] as f32 * scale),
            px(shape.rect[3] as f32 * scale),
        );
        window.paint_quad(quad(
            Bounds::new(at([shape.rect[0], shape.rect[1]]), size),
            if shape.shape == ThumbnailShape::Port {
                size.height / 2.
            } else {
                px(1.)
            },
            crate::chrome::color(fill),
            px(1.),
            crate::chrome::color(border),
            BorderStyle::Solid,
        ));
    }
    for line in &t.links {
        let mut path = PathBuilder::stroke(px(1.));
        path.move_to(at(line[0]));
        for p in &line[1..] {
            path.line_to(at(*p));
        }
        if let Ok(path) = path.build() {
            window.paint_path(path, crate::chrome::color(0x7f9bae));
        }
    }
}
fn thumbnail(picture: Option<Arc<Thumbnail>>, width: f32, height: f32) -> Div {
    div()
        .flex_none()
        .w(px(width))
        .h(px(height))
        .rounded_sm()
        .overflow_hidden()
        .bg(crate::chrome::color(0x10161e))
        .child(
            canvas(
                |b, _, _| b,
                move |b, _, window, _| {
                    if let Some(picture) = &picture {
                        paint_thumbnail(picture, b, window);
                    }
                },
            )
            .size_full(),
        )
}
fn card() -> Div {
    div()
        .flex_none()
        .rounded_md()
        .border_1()
        .border_color(crate::chrome::color(0x34414d))
        .bg(crate::chrome::color(0x1c242e))
}
fn heading(text: &'static str) -> Div {
    div().flex_none().mt_6().mb_3().text_lg().child(text)
}
fn side_item(
    id: &'static str,
    label: String,
    shortcut: &'static str,
    active: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_3()
        .py_2()
        .rounded_sm()
        .cursor_pointer()
        .when(active, |b| b.bg(crate::chrome::color(0x255452)))
        .hover(|s| s.bg(crate::chrome::color(0x2d3b48)))
        .child(div().min_w_0().truncate().child(label))
        .child(
            div()
                .flex_none()
                .text_xs()
                .text_color(crate::chrome::color(0x8fa6b5))
                .child(shortcut),
        )
}
fn rule() -> Div {
    div()
        .flex_none()
        .h(px(1.))
        .mx_3()
        .my_2()
        .bg(crate::chrome::color(0x34414d))
}
impl Wirecraft {
    pub(super) fn refresh_home(&mut self) {
        self.client
            .send("home.overview", json!({"recent":self.home.recent}));
    }
    pub(super) fn toggle_home(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        self.home.open = !self.home.open;
        self.home.settings = false;
        if self.home.open {
            self.refresh_home();
        }
        self.focus.focus(w);
        cx.notify();
    }
    pub(super) fn show_home_settings(&mut self) -> bool {
        self.home.settings = self.home.open;
        self.home.open
    }
    pub(super) fn leave_home(&mut self) {
        self.home.open = false;
        self.home.settings = false;
    }
    /// Keeps the recent list current, however the project got its path. A project that has
    /// never been saved has no file to list yet, and an example's copy is listed as an example.
    pub(super) fn note_recent(&mut self, cx: &App) {
        let path = self
            .snapshot
            .path
            .as_ref()
            .and_then(|p| std::path::absolute(p).ok());
        if path == self.home.noted {
            return;
        }
        let Some(path) = path else {
            self.home.noted = None;
            return;
        };
        if !path.exists() {
            return;
        }
        self.home.noted = Some(path.clone());
        if path.starts_with(&self.home.examples_dir) {
            return;
        }
        let path = path.to_string_lossy().into_owned();
        self.home.recent.retain(|p| *p != path);
        self.home.recent.insert(0, path);
        self.home.recent.truncate(RECENT_LIMIT);
        self.persist_ui(cx);
        if self.home.open {
            self.refresh_home();
        }
    }
    fn open_recent(&mut self, path: String, w: &mut Window, cx: &mut Context<Self>) {
        let current = self
            .snapshot
            .path
            .as_ref()
            .and_then(|p| std::path::absolute(p).ok());
        if current.as_deref() == Some(std::path::Path::new(&path)) {
            self.leave_home();
            cx.notify();
            return;
        }
        self.replace_document(
            "file.open",
            json!({"path":path}),
            "Open another design?",
            "Discard and open",
            w,
            cx,
        );
    }
    fn render_home_side(&self, cx: &mut Context<Self>) -> Div {
        let back = format!("‹ Back to {}", self.snapshot.document.project_name());
        div()
            .flex_none()
            .w(px(250.))
            .h_full()
            .flex()
            .flex_col()
            .gap(px(2.))
            .p_3()
            .bg(crate::chrome::color(0x171d25))
            .border_r_1()
            .border_color(crate::chrome::color(0x29323d))
            .child(
                side_item("home-back", back, "Esc", false)
                    .mb_3()
                    .on_click(cx.listener(|this, _, w, cx| this.toggle_home(w, cx))),
            )
            .child(
                side_item("home-start", "Start".into(), "", !self.home.settings).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.home.settings = false;
                        cx.notify();
                    }),
                ),
            )
            .child(
                side_item("home-new", "New".into(), "Ctrl+N", false)
                    .on_click(cx.listener(|this, _, w, cx| this.new_project(&New, w, cx))),
            )
            .child(
                side_item("home-open", "Open".into(), "Ctrl+O", false)
                    .on_click(cx.listener(|this, _, w, cx| this.open(&Open, w, cx))),
            )
            .child(rule())
            .child(
                side_item("home-save", "Save".into(), "Ctrl+S", false)
                    .on_click(cx.listener(|this, _, w, cx| this.save(&Save, w, cx))),
            )
            .child(
                side_item("home-save-as", "Save As".into(), "Ctrl+Shift+S", false)
                    .on_click(cx.listener(|this, _, w, cx| this.save_as(&SaveAs, w, cx))),
            )
            .child(rule())
            .child(
                side_item(
                    "home-settings",
                    "Settings".into(),
                    "Ctrl+,",
                    self.home.settings,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.home.settings = true;
                    cx.notify();
                })),
            )
    }
    fn render_home_start(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = crate::chrome::color(0x8fa6b5);
        let tile = |id: SharedString, icon: &'static str, title: &str, description: &str| {
            card()
                .id(id)
                .w(px(250.))
                .h(px(72.))
                .flex()
                .items_center()
                .gap_3()
                .px_4()
                .cursor_pointer()
                .hover(|s| s.bg(crate::chrome::color(0x2d3b48)))
                .child(icons::icon(icon))
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(title.to_string())
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child(description.to_string()),
                        ),
                )
        };
        let mut new = div().flex_none().flex().flex_wrap().gap_3();
        for (title, description, language, technology) in NEW_PROJECTS {
            new = new.child(
                tile(
                    format!("home-new-{technology}").into(),
                    "code",
                    title,
                    description,
                )
                .on_click(cx.listener(move |this, _, w, cx| {
                    this.replace_document(
                        "file.new",
                        json!({"technology":{"languageId":language,"technologyId":technology}}),
                        "Create a new design?",
                        "Discard and create",
                        w,
                        cx,
                    );
                })),
            );
        }
        new = new.child(
            tile(
                "home-open-card".into(),
                "open",
                "Open…",
                "An existing .orch project",
            )
            .on_click(cx.listener(|this, _, w, cx| this.open(&Open, w, cx))),
        );
        let mut recent = div().flex_none().flex().flex_col().gap_2();
        if self.home.recent_entries.is_empty() {
            recent = recent.child(
                div()
                    .text_color(muted)
                    .child("Projects you open or save will appear here."),
            );
        }
        for entry in &self.home.recent_entries {
            let (open_path, remove_path) = (entry.path.clone(), entry.path.clone());
            let exists = entry.exists;
            recent = recent.child(
                card()
                    .id(SharedString::from(format!("home-recent-{}", entry.path)))
                    .flex()
                    .items_center()
                    .gap_4()
                    .p_2()
                    .when(exists, |row| {
                        row.cursor_pointer()
                            .hover(|s| s.bg(crate::chrome::color(0x2d3b48)))
                            .on_click(cx.listener(move |this, _, w, cx| {
                                this.open_recent(open_path.clone(), w, cx)
                            }))
                    })
                    .when(!exists, |row| row.opacity(0.55))
                    .child(thumbnail(entry.thumbnail.clone(), 64., 44.))
                    .child(
                        div()
                            .flex_none()
                            .w(px(230.))
                            .truncate()
                            .child(entry.name.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(muted)
                            .child(entry.path.clone()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .w(px(110.))
                            .text_xs()
                            .text_color(muted)
                            .child(if exists {
                                entry.seconds_ago.map_or(String::new(), age_text)
                            } else {
                                "File not found".into()
                            }),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("home-remove-{}", entry.path)))
                            .flex_none()
                            .p_1()
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|s| s.bg(crate::chrome::color(0x34414d)))
                            .tooltip(|window, cx| {
                                gpui_component::tooltip::Tooltip::new("Remove from this list")
                                    .build(window, cx)
                            })
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.home.recent.retain(|p| *p != remove_path);
                                this.home.recent_entries.retain(|e| e.path != remove_path);
                                this.persist_ui(cx);
                                cx.notify();
                            }))
                            .child(icons::icon("close")),
                    ),
            );
        }
        let mut examples = div().flex_none().flex().flex_wrap().gap_3();
        for example in &self.home.examples {
            let id = example.id.clone();
            examples = examples.child(
                card()
                    .id(SharedString::from(format!("home-example-{}", example.id)))
                    .w(px(222.))
                    .h(px(228.))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_2()
                    .cursor_pointer()
                    .hover(|s| s.bg(crate::chrome::color(0x2d3b48)))
                    .on_click(cx.listener(move |this, _, w, cx| {
                        let directory = this.home.examples_dir.clone();
                        this.replace_document(
                            "examples.open",
                            json!({"id":id,"directory":directory}),
                            "Open this example?",
                            "Discard and open",
                            w,
                            cx,
                        );
                    }))
                    .child(thumbnail(example.thumbnail.clone(), 204., 127.))
                    .child(example.title.clone())
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .child(example.description.clone()),
                    ),
            );
        }
        div()
            .id("home-content")
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scrollbar()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .px(px(40.))
                    .py_6()
                    .child(div().flex_none().text_2xl().child("Start"))
                    .child(
                        div()
                            .flex_none()
                            .mt_2()
                            .text_color(muted)
                            .child("Create a project, open a recent file, or explore an example."),
                    )
                    .child(heading("New"))
                    .child(new)
                    .child(heading("Recent"))
                    .child(recent)
                    .child(heading("Examples"))
                    .child(examples),
            )
            .into_any_element()
    }
    pub(super) fn render_home(&self, cx: &mut Context<Self>) -> AnyElement {
        let side = self.render_home_side(cx);
        div()
            .flex()
            .flex_1()
            .min_h_0()
            .child(side)
            .child(if self.home.settings {
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(self.render_options(cx))
                    .into_any_element()
            } else {
                self.render_home_start(cx)
            })
            .into_any_element()
    }
}
