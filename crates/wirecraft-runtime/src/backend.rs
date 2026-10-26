use crate::{
    jobs::Jobs,
    lsp::LanguageSession,
    mcp,
    service::{Request, Service, required},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use futures::StreamExt;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    net::TcpListener,
    sync::{Mutex as AsyncMutex, broadcast, watch},
};
use wirecraft_core::{Error, Result};

#[derive(Clone)]
pub struct Application {
    pub service: Arc<Mutex<Service>>,
    pub events: broadcast::Sender<Value>,
    pub jobs: Jobs,
    lsp: Arc<AsyncMutex<Option<LanguageSession>>>,
    generated: Arc<AsyncMutex<Option<crate::generated_analysis::GeneratedAnalysis>>>,
    lifecycle: Arc<AsyncMutex<()>>,
    pub token: String,
    stop: watch::Sender<bool>,
    recovery: Option<PathBuf>,
    revision: Arc<AtomicU64>,
    settings: Arc<Mutex<(crate::config::Settings, Option<PathBuf>)>>,
    mcp_enabled: Arc<AtomicBool>,
    access_log: Arc<Mutex<std::collections::VecDeque<Value>>>,
}
impl Application {
    pub fn new(service: Service, token: String, recovery: Option<PathBuf>) -> Self {
        let (events, _) = broadcast::channel(256);
        let (stop, _) = watch::channel(false);
        let revision = Arc::new(AtomicU64::new(service.revision));
        Self {
            service: Arc::new(Mutex::new(service)),
            revision,
            jobs: Jobs::new(events.clone()),
            events,
            lsp: Arc::new(AsyncMutex::new(None)),
            generated: Arc::new(AsyncMutex::new(None)),
            lifecycle: Arc::new(AsyncMutex::new(())),
            token,
            stop,
            recovery,
            settings: Arc::new(Mutex::new((crate::config::Settings::default(), None))),
            mcp_enabled: Arc::new(AtomicBool::new(true)),
            access_log: Arc::new(Mutex::new(std::collections::VecDeque::new())),
        }
    }
    pub fn with_settings(self, path: Option<PathBuf>) -> Result<Self> {
        let settings = if let Some(path) = &path {
            crate::config::Settings::load(path)?
        } else {
            crate::config::Settings::default()
        };
        *self
            .settings
            .lock()
            .map_err(|_| Error::new("internal", "Settings lock poisoned"))? = (settings, path);
        Ok(self)
    }
    fn log_access(&self, method: &str, status: u16, elapsed: u128) {
        if let Ok(mut log) = self.access_log.lock() {
            if log.len() == 256 {
                log.pop_front();
            }
            log.push_back(json!({"at":wirecraft_core::model::timestamp(),"method":method,"status":status,"elapsedMs":elapsed}));
        }
    }
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::SeqCst)
    }
    pub async fn execute(&self, r: Request) -> Result<Value> {
        if *self.stop.borrow() && r.command != "system.shutdown" {
            return Err(Error::new("shutting_down", "Backend is shutting down"));
        }
        let a = &r.arguments;
        match r.command.as_str() {
            "settings.get" => {
                return Ok(serde_json::to_value(
                    &self
                        .settings
                        .lock()
                        .map_err(|_| Error::new("internal", "Settings lock poisoned"))?
                        .0,
                )?);
            }
            "settings.replace" => {
                let settings: crate::config::Settings = serde_json::from_value(a.clone())?;
                settings.validate()?;
                let state = self.settings.clone();
                return tokio::task::spawn_blocking(move || {
                    let mut state = state
                        .lock()
                        .map_err(|_| Error::new("internal", "Settings lock poisoned"))?;
                    if let Some(path) = &state.1 {
                        settings.save(path)?;
                    }
                    state.0 = settings;
                    Ok(serde_json::to_value(&state.0)?)
                })
                .await
                .map_err(|e| Error::new("internal", e.to_string()))?;
            }
            "mcp.status" => {
                return Ok(
                    json!({"enabled":self.mcp_enabled.load(Ordering::SeqCst),"log":self.access_log.lock().map_err(|_|Error::new("internal","Access log lock poisoned"))?.clone()}),
                );
            }
            "mcp.set_enabled" => {
                let enabled = a["enabled"]
                    .as_bool()
                    .ok_or_else(|| Error::new("invalid_input", "enabled must be boolean"))?;
                self.mcp_enabled.store(enabled, Ordering::SeqCst);
                return Ok(json!({"enabled":enabled}));
            }
            "system.health" => {
                return Ok(json!({"name":"Wirecraft","status":"ok","revision":self.revision()}));
            }
            "system.shutdown" => {
                let _ = self.stop.send(true);
                return Ok(json!({"shuttingDown":true}));
            }
            "editor.analyze_generated" => {
                let service = self.service.clone();
                let root = a["nodeId"].as_str().map(str::to_string);
                let (project, revision) = tokio::task::spawn_blocking(move || {
                    let s = service
                        .lock()
                        .map_err(|_| Error::new("internal", "Project lock poisoned"))?;
                    Ok::<_, Error>((
                        wirecraft_core::c17::C17::generate(
                            &s.document,
                            root.as_deref().unwrap_or(&s.document.root_node_id),
                        )?,
                        s.revision,
                    ))
                })
                .await
                .map_err(|e| Error::new("internal", e.to_string()))??;
                let _lifecycle = self.lifecycle.lock().await;
                let mut generated = self.generated.lock().await;
                if let Some(session) = generated.as_mut() {
                    session.update(project).await?;
                } else {
                    let args: Vec<String> = serde_json::from_value(
                        a.get("args")
                            .cloned()
                            .unwrap_or(json!(["--background-index=false"])),
                    )?;
                    *generated = Some(
                        crate::generated_analysis::GeneratedAnalysis::start(
                            project,
                            a["executable"].as_str().unwrap_or("clangd"),
                            &args,
                        )
                        .await?,
                    );
                }
                let mut result = generated.as_ref().unwrap().diagnostics().await?;
                if self.revision() != revision {
                    return Err(Error::new(
                        "stale_result",
                        "Model changed during generated-source analysis",
                    ));
                }
                result["revision"] = json!(revision);
                return Ok(result);
            }
            "editor.generated_diagnostics" => {
                let generated = self.generated.lock().await;
                let session = generated.as_ref().ok_or_else(|| {
                    Error::new("unavailable", "Generated-source analysis has not started")
                })?;
                return session.diagnostics().await;
            }
            "editor.stop_generated" => {
                let _lifecycle = self.lifecycle.lock().await;
                if let Some(s) = self.generated.lock().await.take() {
                    s.shutdown().await;
                }
                return Ok(json!({"stopped":true}));
            }
            "lsp.start" => {
                let _lifecycle = self.lifecycle.lock().await;
                if let Some(old) = self.lsp.lock().await.take() {
                    old.shutdown().await;
                }
                let args: Vec<String> =
                    serde_json::from_value(a.get("args").cloned().unwrap_or(json!([])))?;
                let session = LanguageSession::start(
                    a["executable"].as_str().unwrap_or("clangd"),
                    &args,
                    &PathBuf::from(required(a, "root")?),
                )
                .await?;
                let result = json!({"capabilities":session.capabilities,"positionEncoding":session.encoding,"pid":session.pid(),"executable":session.executable});
                let mut rx = session.subscribe();
                let events = self.events.clone();
                tokio::spawn(async move {
                    while let Ok(event) = rx.recv().await {
                        let _ = events.send(event);
                    }
                });
                *self.lsp.lock().await = Some(session);
                return Ok(result);
            }
            "lsp.stop" => {
                let _lifecycle = self.lifecycle.lock().await;
                if let Some(s) = self.lsp.lock().await.take() {
                    s.shutdown().await;
                }
                return Ok(json!({"stopped":true}));
            }
            "jobs.start" => {
                let args: Vec<String> =
                    serde_json::from_value(a.get("args").cloned().unwrap_or(json!([])))?;
                return self
                    .jobs
                    .start(
                        required(a, "executable")?,
                        &args,
                        &PathBuf::from(required(a, "cwd")?),
                        Duration::from_secs(
                            a["timeoutSeconds"].as_u64().unwrap_or(120).clamp(1, 3600),
                        ),
                    )
                    .await;
            }
            "jobs.list" => return Ok(self.jobs.list().await),
            "jobs.get" => return self.jobs.get(required(a, "id")?).await,
            "jobs.cancel" => return self.jobs.cancel(required(a, "id")?).await,
            command if command.starts_with("lsp.") => {
                let session =
                    self.lsp.lock().await.clone().ok_or_else(|| {
                        Error::new("unavailable", "Start a language server first")
                    })?;
                return match command {
                    "lsp.status" => Ok(
                        json!({"alive":session.alive(),"pid":session.pid(),"capabilities":session.capabilities,"positionEncoding":session.encoding}),
                    ),
                    "lsp.open" => {
                        session
                            .open(
                                required(a, "uri")?,
                                required(a, "languageId")?,
                                required_text(a)?,
                                a["version"].as_i64().unwrap_or(1),
                            )
                            .await?;
                        Ok(json!({"opened":true}))
                    }
                    "lsp.change" => {
                        session
                            .change(
                                required(a, "uri")?,
                                required_text(a)?,
                                a["version"].as_i64().ok_or_else(|| {
                                    Error::new("invalid_input", "version is required")
                                })?,
                            )
                            .await?;
                        Ok(json!({"changed":true}))
                    }
                    "lsp.close" => {
                        session.close(required(a, "uri")?).await?;
                        Ok(json!({"closed":true}))
                    }
                    "lsp.diagnostics" => Ok(session.diagnostics()),
                    "lsp.request" => {
                        let method = required(a, "method")?;
                        if !method.starts_with("textDocument/") && !method.starts_with("workspace/")
                        {
                            return Err(Error::new(
                                "invalid_input",
                                "Only document/workspace requests are exposed",
                            ));
                        }
                        if let Some(id) = a["requestId"].as_u64() {
                            session.request_id(id, method, a["params"].clone()).await
                        } else {
                            session.request(method, a["params"].clone()).await
                        }
                    }
                    "lsp.cancel" => {
                        session
                            .cancel(a["requestId"].as_u64().ok_or_else(|| {
                                Error::new("invalid_input", "requestId is required")
                            })?)
                            .await?;
                        Ok(json!({"cancelled":true}))
                    }
                    _ => Err(Error::new("not_found", "Unknown language command")),
                };
            }
            _ => {}
        }
        let service = self.service.clone();
        let events = self.events.clone();
        let recovery = self.recovery.clone();
        let revision = self.revision.clone();
        tokio::task::spawn_blocking(move || {
            let mut s = service
                .lock()
                .map_err(|_| Error::new("internal", "Project lock is poisoned"))?;
            let before = s.revision;
            let before_dirty = s.dirty();
            let before_path = s.path.clone();
            let mut result = s.execute(r);
            revision.store(s.revision, Ordering::SeqCst);
            if before != s.revision {
                let _ = events.send(json!({"type":"document.changed","revision":s.revision}));
            }
            let session_changed = before_dirty != s.dirty() || before_path != s.path;
            if session_changed {
                let _ = events.send(json!({"type":"session.changed","revision":s.revision}));
            }
            if (before != s.revision || session_changed)
                && let Some(path) = recovery
                && let Err(error) = s.write_recovery(&path)
            {
                let _ = events.send(json!({"type":"recovery.failed","error":error}));
                match &mut result {
                    Ok(value) => {
                        value["_wirecraftWarnings"] =
                            json!([{"code":"recovery_failed","message":error.message}])
                    }
                    Err(original) => original
                        .message
                        .push_str(&format!("; recovery also failed: {}", error.message)),
                }
            }
            result
        })
        .await
        .map_err(|e| Error::new("internal", e.to_string()))?
    }
    pub async fn shutdown(&self) {
        let _lifecycle = self.lifecycle.lock().await;
        if let Some(session) = self.lsp.lock().await.take() {
            session.shutdown().await;
        }
        if let Some(generated) = self.generated.lock().await.take() {
            generated.shutdown().await;
        }
        self.jobs.shutdown().await;
    }
    pub fn router(&self) -> Router {
        Router::new()
            .route("/health", get(health))
            .route("/api", post(api))
            .route("/events", get(events))
            .route("/mcp", post(mcp_http))
            .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
            .with_state(self.clone())
    }
    pub async fn serve(&self, listener: TcpListener) -> std::io::Result<()> {
        let mut stop = self.stop.subscribe();
        let shutdown = self.clone();
        let result = axum::serve(listener, self.router())
            .with_graceful_shutdown(async move {
                if !*stop.borrow() {
                    let _ = stop.changed().await;
                }
                shutdown.shutdown().await;
            })
            .await;
        self.shutdown().await;
        result
    }
}
fn required_text(a: &Value) -> Result<&str> {
    a["text"]
        .as_str()
        .ok_or_else(|| Error::new("invalid_input", "text must be a string"))
}
fn authorized(app: &Application, headers: &HeaderMap) -> bool {
    !headers.contains_key("origin")
        && headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .is_some_and(|h| h.strip_prefix("Bearer ") == Some(app.token.as_str()))
}
async fn health(State(app): State<Application>) -> Json<Value> {
    Json(
        json!({"name":"Wirecraft","version":env!("CARGO_PKG_VERSION"),"status":"ok","revision":app.revision()}),
    )
}
async fn api(
    State(app): State<Application>,
    headers: HeaderMap,
    request: std::result::Result<Json<Request>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if !authorized(&app, &headers) {
        return (StatusCode::UNAUTHORIZED,Json(json!({"ok":false,"error":{"code":"unauthorized","message":"Local bearer token required; browser origins are rejected"}}))).into_response();
    }
    let r =
        match request {
            Ok(Json(r)) => r,
            Err(e) => return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok":false,"error":{"code":"invalid_input","message":e.body_text()}})),
            )
                .into_response(),
        };
    let result = app.execute(r).await;
    let rev = app.revision();
    match result {
        Ok(result) => Json(json!({"ok":true,"result":result,"revision":rev})).into_response(),
        Err(error) => Json(json!({"ok":false,"error":error,"revision":rev})).into_response(),
    }
}
async fn mcp_http(
    State(app): State<Application>,
    headers: HeaderMap,
    body: std::result::Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if !authorized(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if !app.mcp_enabled.load(Ordering::SeqCst) {
        app.log_access("POST /mcp", 503, 0);
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(
                json!({"error":{"code":"unavailable","message":"MCP is disabled by this session"}}),
            ),
        )
            .into_response();
    }
    let started = std::time::Instant::now();
    let value =
        match body {
            Ok(Json(v)) => v,
            Err(_) => return Json(
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}}),
            )
            .into_response(),
        };
    let response = match mcp::handle(&app, value).await {
        Some(v) => Json(v).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    };
    app.log_access(
        "POST /mcp",
        response.status().as_u16(),
        started.elapsed().as_millis(),
    );
    response
}
async fn events(State(app): State<Application>, headers: HeaderMap) -> Response {
    if !authorized(&app, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let rx = app.events.subscribe();
    let mut stop = app.stop.subscribe();
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx).map(|v| {
        Ok::<_, std::convert::Infallible>(
            Event::default()
                .json_data(v.unwrap_or_else(|_| json!({"type":"resync"})))
                .unwrap(),
        )
    });
    let stream = stream.take_until(async move {
        if !*stop.borrow() {
            let _ = stop.changed().await;
        }
    });
    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}
