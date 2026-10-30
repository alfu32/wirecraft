//! Wirecraft-owned settings and explicit Java Preferences import.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
use wirecraft_core::{Error, Result, store};
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Tool {
    pub executable: String,
    pub args: Vec<String>,
}
fn tool(executable: &str, args: &[&str]) -> Tool {
    Tool {
        executable: executable.into(),
        args: args.iter().map(|a| a.to_string()).collect(),
    }
}
fn rust_analyzer() -> Tool {
    tool("rust-analyzer", &[])
}
fn gopls() -> Tool {
    tool("gopls", &[])
}
fn kotlin_language_server() -> Tool {
    tool("kotlin-language-server", &[])
}
fn javascript_language_server() -> Tool {
    tool("typescript-language-server", &["--stdio"])
}
fn php_language_server() -> Tool {
    tool("intelephense", &["--stdio"])
}
impl Default for Tool {
    fn default() -> Self {
        Self {
            executable: "clangd".into(),
            args: vec!["--background-index=false".into()],
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Appearance {
    pub theme: String,
    pub designer_font: String,
    pub editor_font: String,
    pub composite_title_size: f32,
    pub reference_width: f64,
    pub reference_height: f64,
    pub wheel_rows: f32,
    pub colors: BTreeMap<String, String>,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            designer_font: "DejaVu Sans".into(),
            editor_font: "DejaVu Sans Mono".into(),
            composite_title_size: 18.,
            reference_width: 1500.,
            reference_height: 1000.,
            wheel_rows: 3.,
            colors: [
                ("canvasBackground", "#10161E"),
                ("gridMinor", "#202A38"),
                ("gridMajor", "#34404D"),
                ("nodeFill", "#202C38"),
                ("nodeStroke", "#8196A9"),
                ("typeFill", "#173B3A"),
                ("typeStroke", "#45BAA8"),
                ("typeText", "#58C6A4"),
                ("compilerFill", "#4C3B17"),
                ("compilerStroke", "#F0AD36"),
                ("libraryFill", "#292E43"),
                ("libraryStroke", "#8893E4"),
                ("errorFill", "#432A2A"),
                ("errorStroke", "#E27B7B"),
                ("testFill", "#273D2C"),
                ("testStroke", "#6AC284"),
                ("textPrimary", "#E6EDF3"),
                ("textSecondary", "#9DB0C3"),
                ("textMuted", "#829BAD"),
                ("selection", "#70D5C6"),
                ("linkDefault", "#9CB5C8"),
                ("linkDependency", "#E3B664"),
                ("linkError", "#E27B7B"),
                ("linkLibrary", "#8893E4"),
                ("linkSource", "#58C6A4"),
                ("linkRunnable", "#CE93D8"),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect(),
        }
    }
}
impl Appearance {
    pub fn light_colors() -> BTreeMap<String, String> {
        let mut colors = Self::default().colors;
        for (key, value) in [
            ("canvasBackground", "#F7F9FB"),
            ("gridMinor", "#E1E7ED"),
            ("gridMajor", "#C9D4DF"),
            ("nodeFill", "#FFFFFF"),
            ("nodeStroke", "#334E68"),
            ("typeFill", "#E3F4EF"),
            ("typeStroke", "#178C7C"),
            ("typeText", "#168B59"),
            ("libraryFill", "#F1F2FF"),
            ("libraryStroke", "#626BD5"),
            ("errorFill", "#FFF2F2"),
            ("errorStroke", "#BD5555"),
            ("testFill", "#EFFAF2"),
            ("testStroke", "#3D995A"),
            ("textPrimary", "#243B53"),
            ("textSecondary", "#526B7C"),
            ("textMuted", "#6C8292"),
            ("selection", "#188C80"),
            ("linkDefault", "#334E68"),
            ("linkDependency", "#A77A2C"),
        ] {
            colors.insert(key.into(), value.into());
        }
        colors
    }
    pub fn color(&self, key: &str, fallback: u32) -> u32 {
        self.colors
            .get(key)
            .and_then(|s| u32::from_str_radix(s.trim_start_matches('#'), 16).ok())
            .unwrap_or(fallback)
    }
}
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct InferenceProfile {
    pub endpoint: String,
    pub model: String,
    pub token_env: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Inference {
    pub profiles: BTreeMap<String, InferenceProfile>,
    pub provider: String,
    pub endpoint: String,
    pub model: String,
    pub token_env: String,
}
impl Default for Inference {
    fn default() -> Self {
        Self {
            profiles: [
                ("claude", "https://api.anthropic.com", "claude-sonnet"),
                ("codex", "https://api.openai.com/v1", "gpt-5-codex"),
                ("chatgpt", "https://api.openai.com/v1", "gpt-5"),
                ("ollama", "http://localhost:11434", "llama3.2"),
            ]
            .into_iter()
            .map(|(id, endpoint, model)| {
                (
                    id.into(),
                    InferenceProfile {
                        endpoint: endpoint.into(),
                        model: model.into(),
                        token_env: String::new(),
                    },
                )
            })
            .collect(),
            provider: "clipboard".into(),
            endpoint: String::new(),
            model: String::new(),
            token_env: String::new(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub appearance: Appearance,
    pub inference: Inference,
    pub editor_indent_spaces: usize,
    pub clangd: Tool,
    #[serde(default = "rust_analyzer")]
    pub rust_analyzer: Tool,
    #[serde(default = "gopls")]
    pub gopls: Tool,
    #[serde(default = "kotlin_language_server")]
    pub kotlin_language_server: Tool,
    /// Serves Node.js and systemJS bodies.
    #[serde(default = "javascript_language_server")]
    pub javascript_language_server: Tool,
    #[serde(default = "php_language_server")]
    pub php_language_server: Tool,
    pub c_compiler: String,
    pub cpp_compiler: String,
    pub legacy_preferences: BTreeMap<String, String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            appearance: Appearance::default(),
            inference: Inference::default(),
            editor_indent_spaces: 4,
            clangd: Tool::default(),
            rust_analyzer: rust_analyzer(),
            gopls: gopls(),
            kotlin_language_server: kotlin_language_server(),
            javascript_language_server: javascript_language_server(),
            php_language_server: php_language_server(),
            c_compiler: "clang".into(),
            cpp_compiler: "clang++".into(),
            legacy_preferences: BTreeMap::new(),
        }
    }
}
impl Settings {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let s: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        s.validate()?;
        Ok(s)
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || !(1..=16).contains(&self.editor_indent_spaces)
            || self.clangd.executable.trim().is_empty()
            || [
                &self.rust_analyzer,
                &self.gopls,
                &self.kotlin_language_server,
                &self.javascript_language_server,
                &self.php_language_server,
            ]
            .iter()
            .any(|t| t.executable.trim().is_empty())
        {
            return Err(Error::new(
                "invalid_input",
                "Unsupported settings schema, indentation or language server executable",
            ));
        }
        let a = &self.appearance;
        if !["dark", "light"].contains(&a.theme.as_str())
            || a.designer_font.trim().is_empty()
            || a.editor_font.trim().is_empty()
            || !a.composite_title_size.is_finite()
            || !(8.0..=72.0).contains(&a.composite_title_size)
            || !a.reference_width.is_finite()
            || !(100.0..=20000.0).contains(&a.reference_width)
            || !a.reference_height.is_finite()
            || !(100.0..=20000.0).contains(&a.reference_height)
            || !a.wheel_rows.is_finite()
            || !(1.0..=30.0).contains(&a.wheel_rows)
            || a.colors.values().any(|s| {
                s.len() != 7 || !s.starts_with('#') || u32::from_str_radix(&s[1..], 16).is_err()
            })
        {
            return Err(Error::new(
                "invalid_input",
                "Invalid appearance settings or #RRGGBB color",
            ));
        }
        if ![
            "clipboard",
            "openai",
            "anthropic",
            "ollama",
            "claude",
            "codex",
            "chatgpt",
        ]
        .contains(&self.inference.provider.as_str())
        {
            return Err(Error::new("invalid_input", "Unknown inference provider"));
        }
        if std::iter::once(self.inference.token_env.as_str())
            .chain(
                self.inference
                    .profiles
                    .values()
                    .map(|p| p.token_env.as_str()),
            )
            .any(|value| !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        {
            return Err(Error::new(
                "invalid_input",
                "Token environment variable must contain letters, digits or underscores",
            ));
        }
        if ["claude", "codex", "chatgpt", "ollama"]
            .iter()
            .any(|id| !self.inference.profiles.contains_key(*id))
        {
            return Err(Error::new(
                "invalid_input",
                "Inference profiles must include claude, codex, chatgpt and ollama",
            ));
        }
        Ok(())
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        store::atomic_write(path, &serde_json::to_vec_pretty(self)?)
    }
}
pub fn import_preferences(xml: &str) -> Result<Settings> {
    use quick_xml::{
        Reader,
        events::{BytesStart, Event},
    };
    fn attribute(reader: &Reader<&[u8]>, e: &BytesStart<'_>, key: &[u8]) -> Result<String> {
        for a in e.attributes() {
            let a = a.map_err(|e| Error::new("invalid_input", e.to_string()))?;
            if a.key.as_ref() == key {
                return a
                    .decoded_and_normalized_value(
                        quick_xml::XmlVersion::Implicit1_0,
                        reader.decoder(),
                    )
                    .map(|s| s.into_owned())
                    .map_err(|e| Error::new("invalid_input", e.to_string()));
            }
        }
        Err(Error::new(
            "invalid_input",
            "Java Preferences entry is missing an attribute",
        ))
    }
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut stack = vec![];
    let mut settings = Settings::default();
    let mut saw_preferences = false;
    loop {
        match reader
            .read_event()
            .map_err(|e| Error::new("invalid_input", e.to_string()))?
        {
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"preferences" => {
                saw_preferences = true
            }
            Event::Start(e) if e.name().as_ref() == b"node" => {
                stack.push(attribute(&reader, &e, b"name")?)
            }
            Event::End(e) if e.name().as_ref() == b"node" => {
                stack.pop();
            }
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"entry" => {
                let path = format!("/{}", stack.join("/"));
                if [
                    "/com/threadwork/app/editor",
                    "/com/threadwork/app/ui",
                    "/com/threadwork/app/designer",
                    "/com/threadwork/app/appearance",
                    "/com/threadwork/app/fonts",
                ]
                .iter()
                .any(|p| path == *p)
                {
                    let key = attribute(&reader, &e, b"key")?;
                    let value = attribute(&reader, &e, b"value")?;
                    if path.ends_with("/editor") && key == "indentSpaces" {
                        settings.editor_indent_spaces = value
                            .parse::<usize>()
                            .map_err(|_| {
                                Error::new("invalid_input", "Invalid indentation in preferences")
                            })?
                            .clamp(1, 16);
                    }
                    settings
                        .legacy_preferences
                        .insert(format!("{path}/{key}"), value);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !saw_preferences {
        return Err(Error::new(
            "invalid_input",
            "Expected a Java Preferences export with a preferences root",
        ));
    }
    let pref = |path: &str, key: &str| {
        settings
            .legacy_preferences
            .get(&format!("/com/threadwork/app/{path}/{key}"))
            .cloned()
    };
    if let Some(theme) = pref("appearance", "theme") {
        settings.appearance.theme = theme.to_lowercase();
        if settings.appearance.theme == "light" {
            settings.appearance.colors = Appearance::light_colors();
        }
    }
    if let Some(rows) = pref("appearance", "scroll.rows.per.wheel.tick") {
        settings.appearance.wheel_rows = rows
            .parse::<f32>()
            .map_err(|_| Error::new("invalid_input", "Invalid legacy wheel rows"))?
            .clamp(1., 12.);
    }
    if let Some(value) = pref("designer", "compositeTitleTargetScreenPx") {
        settings.appearance.composite_title_size = value
            .parse::<f32>()
            .map_err(|_| Error::new("invalid_input", "Invalid legacy composite title size"))?
            .clamp(8., 48.);
    }
    for (key, target) in [
        (
            "compositeReferenceViewportWidth",
            &mut settings.appearance.reference_width,
        ),
        (
            "compositeReferenceViewportHeight",
            &mut settings.appearance.reference_height,
        ),
    ] {
        if let Some(value) = pref("designer", key) {
            *target = value
                .parse::<f64>()
                .map_err(|_| Error::new("invalid_input", "Invalid legacy reference viewport"))?
                .clamp(300., 10000.);
        }
    }
    for (key, value) in &mut settings.appearance.colors {
        if let Some(color) = pref(
            "appearance",
            &format!(
                "{}.{}",
                settings.appearance.theme,
                match key.as_str() {
                    "linkSource" => "linksourcecapability".into(),
                    "linkRunnable" => "linkrunnablecapability".into(),
                    _ => key.to_lowercase(),
                }
            ),
        ) {
            *value = color;
        }
    }
    let font_name = |id: &str| match id {
        "system-monospace" => "DejaVu Sans Mono",
        "d-din-regular" | "d-din-bold" | "d-din-italic" => "D-DIN",
        "monaspace-neon-regular"
        | "monaspace-neon-light"
        | "monaspace-neon-bold"
        | "monaspace-neon-italic" => "Monaspace Neon",
        "monaspace-krypton-regular"
        | "monaspace-krypton-light"
        | "monaspace-krypton-bold"
        | "monaspace-krypton-italic" => "Monaspace Krypton",
        _ => "DejaVu Sans Mono",
    };
    if let Some(value) = pref("fonts", "designerFont") {
        settings.appearance.designer_font = font_name(&value).into();
    }
    if let Some(value) = pref("fonts", "codeFont") {
        settings.appearance.editor_font = font_name(&value).into();
    }
    settings.validate()?;
    Ok(settings)
}
