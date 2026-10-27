//! Portable compiler template sets; no UI, filesystem loader, JVM or subprocess.
use crate::{Document, Error, Result};
use minijinja::{
    AutoEscape, Environment,
    value::{Serde, Value as TemplateValue},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
include!(concat!(env!("OUT_DIR"), "/templates.rs"));
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTemplate {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub layouts: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSet {
    pub id: String,
    pub language: String,
    pub technology: String,
    pub extension: String,
    pub default_layout: String,
    pub roles: BTreeMap<String, String>,
    #[serde(default)]
    pub project_files: Vec<ProjectTemplate>,
    #[serde(default)]
    pub static_files: Vec<String>,
}
pub fn render(source: &str, context: &Value) -> Result<String> {
    let mut env = Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);
    env.set_auto_escape_callback(|_| AutoEscape::None);
    env.set_fuel(Some(2_000_000));
    env.add_test("empty", |v: TemplateValue| {
        v.is_undefined() || v.is_none() || v.len() == Some(0)
    });
    let normalized = regex::Regex::new(r"\$\{([A-Za-z0-9_.]+)\}")
        .unwrap()
        .replace_all(source, "{{ $1 }}")
        .replace("{% elseif", "{% elif");
    env.add_template("role", &normalized)
        .map_err(template_error)?;
    struct LimitedOutput(Vec<u8>);
    impl std::io::Write for LimitedOutput {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.0.len().saturating_add(bytes.len()) > 16 * 1024 * 1024 {
                return Err(std::io::Error::other("Rendered template exceeds 16 MiB"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut result = LimitedOutput(Vec::new());
    env.get_template("role")
        .map_err(template_error)?
        .render_captured_to(Serde(context), &mut result)
        .map_err(template_error)?;
    Ok(String::from_utf8(result.0).expect("Template output is UTF-8"))
}
fn template_error(e: minijinja::Error) -> Error {
    Error::new("template_error", format!("{e:#}"))
}
impl TemplateSet {
    pub fn builtin(id: &str) -> Result<Self> {
        let (language, technology) = match id {
            "c" => ("c", "c-native"),
            "go" => ("go", "go"),
            "kotlin" => ("kotlin", "kotlin-jvm"),
            "nodejs" => ("javascript", "nodejs"),
            "php" => ("php", "php"),
            "quickjs" => ("javascript", "quickjs"),
            _ => {
                return Err(Error::new(
                    "not_found",
                    format!("Unknown compiler template set '{id}'"),
                ));
            }
        };
        let asset = |path: &str| {
            ASSETS
                .iter()
                .find(|(name, _)| *name == format!("{id}/{path}"))
                .map(|(_, text)| *text)
                .ok_or_else(|| {
                    Error::new("not_found", format!("Missing compiler asset {id}/{path}"))
                })
        };
        let properties = asset("compiler.properties")?
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
            .filter_map(|s| s.split_once('='))
            .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            .collect::<BTreeMap<_, _>>();
        let mut roles = BTreeMap::new();
        for (key, path) in &properties {
            if let Some(role) = key.strip_prefix("template.") {
                roles.insert(role.into(), asset(path)?.into());
            }
        }
        let mut project_files = vec![];
        for i in 1..=128 {
            if let Some(path) = properties.get(&format!("project.{i}.path")) {
                let key = format!("project.{i}.content");
                let content = properties
                    .get(&key)
                    .ok_or_else(|| Error::new("invalid_input", format!("Missing {key}")))?;
                project_files.push(ProjectTemplate {
                    path: path.clone(),
                    content: asset(content)?.into(),
                    layouts: properties
                        .get(&format!("project.{i}.layouts"))
                        .map(|s| s.split(',').map(str::to_string).collect())
                        .unwrap_or_default(),
                });
            }
        }
        Ok(Self {
            id: id.into(),
            language: language.into(),
            technology: technology.into(),
            extension: properties.get("fileExtension").cloned().unwrap_or_default(),
            default_layout: properties
                .get("defaultLayoutStrategyId")
                .cloned()
                .unwrap_or_else(|| "single-file".into()),
            roles,
            project_files,
            static_files: properties
                .get("staticFiles")
                .into_iter()
                .flat_map(|s| s.split(','))
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
        })
    }
    pub fn generic(d: &Document) -> Self {
        let n = &d.nodes[&d.root_node_id];
        Self {
            id: "generic".into(),
            language: d.effective(&n.id, "languageId"),
            technology: "compiler-template-set".into(),
            extension: if n.technology.file_extension.is_empty() {
                "txt".into()
            } else {
                n.technology.file_extension.clone()
            },
            default_layout: "classified-filesystem".into(),
            roles: BTreeMap::new(),
            project_files: vec![],
            static_files: vec![],
        }
    }
    pub fn overlay(&mut self, d: &Document) -> Result<()> {
        for n in d.nodes.values().filter(|n| n.name.starts_with('@')) {
            let content = [n.text.initialization.as_str(), n.text.source.as_str()]
                .into_iter()
                .filter(|s| !s.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            let content = if content.is_empty() {
                n.text.specification.clone()
            } else {
                content
            };
            if n.name.eq_ignore_ascii_case("@ProjectFile") {
                let path = n
                    .metadata
                    .get("path")
                    .or_else(|| n.metadata.get("file"))
                    .ok_or_else(|| {
                        Error::new("invalid_input", format!("{} needs metadata.path", n.id))
                    })?;
                self.project_files.retain(|file| file.path != *path);
                self.project_files.push(ProjectTemplate {
                    path: path.clone(),
                    content,
                    layouts: vec![],
                });
                continue;
            }
            if n.name.eq_ignore_ascii_case("@Compiler")
                || n.name.eq_ignore_ascii_case("@StaticFile")
            {
                continue;
            }
            if n.name.eq_ignore_ascii_case("@StaticFiles") {
                self.static_files.extend(
                    content
                        .lines()
                        .map(str::trim)
                        .filter(|s| !s.is_empty() && !s.starts_with('#'))
                        .map(str::to_string),
                );
                continue;
            }
            let name = n.name.trim_start_matches('@');
            let role = override_role(name);
            self.roles.insert(role, content);
        }
        Ok(())
    }
    pub fn role(
        &self,
        kind: &str,
        stereotype: &str,
        composite: bool,
        phase: &str,
        layout: &str,
    ) -> Option<&str> {
        let stereo = kebab(if composite && stereotype == "Node" {
            "CompositeWorker"
        } else {
            stereotype
        });
        let fallback = if composite { "composite" } else { kind };
        [
            format!("{stereo}.{phase}.{layout}"),
            format!("{stereo}.{phase}"),
            format!("{fallback}.{phase}.{layout}"),
            format!("{fallback}.{phase}"),
            format!("node.{phase}.{layout}"),
            format!("node.{phase}"),
        ]
        .iter()
        .find_map(|key| self.roles.get(key).map(String::as_str))
    }
    pub fn catalog() -> Value {
        let sets=["c","go","kotlin","nodejs","php","quickjs"].into_iter().map(|id|{let set=Self::builtin(id).unwrap();json!({"id":id,"languageId":set.language,"technologyId":set.technology,"defaultLayout":set.default_layout,"roles":set.roles.keys().collect::<Vec<_>>(),"projectFiles":set.project_files,"sourceRevision":"7d392d3a0e30b86445e00e7c03032a5188b26593"})}).collect::<Vec<_>>();
        json!({"templateSets":sets})
    }
}
fn override_role(name: &str) -> String {
    let normalized = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase();
    for subject in [
        "node",
        "processor",
        "group",
        "type",
        "link",
        "note",
        "generator",
        "transformer",
        "sink",
        "service-library",
        "composite-worker",
        "composite-error-handler",
        "error-handler",
        "test-suite",
    ] {
        for phase in [
            "declaration",
            "instantiation",
            "hoisted-declaration",
            "forward-declaration",
        ] {
            let key = format!("{subject}.{phase}");
            if key
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                == normalized
            {
                return key;
            }
        }
    }
    let token = kebab(name);
    for (old, new) in [
        ("-hoisted-declaration", ".hoisted-declaration"),
        ("-forward-declaration", ".forward-declaration"),
        ("-declaration", ".declaration"),
        ("-instantiation", ".instantiation"),
    ] {
        if let Some(prefix) = token.strip_suffix(old) {
            return format!("{prefix}{new}");
        }
    }
    match token.as_str() {
        "composite-single-file" => "composite.single-file",
        "composite-file-based" => "composite.file-based",
        "composite-direct-file-system" => "composite.direct-file-system",
        "composite-classified-file-system" => "composite.classified-file-system",
        "composite-source-set" => "composite.source-set",
        "runtime-support" => "runtime.support",
        "child-import" => "child.import",
        "primary-file-path" => "primary-file.path",
        "static-file-path" => "static-file.path",
        "project-name" => "project.name",
        "processor" => "processor.declaration",
        "node" => "node.declaration",
        "group" => "group.declaration",
        "type" => "type.declaration",
        "note" => "note.declaration",
        "link" => "link.declaration",
        _ => &token,
    }
    .into()
}
pub(crate) fn kebab(value: &str) -> String {
    regex::Regex::new(r"([a-z0-9])([A-Z])")
        .unwrap()
        .replace_all(value, "$1-$2")
        .to_lowercase()
}
