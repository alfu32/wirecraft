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
        let file = project.files.into_iter().next().ok_or_else(|| {
            Error::new("invalid_input", "Generated project has no translation unit")
        })?;
        let path = directory.path().join("network.c");
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
        };
        analysis.synchronize(None).await?;
        Ok(analysis)
    }
    async fn synchronize(&mut self, next: Option<VirtualFile>) -> Result<()> {
        if let Some(file) = next {
            self.file = file;
        }
        self.version += 1;
        let path = self.directory.path().join("network.c");
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
        let file = project
            .files
            .into_iter()
            .next()
            .ok_or_else(|| Error::new("invalid_input", "Generated project is empty"))?;
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
