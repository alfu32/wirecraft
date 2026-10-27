//! A generated translation unit and its independent supervised clangd workspace.
use crate::lsp::LanguageSession;
use serde_json::{Value, json};
use std::time::Duration;
use wirecraft_core::{
    Error, Result,
    generation::{GeneratedProject, VirtualFile},
};
pub struct GeneratedAnalysis {
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
        executable: &str,
        args: &[String],
    ) -> Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("wirecraft-c17-analysis-")
            .tempdir()?;
        let staged = stage_files(&project, directory.path())?;
        let file = project.files.into_iter().next().ok_or_else(|| {
            Error::new("invalid_input", "Generated project has no translation unit")
        })?;
        let path = directory.path().join(&file.path);
        let uri = url::Url::from_file_path(&path)
            .map_err(|_| Error::new("invalid_input", "Invalid generated URI"))?
            .to_string();
        let session = LanguageSession::start(executable, args, directory.path()).await?;
        let mut analysis = Self {
            directory,
            session,
            uri,
            version: 0,
            file,
            staged,
        };
        analysis.synchronize(None).await?;
        Ok(analysis)
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
                .open(&self.uri, "c", &self.file.content, self.version)
                .await
        } else {
            self.session
                .change(&self.uri, &self.file.content, self.version)
                .await
        }
    }
    pub async fn update(&mut self, project: GeneratedProject) -> Result<()> {
        let next = stage_files(&project, self.directory.path())?;
        for old in &self.staged {
            if !next.contains(old) {
                let _ = std::fs::remove_file(self.directory.path().join(old));
            }
        }
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
    pub async fn diagnostics(&self) -> Result<Value> {
        let mut events = self.session.subscribe();
        tokio::time::timeout(Duration::from_secs(10),async{
   loop{let diagnostics=self.session.diagnostics();if diagnostics[&self.uri]["version"]==self.version{
    let entries=diagnostics[&self.uri]["diagnostics"].as_array().cloned().unwrap_or_default().into_iter().map(|d|{
     let line=d["range"]["start"]["line"].as_u64().unwrap_or(0)as usize+1;let column=d["range"]["start"]["character"].as_u64().unwrap_or(0)as usize+1;
     json!({"message":d["message"],"severity":d["severity"],"code":d["code"],"source":self.file.source_location(line,column),"generated":{"uri":self.uri,"line":line,"column":column},"diagnostic":d})
    }).collect::<Vec<_>>();return Ok(json!({"diagnostics":entries,"positionEncoding":self.session.encoding,"uri":self.uri,"version":self.version,"pid":self.session.pid()}));
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
        if !text[..start].ends_with('.') && !text[..start].ends_with("->") {
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
        items.sort_by_cached_key(|i| {
            i["sortText"]
                .as_str()
                .or(i["filterText"].as_str())
                .or(i["label"].as_str())
                .unwrap_or("")
                .trim()
                .to_string()
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
        wirecraft_core::store::atomic_write(&directory.join(&file.path), &data)?;
    }
    Ok(paths.into_iter().collect())
}

impl GeneratedAnalysis {
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
        for tuple in data.chunks_exact(5) {
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
        Ok(
            json!({"tokens":tokens,"positionEncoding":"utf-32","serverPositionEncoding":self.session.encoding,"version":self.version,"pid":self.session.pid(),"generatedBytes":self.file.content.len(),"generatedFiles":self.staged.len()}),
        )
    }
}
