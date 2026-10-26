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
