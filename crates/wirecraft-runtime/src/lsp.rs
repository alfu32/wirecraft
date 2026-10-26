//! Supervised JSON-RPC language server session. No UI dependency.
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::{Mutex as AsyncMutex, broadcast, oneshot},
};
use wirecraft_core::{Error, Result};

struct Inner {
    child: AsyncMutex<Child>,
    input: AsyncMutex<ChildStdin>,
    pending: Mutex<BTreeMap<u64, oneshot::Sender<Result<Value>>>>,
    documents: Mutex<BTreeMap<String, i64>>,
    diagnostics: Mutex<BTreeMap<String, Value>>,
    next: AtomicU64,
    alive: AtomicBool,
    events: broadcast::Sender<Value>,
}
#[derive(Clone)]
pub struct LanguageSession {
    inner: Arc<Inner>,
    pub root: PathBuf,
    pub capabilities: Value,
    pub encoding: String,
    pub executable: String,
}
impl LanguageSession {
    pub async fn start(executable: &str, args: &[String], root: &Path) -> Result<Self> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(Error::new("invalid_input", "Workspace must be a directory"));
        }
        let mut cmd = Command::new(executable);
        cmd.args(args)
            .current_dir(&root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        let mut child = cmd.spawn().map_err(|e| {
            Error::new(
                "tool_unavailable",
                format!("Cannot start {executable}: {e}"),
            )
        })?;
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (events, _) = broadcast::channel(128);
        let inner = Arc::new(Inner {
            child: AsyncMutex::new(child),
            input: AsyncMutex::new(input),
            pending: Mutex::new(BTreeMap::new()),
            documents: Mutex::new(BTreeMap::new()),
            diagnostics: Mutex::new(BTreeMap::new()),
            next: AtomicU64::new(1),
            alive: AtomicBool::new(true),
            events,
        });
        let weak = Arc::downgrade(&inner);
        tokio::spawn(async move {
            read_loop(output, weak).await;
        });
        let mut session = Self {
            inner,
            root,
            capabilities: json!({}),
            encoding: "utf-16".into(),
            executable: executable.into(),
        };
        let uri = url::Url::from_directory_path(&session.root)
            .map_err(|_| Error::new("invalid_input", "Invalid workspace URI"))?
            .to_string();
        let initialized=session.request("initialize",json!({"processId":std::process::id(),"clientInfo":{"name":"wirecraft","version":env!("CARGO_PKG_VERSION")},"rootUri":uri,"workspaceFolders":[{"uri":uri,"name":session.root.file_name().unwrap_or_default().to_string_lossy()}],"capabilities":{"general":{"positionEncodings":["utf-16","utf-8","utf-32"]},"textDocument":{"synchronization":{"dynamicRegistration":false,"didSave":true},"publishDiagnostics":{"versionSupport":true},"completion":{"completionItem":{"snippetSupport":false}}},"workspace":{"configuration":true,"workspaceFolders":true}},"initializationOptions":{}})).await;
        let initialized = match initialized {
            Ok(v) => v,
            Err(e) => {
                session.shutdown().await;
                return Err(e);
            }
        };
        session.capabilities = initialized["capabilities"].clone();
        session.encoding = session.capabilities["positionEncoding"]
            .as_str()
            .unwrap_or("utf-16")
            .into();
        if !["utf-8", "utf-16", "utf-32"].contains(&session.encoding.as_str()) {
            session.shutdown().await;
            return Err(Error::new("unsupported", "Unsupported position encoding"));
        }
        session.notify("initialized", json!({})).await?;
        Ok(session)
    }
    pub fn alive(&self) -> bool {
        self.inner.alive.load(Ordering::SeqCst)
    }
    pub fn pid(&self) -> Option<u32> {
        self.inner.child.try_lock().ok().and_then(|c| c.id())
    }
    pub fn subscribe(&self) -> broadcast::Receiver<Value> {
        self.inner.events.subscribe()
    }
    pub async fn notify(&self, method: &str, params: Value) -> Result<()> {
        send(
            &self.inner,
            json!({"jsonrpc":"2.0","method":method,"params":params}),
        )
        .await
    }
    pub async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.inner.next.fetch_add(1, Ordering::SeqCst);
        self.request_id(id, method, params).await
    }
    pub async fn request_id(&self, id: u64, method: &str, params: Value) -> Result<Value> {
        if !self.alive() {
            return Err(Error::new("process_failed", "Language server has exited"));
        }
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.inner.pending.lock().unwrap();
            if pending.contains_key(&id) {
                return Err(Error::new("conflict", "Request ID already pending"));
            }
            pending.insert(id, tx);
        }
        if let Err(e) = send(
            &self.inner,
            json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}),
        )
        .await
        {
            self.inner.pending.lock().unwrap().remove(&id);
            return Err(e);
        }
        match tokio::time::timeout(Duration::from_secs(15), rx).await {
            Ok(Ok(r)) => r,
            Ok(Err(_)) => Err(Error::new(
                "process_failed",
                "Language server connection closed",
            )),
            Err(_) => {
                self.cancel(id).await?;
                Err(Error::new(
                    "timeout",
                    format!("Language server request '{method}' timed out"),
                ))
            }
        }
    }
    pub async fn cancel(&self, id: u64) -> Result<()> {
        self.notify("$/cancelRequest", json!({"id":id})).await?;
        if let Some(p) = self.inner.pending.lock().unwrap().remove(&id) {
            let _ = p.send(Err(Error::new(
                "cancelled",
                "Language server request cancelled",
            )));
        }
        Ok(())
    }
    pub async fn open(&self, uri: &str, language: &str, text: &str, version: i64) -> Result<()> {
        validate_uri(uri, &self.root)?;
        {
            let mut docs = self.inner.documents.lock().unwrap();
            if docs.contains_key(uri) {
                return Err(Error::new("conflict", "Document is already open"));
            }
            docs.insert(uri.into(), version);
        }
        self.notify(
            "textDocument/didOpen",
            json!({"textDocument":{"uri":uri,"languageId":language,"version":version,"text":text}}),
        )
        .await
    }
    pub async fn change(&self, uri: &str, text: &str, version: i64) -> Result<()> {
        {
            let mut docs = self.inner.documents.lock().unwrap();
            let old = docs
                .get_mut(uri)
                .ok_or_else(|| Error::new("not_found", "Document is not open"))?;
            if version <= *old {
                return Err(Error::new("conflict", "Document version must increase"));
            }
            *old = version;
        }
        self.inner.diagnostics.lock().unwrap().remove(uri);
        self.notify(
            "textDocument/didChange",
            json!({"textDocument":{"uri":uri,"version":version},"contentChanges":[{"text":text}]}),
        )
        .await
    }
    pub async fn close(&self, uri: &str) -> Result<()> {
        if self.inner.documents.lock().unwrap().remove(uri).is_none() {
            return Err(Error::new("not_found", "Document is not open"));
        }
        self.inner.diagnostics.lock().unwrap().remove(uri);
        self.notify("textDocument/didClose", json!({"textDocument":{"uri":uri}}))
            .await
    }
    pub fn diagnostics(&self) -> Value {
        json!(self.inner.diagnostics.lock().unwrap().clone())
    }
    pub async fn shutdown(&self) {
        if self.alive() {
            let _ = tokio::time::timeout(
                Duration::from_secs(2),
                self.request("shutdown", Value::Null),
            )
            .await;
            let _ = self.notify("exit", Value::Null).await;
        }
        let mut child = self.inner.child.lock().await;
        if tokio::time::timeout(Duration::from_secs(2), child.wait())
            .await
            .is_err()
        {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.inner.alive.store(false, Ordering::SeqCst);
        fail_pending(&self.inner, "Language server shut down");
    }
}
async fn send(inner: &Inner, value: Value) -> Result<()> {
    let data = serde_json::to_vec(&value)?;
    let mut input = inner.input.lock().await;
    input
        .write_all(format!("Content-Length: {}\r\n\r\n", data.len()).as_bytes())
        .await?;
    input.write_all(&data).await?;
    input.flush().await?;
    Ok(())
}
async fn read_loop(output: tokio::process::ChildStdout, weak: Weak<Inner>) {
    let mut reader = BufReader::new(output);
    loop {
        let mut length = None;
        let mut headers = 0;
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) | Err(_) => {
                    if let Some(inner) = weak.upgrade() {
                        inner.alive.store(false, Ordering::SeqCst);
                        fail_pending(&inner, "Language server exited");
                        let _ = inner.events.send(json!({"type":"lsp.exited"}));
                    }
                    return;
                }
                _ => {}
            }
            headers += line.len();
            if headers > 8192 {
                return;
            }
            if line.trim().is_empty() {
                break;
            }
            if let Some((key, value)) = line.split_once(':')
                && key.eq_ignore_ascii_case("Content-Length")
            {
                length = value.trim().parse::<usize>().ok();
            }
        }
        let Some(length) = length.filter(|n| *n <= 16 * 1024 * 1024) else {
            break;
        };
        let mut bytes = vec![0; length];
        if reader.read_exact(&mut bytes).await.is_err() {
            break;
        }
        let Ok(v) = serde_json::from_slice::<Value>(&bytes) else {
            break;
        };
        let Some(inner) = weak.upgrade() else {
            return;
        };
        if let Some(method) = v["method"].as_str() {
            if let Some(id) = v.get("id") {
                let result = match method {
                    "workspace/configuration" => Value::Array(
                        v["params"]["items"]
                            .as_array()
                            .map(|a| vec![json!({}); a.len()])
                            .unwrap_or_default(),
                    ),
                    "workspace/workspaceFolders" => Value::Null,
                    "window/workDoneProgress/create"
                    | "client/registerCapability"
                    | "client/unregisterCapability" => Value::Null,
                    _ => {
                        let _=send(&inner,json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Unsupported client method"}})).await;
                        continue;
                    }
                };
                let _ = send(&inner, json!({"jsonrpc":"2.0","id":id,"result":result})).await;
            } else if method == "textDocument/publishDiagnostics" {
                let params = &v["params"];
                if let Some(uri) = params["uri"].as_str() {
                    let current = inner.documents.lock().unwrap().get(uri).copied();
                    if current.is_some()
                        && params["version"]
                            .as_i64()
                            .is_none_or(|v| Some(v) == current)
                    {
                        inner
                            .diagnostics
                            .lock()
                            .unwrap()
                            .insert(uri.into(), params.clone());
                        let _ = inner
                            .events
                            .send(json!({"type":"lsp.diagnostics","data":params}));
                    }
                }
            } else {
                let _ = inner
                    .events
                    .send(json!({"type":"lsp.notification","method":method,"data":v["params"]}));
            }
        } else if let Some(id) = v["id"].as_u64()
            && let Some(p) = inner.pending.lock().unwrap().remove(&id)
        {
            let result = if let Some(error) = v.get("error") {
                Err(Error::new("lsp", error.to_string()))
            } else {
                Ok(v["result"].clone())
            };
            let _ = p.send(result);
        }
    }
    if let Some(inner) = weak.upgrade() {
        inner.alive.store(false, Ordering::SeqCst);
        fail_pending(&inner, "Invalid or closed language server stream");
    }
}
fn fail_pending(inner: &Inner, message: &str) {
    let pending = std::mem::take(&mut *inner.pending.lock().unwrap());
    for (_, p) in pending {
        let _ = p.send(Err(Error::new("process_failed", message)));
    }
}
fn validate_uri(uri: &str, root: &Path) -> Result<()> {
    let url = url::Url::parse(uri).map_err(|e| Error::new("invalid_input", e.to_string()))?;
    let path = url
        .to_file_path()
        .map_err(|_| Error::new("invalid_input", "Expected a file URI"))?;
    // Resolve the parent for unsaved/new files too. Reject escaping the workspace via symlinks.
    let parent = path
        .parent()
        .ok_or_else(|| Error::new("invalid_input", "File URI needs a parent"))?
        .canonicalize()?;
    if !parent.starts_with(root) {
        return Err(Error::new(
            "invalid_input",
            "Document is outside the language workspace",
        ));
    }
    Ok(())
}
/// Convert a UTF-8 byte offset to negotiated LSP coordinates (0 based).
pub fn position(text: &str, offset: usize, encoding: &str) -> Result<Value> {
    if offset > text.len() || !text.is_char_boundary(offset) {
        return Err(Error::new(
            "invalid_input",
            "Offset is not a UTF-8 character boundary",
        ));
    }
    let prefix = &text[..offset];
    let line = prefix.bytes().filter(|b| *b == b'\n').count();
    let column = prefix.rsplit('\n').next().unwrap_or("");
    let character = match encoding {
        "utf-8" => column.len(),
        "utf-16" => column.encode_utf16().count(),
        "utf-32" => column.chars().count(),
        _ => return Err(Error::new("unsupported", "Unknown position encoding")),
    };
    Ok(json!({"line":line,"character":character}))
}
