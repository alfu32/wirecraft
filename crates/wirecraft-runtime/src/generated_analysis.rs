//! A generated translation unit and its independent supervised language server workspace:
//! clangd for C17 units, rust-analyzer for Rust programs (with their `Cargo.toml`).
use crate::lsp::LanguageSession;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::broadcast;
use wirecraft_core::{
    Error, Result,
    generation::{GeneratedProject, VirtualFile},
};
pub struct GeneratedAnalysis {
    /// LSP language of the unit: `c` or `rust`.
    pub language: String,
    directory: tempfile::TempDir,
    session: LanguageSession,
    uri: String,
    version: i64,
    file: VirtualFile,
    staged: Vec<String>,
}
impl GeneratedAnalysis {
    pub async fn start(
        project: GeneratedProject,
        language: &str,
        executable: &str,
        args: &[String],
    ) -> Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix(if language == "rust" {
                "wirecraft-rust-analysis-"
            } else {
                "wirecraft-c17-analysis-"
            })
            .tempdir()?;
        let (project, staged) = stage_project(project, directory.path().to_owned(), vec![]).await?;
        let file = project.files.into_iter().next().ok_or_else(|| {
            Error::new("invalid_input", "Generated project has no translation unit")
        })?;
        let path = directory.path().join(&file.path);
        let uri = url::Url::from_file_path(&path)
            .map_err(|_| Error::new("invalid_input", "Invalid generated URI"))?
            .to_string();
        let session = if language == "rust" {
            // Readiness, `cargo check` progress and Markdown hovers.
            let capabilities = json!({"experimental":{"serverStatusNotification":true},"window":{"workDoneProgress":true},"textDocument":{"hover":{"contentFormat":["markdown","plaintext"]}}});
            LanguageSession::start_with(executable, args, directory.path(), capabilities).await?
        } else {
            LanguageSession::start(executable, args, directory.path()).await?
        };
        let mut analysis = Self {
            language: language.into(),
            directory,
            session,
            uri,
            version: 0,
            file,
            staged,
        };
        analysis.synchronize(None).await?;
        if language == "rust" {
            analysis.ready().await;
        }
        Ok(analysis)
    }
    /// rust-analyzer answers before it has loaded the workspace and the standard
    /// library, with incomplete results; wait until it reports itself quiescent.
    async fn ready(&self) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
        while self.session.alive()
            && self.session.status()["quiescent"] != true
            && tokio::time::Instant::now() < deadline
        {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    async fn synchronize(&mut self, next: Option<VirtualFile>) -> Result<()> {
        if let Some(file) = next {
            self.file = file;
        }
        self.version += 1;
        let path = self.directory.path().join(&self.file.path);
        let content = self.file.content.clone();
        tokio::task::spawn_blocking(move || {
            wirecraft_core::store::atomic_write(&path, content.as_bytes())
        })
        .await
        .map_err(|e| Error::new("internal", e.to_string()))??;
        if self.version == 1 {
            self.session
                .open(&self.uri, &self.language, &self.file.content, self.version)
                .await
        } else {
            self.session
                .change(&self.uri, &self.file.content, self.version)
                .await
        }
    }
    pub async fn update(&mut self, project: GeneratedProject) -> Result<()> {
        let (project, next) = stage_project(
            project,
            self.directory.path().to_owned(),
            self.staged.clone(),
        )
        .await?;
        self.staged = next;
        let file = project
            .files
            .into_iter()
            .next()
            .ok_or_else(|| Error::new("invalid_input", "Generated project is empty"))?;
        if file.path != self.file.path {
            self.session.close(&self.uri).await?;
            self.uri = url::Url::from_file_path(self.directory.path().join(&file.path))
                .map_err(|_| Error::new("invalid_input", "Invalid generated URI"))?
                .to_string();
            self.version = 0;
        }
        self.synchronize(Some(file)).await
    }
    fn entries(&self, diagnostics: &[Value]) -> Vec<Value> {
        diagnostics.iter().map(|d| {
            let line = d["range"]["start"]["line"].as_u64().unwrap_or(0) as usize + 1;
            let column = d["range"]["start"]["character"].as_u64().unwrap_or(0) as usize + 1;
            json!({"message":d["message"],"severity":d["severity"],"code":d["code"],"source":self.file.source_location(line,column),"generated":{"uri":self.uri,"line":line,"column":column},"diagnostic":d})
        }).collect()
    }
    /// rust-analyzer's own diagnostics, which it serves on request only.
    async fn native_diagnostics(&self) -> Result<Vec<Value>> {
        let report = self
            .session
            .request(
                "textDocument/diagnostic",
                json!({"textDocument":{"uri":self.uri}}),
            )
            .await?;
        Ok(report["items"].as_array().cloned().unwrap_or_default())
    }
    /// Saving the staged program runs rust-analyzer's `cargo check`, whose rustc
    /// diagnostics are authoritative. Without one (check disabled, no cargo) the
    /// server's native diagnostics are reported instead.
    async fn rust_diagnostics(&self) -> Result<Value> {
        let mut events = self.session.subscribe();
        self.session
            .notify(
                "textDocument/didSave",
                json!({"textDocument":{"uri":self.uri}}),
            )
            .await?;
        let flycheck = |event: &Value, kind: &str| {
            event["method"] == "$/progress"
                && event["data"]["token"]
                    .as_str()
                    .is_some_and(|t| t.contains("flycheck"))
                && event["data"]["value"]["kind"] == kind
        };
        let mut checked = false;
        let started = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                match events.recv().await {
                    Ok(event) if flycheck(&event, "begin") => return true,
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(_) => return false,
                }
            }
        })
        .await
        .unwrap_or(false);
        if started {
            // A check restarted while it runs (a workspace reload, another save)
            // ends without results and begins again; wait for the last one.
            checked = tokio::time::timeout(Duration::from_secs(120), async {
                let mut running = true;
                loop {
                    let wait = if running {
                        Duration::from_secs(120)
                    } else {
                        Duration::from_millis(500)
                    };
                    match tokio::time::timeout(wait, events.recv()).await {
                        Err(_) => return true,
                        Ok(Ok(event)) if flycheck(&event, "end") => running = false,
                        Ok(Ok(event)) if flycheck(&event, "begin") => running = true,
                        Ok(Ok(_) | Err(broadcast::error::RecvError::Lagged(_))) => {}
                        Ok(Err(_)) => return false,
                    }
                }
            })
            .await
            .map_err(|_| Error::new("timeout", "cargo check has not finished"))?;
        }
        if !self.session.alive() {
            return Err(Error::new("process_failed", "rust-analyzer exited"));
        }
        let published = self.session.diagnostics();
        let diagnostics = if checked {
            if published[&self.uri]["version"] == self.version {
                published[&self.uri]["diagnostics"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
            } else {
                // A check without findings for an unchanged file publishes nothing.
                vec![]
            }
        } else {
            self.native_diagnostics().await?
        };
        Ok(
            json!({"diagnostics":self.entries(&diagnostics),"checker":if checked {"cargo check"} else {"rust-analyzer"},"positionEncoding":self.session.encoding,"uri":self.uri,"version":self.version,"pid":self.session.pid()}),
        )
    }
    pub async fn diagnostics(&self) -> Result<Value> {
        if self.language == "rust" {
            return self.rust_diagnostics().await;
        }
        let mut events = self.session.subscribe();
        tokio::time::timeout(Duration::from_secs(10),async{
   loop{let diagnostics=self.session.diagnostics();if diagnostics[&self.uri]["version"]==self.version{
    let entries=self.entries(diagnostics[&self.uri]["diagnostics"].as_array().map_or(&[][..],|v|v));return Ok(json!({"diagnostics":entries,"positionEncoding":self.session.encoding,"uri":self.uri,"version":self.version,"pid":self.session.pid()}));
   }
   if !self.session.alive(){return Err(Error::new("process_failed","Generated-source clangd exited"));}
   events.recv().await.map_err(|e|Error::new("process_failed",e.to_string()))?;
  }
 }).await.map_err(|_|Error::new("timeout","clangd has not published generated-source diagnostics"))?
    }
    pub async fn shutdown(&self) {
        self.session.shutdown().await;
    }
}

impl GeneratedAnalysis {
    pub fn alive(&self) -> bool {
        self.session.alive()
    }
    pub async fn complete(
        &self,
        context: &wirecraft_core::editor::BodyContext,
        text: &str,
        offset: usize,
    ) -> Result<Value> {
        use crate::lsp::{offset as byte_offset, position};
        let local = position(text, offset, "utf-32")?;
        let line =
            context.source.generated_start_line - 1 + local["line"].as_u64().unwrap() as usize;
        let column =
            context.source.generated_column_offset + local["character"].as_u64().unwrap() as usize;
        let generated_offset = byte_offset(&self.file.content, line, column, "utf-32")?;
        let response = self.session.request("textDocument/completion", json!({"textDocument":{"uri":self.uri},"position":position(&self.file.content, generated_offset, &self.session.encoding)?,"context":{"triggerKind":1}})).await?;
        let entries = response.as_array().or_else(|| response["items"].as_array());
        let start = text[..offset]
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
            .last()
            .map(|(i, _)| i)
            .unwrap_or(offset);
        let prefix = &text[start..offset];
        let fallback = json!({"start":position(text,start,"utf-32")?,"end":local});
        let map_position = |p: &Value| -> Option<Value> {
            let byte = byte_offset(
                &self.file.content,
                p["line"].as_u64()? as usize,
                p["character"].as_u64()? as usize,
                &self.session.encoding,
            )
            .ok()?;
            let p = position(&self.file.content, byte, "utf-32").ok()?;
            let line = (p["line"].as_u64()? as usize)
                .checked_sub(context.source.generated_start_line - 1)?;
            let character = (p["character"].as_u64()? as usize)
                .checked_sub(context.source.generated_column_offset)?;
            byte_offset(text, line, character, "utf-32").ok()?;
            Some(json!({"line":line,"character":character}))
        };
        let mut items = Vec::new();
        let mut names = std::collections::BTreeSet::new();
        for item in entries.into_iter().flatten() {
            if item["additionalTextEdits"]
                .as_array()
                .is_some_and(|v| !v.is_empty())
                || item["insertTextFormat"] == 2
            {
                continue;
            }
            let label = item["filterText"]
                .as_str()
                .or(item["label"].as_str())
                .unwrap_or("")
                .trim();
            if context
                .excluded_library_prefixes
                .iter()
                .any(|p| label.starts_with(p))
            {
                continue;
            }
            let edit = &item["textEdit"];
            let mut range = fallback.clone();
            if edit.is_object() {
                let source_range = edit
                    .get("range")
                    .or_else(|| edit.get("replace"))
                    .or_else(|| edit.get("insert"));
                let Some(source_range) = source_range else {
                    continue;
                };
                let (Some(start), Some(end)) = (
                    map_position(&source_range["start"]),
                    map_position(&source_range["end"]),
                ) else {
                    continue;
                };
                range = json!({"start":start,"end":end});
            }
            let new_text = edit["newText"]
                .as_str()
                .or(item["insertText"].as_str())
                .or(item["label"].as_str())
                .unwrap_or("");
            let mut item = item.clone();
            item["textEdit"] = json!({"range":range,"newText":new_text});
            item.as_object_mut().unwrap().remove("additionalTextEdits");
            names.insert(label.to_string());
            items.push(item);
        }
        // Model context adds direction/type details for the exact generated buffer arguments.
        if !text[..start].ends_with('.')
            && !text[..start].ends_with("->")
            && !text[..start].ends_with("::")
        {
            for symbol in &context.symbols {
                let label = symbol["label"].as_str().unwrap();
                if !label.starts_with(prefix) {
                    continue;
                }
                let mut item = symbol.clone();
                item["textEdit"] = json!({"range":fallback,"newText":label});
                if let Some(existing) = items.iter_mut().find(|i| {
                    i["filterText"]
                        .as_str()
                        .or(i["label"].as_str())
                        .is_some_and(|s| s.trim() == label)
                }) {
                    existing["detail"] = symbol["detail"].clone();
                } else if names.insert(label.into()) {
                    items.push(item);
                }
            }
        }
        // rust-analyzer leaves filtering to the client: keep items whose name contains the
        // typed prefix's characters in order, and rank those starting with it first.
        let name = |i: &Value| {
            i["filterText"]
                .as_str()
                .or(i["label"].as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase()
        };
        let typed = prefix.to_lowercase();
        items.retain(|i| {
            let mut rest = name(i);
            typed.chars().all(|c| match rest.find(c) {
                Some(at) => {
                    rest.drain(..at + c.len_utf8());
                    true
                }
                None => false,
            })
        });
        items.sort_by_cached_key(|i| {
            (
                !name(i).starts_with(&typed),
                i["sortText"]
                    .as_str()
                    .or(i["filterText"].as_str())
                    .or(i["label"].as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            )
        });
        Ok(
            json!({"items":items,"isIncomplete":response["isIncomplete"].as_bool().unwrap_or(false),"positionEncoding":"utf-32","serverPositionEncoding":self.session.encoding,"pid":self.session.pid(),"version":self.version}),
        )
    }
}

fn stage_files(project: &GeneratedProject, directory: &std::path::Path) -> Result<Vec<String>> {
    use base64::Engine;
    let mut paths = std::collections::BTreeSet::new();
    for file in &project.files {
        wirecraft_core::generation::safe_relative_path(&file.path)?;
        if !paths.insert(file.path.clone()) {
            return Err(Error::new(
                "invalid_input",
                "Duplicate generated analysis path",
            ));
        }
        let data = if let Some(binary) = &file.binary_content {
            base64::engine::general_purpose::STANDARD
                .decode(binary)
                .map_err(|e| Error::new("invalid_input", e.to_string()))?
        } else {
            file.content.as_bytes().to_vec()
        };
        // Rewriting an unchanged manifest would make rust-analyzer reload the workspace.
        let path = directory.join(&file.path);
        if std::fs::read(&path).ok().as_deref() != Some(&data[..]) {
            wirecraft_core::store::atomic_write(&path, &data)?;
        }
    }
    Ok(paths.into_iter().collect())
}
async fn stage_project(
    project: GeneratedProject,
    directory: std::path::PathBuf,
    old: Vec<String>,
) -> Result<(GeneratedProject, Vec<String>)> {
    tokio::task::spawn_blocking(move || {
        let staged = stage_files(&project, &directory)?;
        for path in old {
            if !staged.contains(&path) {
                let _ = std::fs::remove_file(directory.join(path));
            }
        }
        Ok((project, staged))
    })
    .await
    .map_err(|e| Error::new("internal", e.to_string()))?
}

impl GeneratedAnalysis {
    /// `textDocument/definition` or `textDocument/references` at a body offset, with
    /// every location mapped to the model (`BodyContext::locate`). Locations in other
    /// files are `generated` (staged companions) or `external` (system headers, the
    /// Rust standard library).
    pub async fn locations(
        &self,
        context: &wirecraft_core::editor::BodyContext,
        text: &str,
        offset: usize,
        method: &str,
    ) -> Result<Value> {
        use crate::lsp::{offset as byte_offset, position};
        let local = position(text, offset, "utf-32")?;
        let line =
            context.source.generated_start_line - 1 + local["line"].as_u64().unwrap() as usize;
        let column =
            context.source.generated_column_offset + local["character"].as_u64().unwrap() as usize;
        let generated = byte_offset(&self.file.content, line, column, "utf-32")?;
        let mut params = json!({"textDocument":{"uri":self.uri},"position":position(&self.file.content,generated,&self.session.encoding)?});
        if method == "textDocument/references" {
            params["context"] = json!({"includeDeclaration":true});
        }
        let response = self.session.request(method, params.clone()).await?;
        let start = text[..offset]
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
            .last()
            .map_or(offset, |(i, _)| i);
        let end = offset
            + text[offset..]
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(text.len() - offset);
        let identifier = &text[start..end];
        // Location, Location[] or LocationLink[]; a link's selection range is the name.
        let flatten = |response: Value| match response {
            Value::Array(entries) => entries,
            Value::Null => vec![],
            single => vec![single],
        };
        let mut entries = flatten(response);
        if method == "textDocument/references" {
            for (line, character) in context.related_positions(identifier) {
                let generated = byte_offset(&self.file.content, line, character, "utf-32")?;
                params["position"] =
                    position(&self.file.content, generated, &self.session.encoding)?;
                entries.extend(flatten(self.session.request(method, params.clone()).await?));
            }
        }
        let mut locations = Vec::new();
        for entry in entries {
            let Some(uri) = entry["uri"].as_str().or(entry["targetUri"].as_str()) else {
                continue;
            };
            let range = if entry["targetSelectionRange"].is_object() {
                &entry["targetSelectionRange"]
            } else if entry["targetRange"].is_object() {
                &entry["targetRange"]
            } else {
                &entry["range"]
            };
            let (Some(row), Some(character)) = (
                range["start"]["line"].as_u64(),
                range["start"]["character"].as_u64(),
            ) else {
                continue;
            };
            let location = if uri == self.uri {
                let Ok(byte) = byte_offset(
                    &self.file.content,
                    row as usize,
                    character as usize,
                    &self.session.encoding,
                ) else {
                    continue;
                };
                let p = position(&self.file.content, byte, "utf-32")?;
                context.locate(
                    p["line"].as_u64().unwrap() as usize,
                    p["character"].as_u64().unwrap() as usize,
                    identifier,
                )
            } else {
                let path = url::Url::parse(uri)
                    .ok()
                    .and_then(|u| u.to_file_path().ok())
                    .unwrap_or_default();
                match path.strip_prefix(self.directory.path()) {
                    Ok(relative) => {
                        json!({"kind":"generated","path":relative.to_string_lossy(),"line":row+1,"column":character+1,"text":""})
                    }
                    Err(_) => {
                        json!({"kind":"external","path":path.to_string_lossy(),"line":row+1,"column":character+1,"text":""})
                    }
                }
            };
            if !locations.contains(&location) {
                locations.push(location);
            }
        }
        // Authored bodies first, then entities, then generated and external code.
        locations.sort_by_key(|l| {
            (
                ["body", "entity", "generated", "external"]
                    .iter()
                    .position(|k| l["kind"] == *k),
                l["source"]["nodeId"].as_str().map(str::to_string),
                l["source"]["line"].as_u64().or(l["line"].as_u64()),
                l["source"]["column"].as_u64().or(l["column"].as_u64()),
            )
        });
        Ok(
            json!({"locations":locations,"identifier":identifier,"version":self.version,"pid":self.session.pid()}),
        )
    }
    pub async fn hover(
        &self,
        context: &wirecraft_core::editor::BodyContext,
        text: &str,
        offset: usize,
    ) -> Result<Value> {
        use crate::lsp::{offset as byte_offset, position};
        let local = position(text, offset, "utf-32")?;
        let line =
            context.source.generated_start_line - 1 + local["line"].as_u64().unwrap() as usize;
        let column =
            context.source.generated_column_offset + local["character"].as_u64().unwrap() as usize;
        let generated = byte_offset(&self.file.content, line, column, "utf-32")?;
        let mut hover=self.session.request("textDocument/hover",json!({"textDocument":{"uri":self.uri},"position":position(&self.file.content,generated,&self.session.encoding)?})).await?;
        // The widget already knows the hovered word's exact UTF-8 range. LSP's
        // generated range must never be interpreted as an authored body range.
        if let Some(object) = hover.as_object_mut() {
            object.remove("range");
        }
        let start = text[..offset]
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
            .last()
            .map_or(offset, |(i, _)| i);
        let end = offset
            + text[offset..]
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(text.len() - offset);
        let identifier = &text[start..end];
        if let Some(symbol) = context
            .symbols
            .iter()
            .find(|s| s["label"].as_str() == Some(identifier))
        {
            let signature = symbol["detail"].as_str().unwrap_or("");
            let provenance = symbol["documentation"].as_str().unwrap_or("");
            // Library aliases are function pointers in generated C. Show the
            // callable signature plus its model dependency, not an anonymous
            // pointer type whose provenance has been lost.
            hover = json!({"contents":{"kind":"markdown","value":format!("```{}\n{signature}\n```\n\n{provenance}",self.language)}});
        }
        if let Some(signature) = context.type_signatures.get(identifier) {
            hover = json!({"contents":{"kind":"markdown","value":format!("```{}\n{signature}\n```",self.language)}});
        }
        Ok(
            json!({"hover":hover,"identifier":identifier,"version":self.version,"pid":self.session.pid()}),
        )
    }
    pub async fn semantic_tokens(
        &self,
        context: &wirecraft_core::editor::BodyContext,
        text: &str,
    ) -> Result<Value> {
        use crate::lsp::{offset, position};
        let provider = &self.session.capabilities["semanticTokensProvider"];
        if !provider.is_object() {
            return Err(Error::new(
                "unsupported",
                "Language server does not advertise semantic tokens",
            ));
        }
        let response = self
            .session
            .request(
                "textDocument/semanticTokens/full",
                json!({"textDocument":{"uri":self.uri}}),
            )
            .await?;
        let data = response["data"]
            .as_array()
            .ok_or_else(|| Error::new("invalid_response", "Missing semantic token data"))?;
        if data.len() % 5 != 0 {
            return Err(Error::new(
                "invalid_response",
                "Invalid semantic token tuple count",
            ));
        }
        let types = provider["legend"]["tokenTypes"]
            .as_array()
            .ok_or_else(|| Error::new("invalid_response", "Missing semantic token legend"))?;
        let modifiers = provider["legend"]["tokenModifiers"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut line = 0usize;
        let mut column = 0usize;
        let mut tokens = Vec::new();
        for tuple in data.as_chunks::<5>().0 {
            let values = tuple
                .iter()
                .map(|v| {
                    v.as_u64()
                        .and_then(|n| usize::try_from(n).ok())
                        .ok_or_else(|| {
                            Error::new("invalid_response", "Invalid semantic token value")
                        })
                })
                .collect::<Result<Vec<_>>>()?;
            line = line
                .checked_add(values[0])
                .ok_or_else(|| Error::new("invalid_response", "Semantic line overflow"))?;
            column = if values[0] == 0 {
                column
                    .checked_add(values[1])
                    .ok_or_else(|| Error::new("invalid_response", "Semantic column overflow"))?
            } else {
                values[1]
            };
            if line + 1 < context.source.generated_start_line
                || line + 1 > context.source.generated_end_line
            {
                continue;
            }
            let end_column = column
                .checked_add(values[2])
                .ok_or_else(|| Error::new("invalid_response", "Semantic range overflow"))?;
            let (Ok(start), Ok(end)) = (
                offset(&self.file.content, line, column, &self.session.encoding),
                offset(&self.file.content, line, end_column, &self.session.encoding),
            ) else {
                continue;
            };
            let p = position(&self.file.content, start, "utf-32")?;
            let q = position(&self.file.content, end, "utf-32")?;
            let local_line = line - (context.source.generated_start_line - 1);
            let (Some(a), Some(b)) = (
                (p["character"].as_u64().unwrap() as usize)
                    .checked_sub(context.source.generated_column_offset),
                (q["character"].as_u64().unwrap() as usize)
                    .checked_sub(context.source.generated_column_offset),
            ) else {
                continue;
            };
            let (Ok(start), Ok(end)) = (
                offset(text, local_line, a, "utf-32"),
                offset(text, local_line, b, "utf-32"),
            ) else {
                continue;
            };
            if start >= end {
                continue;
            }
            let mut kind = types
                .get(values[3])
                .and_then(Value::as_str)
                .unwrap_or("variable");
            if context
                .symbols
                .iter()
                .any(|s| s["kind"] == 3 && s["label"].as_str() == Some(&text[start..end]))
            {
                kind = "function";
            }
            let flags = modifiers
                .iter()
                .enumerate()
                .filter(|(bit, _)| *bit < usize::BITS as usize && values[4] & (1usize << bit) != 0)
                .map(|(_, s)| s.clone())
                .collect::<Vec<_>>();
            tokens.push(json!({"startByte":start,"endByte":end,"kind":kind,"modifiers":flags,"range":{"start":{"line":local_line,"character":a},"end":{"line":local_line,"character":b}}}));
        }
        // Do not wait for a separate diagnostics publication while holding the
        // editor session. Automatic compilation owns that wait independently.
        let published = self.session.diagnostics();
        let (current, diagnostics) = if self.language == "rust" {
            // Native diagnostics are computed for this exact version on request.
            (true, json!(self.entries(&self.native_diagnostics().await?)))
        } else if published[&self.uri]["version"] == self.version {
            (true, self.diagnostics().await?["diagnostics"].clone())
        } else {
            (false, Value::Null)
        };
        Ok(
            json!({"tokens":tokens,"diagnostics":diagnostics,"diagnosticsCurrent":current,"positionEncoding":"utf-32","serverPositionEncoding":self.session.encoding,"version":self.version,"pid":self.session.pid(),"generatedBytes":self.file.content.len(),"generatedFiles":self.staged.len()}),
        )
    }
}
