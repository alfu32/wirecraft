//! Compact value controls shared by inspector rows and inline tables.
use super::*;
use gpui_component::{
    Sizable,
    button::Button,
    menu::{DropdownMenu, PopupMenuItem},
};
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
    Button::new(("choice", input.entity_id()))
        .label(label)
        .small()
        .outline()
        .dropdown_caret(true)
        .w_full()
        .dropdown_menu_with_anchor(Corner::TopRight, move |menu, _, _| {
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
            "/technology/technologyId" => {
                list(&["c-native", "file-export", "multi-tech"], "Inherited")
            }
            "/fileLayoutStrategyId" => vec![
                ("none".into(), "Inherited".into()),
                ("single-file".into(), "Single file".into()),
                ("direct-filesystem".into(), "Filesystem".into()),
                ("source-set".into(), "Source set".into()),
                ("classified".into(), "Classified".into()),
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
