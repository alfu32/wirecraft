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
pub struct Settings {
    pub schema_version: u32,
    pub editor_indent_spaces: usize,
    pub clangd: Tool,
    pub c_compiler: String,
    pub cpp_compiler: String,
    pub legacy_preferences: BTreeMap<String, String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            editor_indent_spaces: 4,
            clangd: Tool::default(),
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
        {
            return Err(Error::new(
                "invalid_input",
                "Unsupported settings schema, indentation or language server executable",
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
    settings.validate()?;
    Ok(settings)
}
