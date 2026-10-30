//! Compact value controls shared by inspector rows and inline tables.
use super::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Selectable, Sizable,
    menu::{DropdownMenu, PopupMenuItem},
};
use std::collections::HashMap;
/// Last laid-out width of each choice trigger, by its input: the menu opens exactly as
/// wide as the trigger (the menu is built before the trigger's first paint after opening).
#[derive(Default)]
struct ChoiceWidths(HashMap<EntityId, Pixels>);
impl Global for ChoiceWidths {}
/// Dropdown trigger with its value aligned left and the caret at the right edge.
#[derive(IntoElement)]
struct ChoiceTrigger {
    base: Stateful<Div>,
    input: EntityId,
    label: String,
    selected: bool,
}
impl Styled for ChoiceTrigger {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}
impl InteractiveElement for ChoiceTrigger {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}
impl Selectable for ChoiceTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    fn is_selected(&self) -> bool {
        self.selected
    }
}
impl DropdownMenu for ChoiceTrigger {}
impl RenderOnce for ChoiceTrigger {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let input = self.input;
        let theme = cx.theme();
        self.base
            .relative()
            .h_6()
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.input)
            .text_sm()
            .cursor_default()
            .when(self.selected, |b| b.bg(theme.secondary_active))
            .when(!self.selected, |b| b.hover(|b| b.bg(theme.secondary_hover)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(self.label),
            )
            .child(Icon::new(IconName::ChevronDown).small())
            .child(
                canvas(
                    move |bounds, _, cx| {
                        cx.default_global::<ChoiceWidths>()
                            .0
                            .insert(input, bounds.size.width);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
    }
}
pub(super) fn choices(
    input: &Entity<InputState>,
    mut values: Vec<(String, String)>,
    empty: &str,
    cx: &App,
) -> AnyElement {
    let current = input.read(cx).value().to_string();
    if !values.iter().any(|(value, _)| value == &current) {
        values.push((current.clone(), current.clone()));
    }
    let label = values
        .iter()
        .find(|(v, _)| v == &current)
        .map(|(_, l)| l.as_str())
        .filter(|v| !v.is_empty())
        .unwrap_or(empty)
        .to_string();
    let input = input.clone();
    let id = input.entity_id();
    ChoiceTrigger {
        base: div().id(("choice", id)),
        input: id,
        label,
        selected: false,
    }
    .w_full()
    .dropdown_menu_with_anchor(Corner::TopLeft, move |menu, _, cx| {
        // The item list sits inside the menu's one-pixel border.
        let menu = match cx.try_global::<ChoiceWidths>().and_then(|w| w.0.get(&id)) {
            Some(&width) => menu.min_w(width - px(2.)).max_w(width - px(2.)),
            None => menu,
        };
        values.iter().fold(menu, |menu, (value, label)| {
            let value = value.clone();
            let input = input.clone();
            menu.item(
                PopupMenuItem::new(if label.is_empty() {
                    "None".to_string()
                } else {
                    label.clone()
                })
                .checked(value == current)
                .on_click(move |_, w, cx| {
                    input.update(cx, |s, cx| s.set_value(value.clone(), w, cx));
                }),
            )
        })
    })
    .into_any_element()
}
pub(super) fn list(values: &[&str], empty: &str) -> Vec<(String, String)> {
    std::iter::once((String::new(), empty.to_string()))
        .chain(values.iter().map(|s| (s.to_string(), s.to_string())))
        .collect()
}
pub(super) fn types(d: &Document) -> Vec<(String, String)> {
    let mut values = [
        "string", "boolean", "number", "date", "int", "float", "double", "char *", "void *",
    ]
    .map(|s| (s.to_string(), s.to_string()))
    .to_vec();
    values.extend(
        d.nodes
            .values()
            .filter(|n| n.kind == wirecraft_core::NodeKind::Type)
            .map(|n| (n.id.clone(), n.name.clone())),
    );
    values
}
pub(super) fn language(input: &Entity<InputState>, cx: &App) -> AnyElement {
    choices(
        input,
        list(
            &[
                "c",
                "cpp",
                "go",
                "kotlin",
                "javascript",
                "php",
                "python",
                "rust",
                "shellscript",
                "markdown",
                "json",
                "xml",
                "html",
                "plain",
            ],
            "Inherited",
        ),
        "Inherited",
        cx,
    )
}
impl Wirecraft {
    pub(super) fn inspector_control(&self, field: &workbench::FormField, cx: &App) -> AnyElement {
        let path = field.path;
        if path == "/technology/languageId" || path.ends_with("LanguageId") {
            return language(&field.input, cx);
        }
        let options = match path {
            "/technology/technologyId" => list(
                &[
                    "c-native",
                    "go",
                    "kotlin-jvm",
                    "nodejs",
                    "php",
                    "systemjs",
                    "rust",
                    "compiler-template-set",
                    "file-export",
                    "multi-tech",
                ],
                "Inherited",
            ),
            "/fileLayoutStrategyId" => vec![
                ("none".into(), "Inherited".into()),
                ("single-file".into(), "Single file".into()),
                (
                    "direct-file-system-homomorphism".into(),
                    "Filesystem".into(),
                ),
                ("source-set".into(), "Source set".into()),
                ("classified-filesystem".into(), "Classified".into()),
            ],
            "/status" => list(
                &[
                    "BUSINESS",
                    "FUNCTIONAL",
                    "IMPLEMENTATION",
                    "TESTING",
                    "DEPLOYED",
                    "CANCELED",
                ],
                "None",
            ),
            "/io" => vec![
                (String::new(), "None".into()),
                ("in".into(), "IO-in".into()),
                ("out".into(), "IO-out".into()),
                ("inout".into(), "IO-in/out".into()),
            ],
            "/link/interactionKind" => list(
                &["auto", "data", "lib", "src", "run", "type-usage"],
                "Inherited",
            ),
            "/link/transportKind" => list(&["in-process", "error"], "Inherited"),
            "/link/typeDefinitionId" => {
                let mut v = vec![(String::new(), "None".into())];
                v.extend(types(&self.snapshot.document));
                v
            }
            "/responsible" | "/assignee" => {
                let mut v = vec![(
                    String::new(),
                    if path == "/responsible" {
                        "Inherited"
                    } else {
                        "None"
                    }
                    .into(),
                )];
                v.push(("local".into(), "Local user".into()));
                v.extend(self.snapshot.document.users.iter().filter_map(|u| {
                    Some((
                        u["id"].as_str()?.to_string(),
                        u["displayName"]
                            .as_str()
                            .or(u["fullName"].as_str())
                            .or(u["name"].as_str())
                            .or(u["id"].as_str())?
                            .to_string(),
                    ))
                }));
                v
            }
            _ => return Input::new(&field.input).small().into_any_element(),
        };
        choices(&field.input, options, "Inherited", cx)
    }
}
