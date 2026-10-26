use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};
use wirecraft_core::store;
use wirecraft_runtime::service::{Request, Snapshot};
#[derive(Clone)]
pub struct Client {
    pub sender: mpsc::Sender<Message>,
}
pub enum Message {
    Execute(Request),
    SaveUi(Value),
    Analyze {
        node_id: String,
        language: String,
        text: String,
        build: bool,
    },
    Shutdown,
}
pub enum Update {
    Snapshot(Box<Snapshot>),
    Result { command: String, value: Value },
    Error(String),
    Connected { endpoint: String, token: String },
}
impl Client {
    pub fn start(
        state_dir: PathBuf,
        project: Option<PathBuf>,
    ) -> (
        Self,
        async_channel::Receiver<Update>,
        std::thread::JoinHandle<()>,
    ) {
        let (sender, receiver) = mpsc::channel();
        let (updates, rx) = async_channel::unbounded();
        let worker = std::thread::Builder::new()
            .name("wirecraft-client".into())
            .spawn(move || {
                if let Err(e) = worker(receiver, updates.clone(), state_dir, project) {
                    let _ = updates.send_blocking(Update::Error(e.to_string()));
                }
            })
            .expect("client worker");
        (Self { sender }, rx, worker)
    }
    pub fn send(&self, command: &str, arguments: Value) {
        let _ = self
            .sender
            .send(Message::Execute(Request::new(command, arguments)));
    }
}
struct OwnedBackend {
    child: Child,
    client: reqwest::blocking::Client,
    endpoint: String,
    token: String,
}
impl OwnedBackend {
    fn call(&self, request: &Request) -> anyhow::Result<Value> {
        let value: Value = self
            .client
            .post(format!("{}/api", self.endpoint))
            .bearer_auth(&self.token)
            .json(request)
            .send()?
            .error_for_status()?
            .json()?;
        if value["ok"] != true {
            anyhow::bail!(
                "{}: {}",
                value["error"]["code"].as_str().unwrap_or("error"),
                value["error"]["message"]
                    .as_str()
                    .unwrap_or("Backend request failed")
            );
        }
        Ok(value["result"].clone())
    }
}
impl Drop for OwnedBackend {
    fn drop(&mut self) {
        let _ = self.call(&Request::new("system.shutdown", json!({})));
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn worker(
    rx: mpsc::Receiver<Message>,
    updates: async_channel::Sender<Update>,
    state_dir: PathBuf,
    project: Option<PathBuf>,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(&state_dir)?;
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(state_dir.join("session.lock"))?;
    lock.try_lock().map_err(|_|anyhow::anyhow!("Another Wirecraft desktop owns this state directory; choose --state-dir for an independent session"))?;

    let token = uuid_token();
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--backend", "serve", "--token", &token, "--recovery"])
        .arg(state_dir.join("recovery.json"))
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    if let Some(path) = project {
        command.arg("--project").arg(path);
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: only async-signal-safe syscalls are used between fork and exec.
        unsafe {
            command.pre_exec(|| {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getppid() == 1 {
                    return Err(std::io::Error::other("Desktop parent exited"));
                }
                Ok(())
            });
        }
    }
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
        let _ = ready_tx.send(result);
    });
    let ready = match ready_rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(line)) => line,
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("Backend did not report readiness within 10 seconds");
        }
    };
    let discovery: Value = match serde_json::from_str(&ready) {
        Ok(v) => v,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e.into());
        }
    };
    let mut backend = OwnedBackend {
        child,
        client: reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(25))
            .build()?,
        endpoint: discovery["endpoint"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing backend endpoint"))?
            .into(),
        token,
    };
    updates.send_blocking(Update::Connected {
        endpoint: backend.endpoint.clone(),
        token: backend.token.clone(),
    })?;
    let mut revision = None;
    let mut intelligence = Intelligence::default();
    loop {
        match rx.recv_timeout(Duration::from_millis(300)) {
            Ok(Message::Execute(request)) => {
                let command = request.command.clone();
                if command == "lsp.stop" {
                    intelligence.started = false;
                    intelligence.versions.clear();
                }
                match backend.call(&request) {
                    Ok(value) => {
                        updates.send_blocking(Update::Result { command, value })?;
                        revision = None;
                    }
                    Err(e) => {
                        updates.send_blocking(Update::Error(e.to_string()))?;
                    }
                }
            }
            Ok(Message::Analyze {
                node_id,
                language,
                text,
                build,
            }) => {
                match intelligence.analyze(&backend, &state_dir, &node_id, &language, &text, build)
                {
                    Ok(value) => {
                        updates.send_blocking(Update::Result {
                            command: if build {
                                "jobs.start".into()
                            } else {
                                "lsp.diagnostics".into()
                            },
                            value,
                        })?;
                    }
                    Err(e) => {
                        updates.send_blocking(Update::Error(e.to_string()))?;
                    }
                }
            }
            Ok(Message::SaveUi(value)) => {
                if let Err(e) = store::atomic_write(
                    &state_dir.join("ui.json"),
                    serde_json::to_string_pretty(&value)?.as_bytes(),
                ) {
                    updates.send_blocking(Update::Error(format!("Session state: {e}")))?;
                }
            }
            Ok(Message::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if backend.child.try_wait()?.is_some() {
            anyhow::bail!(
                "Owned backend exited. Relaunch Wirecraft to restore its recovery snapshot."
            );
        }
        let status = backend.call(&Request::new("session.status", json!({})))?;
        if revision != status["revision"].as_u64() {
            let value = backend.call(&Request::new("session.snapshot", json!({})))?;
            let snapshot: Snapshot = serde_json::from_value(value)?;
            revision = Some(snapshot.revision);
            updates.send_blocking(Update::Snapshot(Box::new(snapshot)))?;
        }
    }
    Ok(())
}
fn uuid_token() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Default)]
struct Intelligence {
    started: bool,
    versions: std::collections::BTreeMap<String, i64>,
}
impl Intelligence {
    fn analyze(
        &mut self,
        backend: &OwnedBackend,
        state_dir: &std::path::Path,
        node_id: &str,
        language: &str,
        text: &str,
        build: bool,
    ) -> anyhow::Result<Value> {
        let root = state_dir.join("intelligence");
        std::fs::create_dir_all(&root)?;
        let root = root.canonicalize()?;
        let safe: String = node_id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let extension = if language == "cpp" { "cpp" } else { "c" };
        let path = root.join(format!("entity_{safe}.{extension}"));
        store::atomic_write(&path, text.as_bytes())?;
        let uri = url_file(&path)?;
        if build {
            let executable = std::env::var("WIRECRAFT_CC").unwrap_or_else(|_| {
                if language == "cpp" {
                    "clang++".into()
                } else {
                    "clang".into()
                }
            });
            return backend.call(&Request::new("jobs.start",json!({"executable":executable,"args":["-fsyntax-only",if language=="cpp"{"-std=c++20"}else{"-std=c17"},path],"cwd":root})));
        }
        if !self.started {
            let executable = std::env::var("WIRECRAFT_CLANGD").unwrap_or_else(|_| "clangd".into());
            backend.call(&Request::new(
                "lsp.start",
                json!({"executable":executable,"root":root,"args":["--background-index=false"]}),
            ))?;
            self.started = true;
        }
        let old = self.versions.get(&uri).copied();
        let version = old.unwrap_or(0) + 1;
        backend.call(&Request::new(
            if old.is_some() {
                "lsp.change"
            } else {
                "lsp.open"
            },
            json!({"uri":uri,"languageId":language,"version":version,"text":text}),
        ))?;
        self.versions.insert(uri.clone(), version);
        let until = Instant::now() + Duration::from_secs(4);
        loop {
            let diagnostics = backend.call(&Request::new("lsp.diagnostics", json!({})))?;
            if diagnostics[&uri]["version"] == version {
                return Ok(
                    json!({"document":node_id,"languageId":language,"projection":"standalone selected source section","diagnostics":diagnostics[&uri]}),
                );
            }
            if Instant::now() > until {
                anyhow::bail!("clangd has not published diagnostics yet; use Refresh diagnostics");
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    }
}
fn url_file(path: &std::path::Path) -> anyhow::Result<String> {
    url::Url::from_file_path(path)
        .map(|u| u.to_string())
        .map_err(|_| anyhow::anyhow!("Invalid source path"))
}
