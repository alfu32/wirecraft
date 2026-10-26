use serde_json::{Value, json};
use std::time::Duration;
use wirecraft_core::Document;
use wirecraft_runtime::{
    backend::Application,
    lsp::{LanguageSession, position},
    mcp,
    service::{Request, Service},
};
fn service() -> Service {
    Service::new(Document::new("fixture"), None)
}
#[test]
fn transactions_conflict_undo_save_and_unsaved_protection() {
    let mut s = service();
    let result = s
        .execute(Request::new(
            "threadwork.create_entity",
            json!({"name":"reader"}),
        ))
        .unwrap();
    let id = result["entity"]["id"].as_str().unwrap();
    let rev = s.revision;
    assert!(
        s.execute(Request::new(
            "threadwork.update_entity",
            json!({"nodeId":id,"patch":{"layout":{"width":-2}}})
        ))
        .is_err()
    );
    assert_eq!(s.revision, rev);
    assert_eq!(s.document.nodes[id].layout.width, 200.);
    let mut stale = Request::new("threadwork.delete_entity", json!({"nodeId":id}));
    stale.expected_revision = Some(0);
    assert_eq!(s.execute(stale).unwrap_err().code, "conflict");
    assert_eq!(
        s.execute(Request::new("file.new", json!({})))
            .unwrap_err()
            .code,
        "unsaved_changes"
    );
    s.execute(Request::new("edit.undo", json!({}))).unwrap();
    assert_eq!(s.document.nodes.len(), 1);
    assert!(!s.dirty());
    s.execute(Request::new("edit.redo", json!({}))).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("flow.orch");
    s.execute(Request::new("file.save", json!({"path":path})))
        .unwrap();
    assert!(!s.dirty());
    assert_eq!(wirecraft_core::store::load(&path).unwrap().nodes.len(), 2);
}
#[test]
fn user_merge_and_partial_text_updates() {
    let mut s = service();
    for name in ["old", "new"] {
        s.execute(Request::new(
            "threadwork.register_user",
            json!({"user":{"emailAddress":"user@example.test","source":"local","fullName":name}}),
        ))
        .unwrap();
    }
    assert_eq!(s.document.users.len(), 1);
    assert_eq!(s.document.users[0]["fullName"], "new");
    s.execute(Request::new(
        "threadwork.update_entity",
        json!({"nodeId":"root","patch":{"text":{"declaration":"int value;"}}}),
    ))
    .unwrap();
    s.execute(Request::new(
        "threadwork.update_entity",
        json!({"nodeId":"root","patch":{"text":{"specification":"kept"}}}),
    ))
    .unwrap();
    assert_eq!(s.document.nodes["root"].text.source, "int value;");
}
#[tokio::test]
async fn real_http_mcp_client_and_shared_revision() {
    let app = Application::new(service(), "temporary-test-token".into(), None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server_app = app.clone();
    let server = tokio::spawn(async move {
        server_app.serve(listener).await.unwrap();
    });
    let client = reqwest::Client::new();
    assert_eq!(
        client
            .post(format!("{url}/api"))
            .json(&Request::new("session.snapshot", json!({})))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        client
            .post(format!("{url}/api"))
            .bearer_auth("temporary-test-token")
            .header("Origin", "https://example.test")
            .json(&Request::new("session.snapshot", json!({})))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let rpc = |id, method: &str, params: Value| json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
    let initialize: Value = client
        .post(format!("{url}/mcp"))
        .bearer_auth("temporary-test-token")
        .json(&rpc(
            1,
            "initialize",
            json!({"protocolVersion":"2025-11-25"}),
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(initialize["result"]["protocolVersion"], "2025-11-25");
    let tools: Value = client
        .post(format!("{url}/mcp"))
        .bearer_auth("temporary-test-token")
        .json(&rpc(2, "tools/list", json!({})))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 37);
    let created: Value = client
        .post(format!("{url}/mcp"))
        .bearer_auth("temporary-test-token")
        .json(&rpc(
            3,
            "tools/call",
            json!({"name":"threadwork.create_entity","arguments":{"name":"from MCP"}}),
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(created["result"]["isError"], false);
    let report: Value = client
        .post(format!("{url}/mcp"))
        .bearer_auth("temporary-test-token")
        .json(&rpc(
            5,
            "tools/call",
            json!({"name":"wirecraft.analyze_network","arguments":{}}),
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        report["result"]["structuredContent"]["counts"]["entities"],
        2
    );
    assert!(report["result"]["structuredContent"]["graphs"]["principal"].is_object());
    let prompts = mcp::handle(&app, rpc(6, "prompts/list", json!({})))
        .await
        .unwrap();
    assert_eq!(prompts["result"]["prompts"].as_array().unwrap().len(), 2);
    let resources = mcp::handle(&app, rpc(7, "resources/list", json!({})))
        .await
        .unwrap();
    assert_eq!(
        resources["result"]["resources"].as_array().unwrap().len(),
        5
    );
    let resource = mcp::handle(
        &app,
        rpc(8, "resources/read", json!({"uri":"threadwork://design"})),
    )
    .await
    .unwrap();
    let design: Value =
        serde_json::from_str(resource["result"]["contents"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(design["nodes"].as_object().unwrap().len(), 2);
    let snapshot: Value = client
        .post(format!("{url}/api"))
        .bearer_auth("temporary-test-token")
        .json(&Request::new("session.snapshot", json!({})))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(snapshot["revision"], 1);
    assert_eq!(
        snapshot["result"]["document"]["nodes"]
            .as_object()
            .unwrap()
            .len(),
        2
    );
    let notification = client
        .post(format!("{url}/mcp"))
        .bearer_auth("temporary-test-token")
        .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
        .send()
        .await
        .unwrap();
    assert_eq!(notification.status(), 202);
    let malformed: Value = client
        .post(format!("{url}/mcp"))
        .bearer_auth("temporary-test-token")
        .header("Content-Type", "application/json")
        .body("{")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(malformed["error"]["code"], -32700);
    let unknown = mcp::handle(&app, rpc(4, "tools/call", json!({"name":"missing"})))
        .await
        .unwrap();
    assert_eq!(unknown["result"]["isError"], true);
    app.execute(Request::new("system.shutdown", json!({})))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(4), server)
        .await
        .unwrap()
        .unwrap();
}
#[test]
fn unicode_lsp_positions() {
    let text = "first\na😀éz";
    let off = "first\na😀".len();
    assert_eq!(
        position(text, off, "utf-16").unwrap(),
        json!({"line":1,"character":3})
    );
    assert_eq!(position(text, off, "utf-8").unwrap()["character"], 5);
    assert_eq!(position(text, off, "utf-32").unwrap()["character"], 2);
    assert!(position(text, 9, "utf-16").is_err());
}
#[tokio::test]
async fn real_clangd_sync_diagnostics_requests_restart_and_cleanup() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sample.c");
    std::fs::write(&path, "int main(void) { return missing; }\n").unwrap();
    let uri = url::Url::from_file_path(&path).unwrap().to_string();
    let session =
        LanguageSession::start("clangd", &["--background-index=false".into()], temp.path())
            .await
            .expect("clangd is required for this test");
    let pid = session.pid().unwrap();
    assert!(session.alive());
    session
        .open(&uri, "c", "int main(void) { return missing; }\n", 1)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if session.diagnostics()[&uri]["diagnostics"]
                .as_array()
                .is_some_and(|a| !a.is_empty())
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    let symbols = session
        .request(
            "textDocument/documentSymbol",
            json!({"textDocument":{"uri":uri}}),
        )
        .await
        .unwrap();
    assert!(!symbols.as_array().unwrap().is_empty());
    let completions = session
        .request(
            "textDocument/completion",
            json!({"textDocument":{"uri":uri},"position":{"line":0,"character":4}}),
        )
        .await
        .unwrap();
    assert!(completions.is_array() || completions.is_object() || completions.is_null());
    session
        .change(&uri, "int main(void) { return 0; }\n", 2)
        .await
        .unwrap();
    assert!(session.change(&uri, "", 2).await.is_err());
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let d = session.diagnostics();
            if d[&uri]["version"] == 2
                && d[&uri]["diagnostics"]
                    .as_array()
                    .is_some_and(|a| a.is_empty())
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    session.close(&uri).await.unwrap();
    session.shutdown().await;
    assert!(!session.alive());
    #[cfg(target_os = "linux")]
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    let restarted = LanguageSession::start("clangd", &[], temp.path())
        .await
        .unwrap();
    restarted.shutdown().await;
    assert_eq!(
        LanguageSession::start("wirecraft-missing-language-server", &[], temp.path())
            .await
            .err()
            .unwrap()
            .code,
        "tool_unavailable"
    );
}
#[tokio::test]
async fn build_jobs_failure_cancellation_and_timeout() {
    let app = Application::new(service(), "test".into(), None);
    let temp = tempfile::tempdir().unwrap();
    let started = app
        .jobs
        .start(
            "sh",
            &["-c".into(), "printf out; printf err >&2; exit 7".into()],
            temp.path(),
            Duration::from_secs(5),
        )
        .await
        .unwrap();
    let id = started["id"].as_str().unwrap();
    let done = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let s = app.jobs.get(id).await.unwrap();
            if s["status"] != "running" {
                break s;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(done["exitCode"], 7);
    assert_eq!(done["stdout"], "out");
    assert_eq!(done["stderr"], "err");
    let job = app
        .jobs
        .start(
            "sleep",
            &["30".into()],
            temp.path(),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    let id = job["id"].as_str().unwrap();
    app.jobs.cancel(id).await.unwrap();
    app.shutdown().await;
    assert_eq!(app.jobs.get(id).await.unwrap()["status"], "cancelled");
    let job = app
        .jobs
        .start(
            "sleep",
            &["30".into()],
            temp.path(),
            Duration::from_millis(30),
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(
        app.jobs.get(job["id"].as_str().unwrap()).await.unwrap()["status"],
        "timed_out"
    );
    assert!(
        app.jobs
            .start(
                "wirecraft-missing-compiler",
                &[],
                temp.path(),
                Duration::from_secs(1)
            )
            .await
            .is_err()
    );
}

#[test]
fn recovery_preserves_unsaved_state_path_and_draft_save_failure() {
    let temp = tempfile::tempdir().unwrap();
    let recovery = temp.path().join("recovery.json");
    let mut s = service();
    s.execute(Request::new(
        "threadwork.create_entity",
        json!({"name":"unsaved"}),
    ))
    .unwrap();
    s.write_recovery(&recovery).unwrap();
    let restored = Service::read_recovery(&recovery).unwrap();
    assert_eq!(restored.document.nodes.len(), 2);
    assert!(restored.dirty());
    assert!(restored.path.is_none());
    let path = temp.path().join("saved.orch");
    s.execute(Request::new("file.save", json!({"path":path})))
        .unwrap();
    s.write_recovery(&recovery).unwrap();
    let restored = Service::read_recovery(&recovery).unwrap();
    assert!(!restored.dirty());
    assert_eq!(restored.path.as_deref(), Some(path.as_path()));
    let before = std::fs::read(&path).unwrap();
    assert!(
        s.execute(Request::new(
            "file.commit_and_save",
            json!({"path":path,"nodeId":"root","patch":{"layout":{"height":-1}}})
        ))
        .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
#[test]
fn stdio_client_initializes_lists_calls_and_stops() {
    use std::io::Write;
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_wirecraft"))
        .arg("mcp-stdio")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"threadwork.validate_design","arguments":{}}}),
    ];
    let mut input = child.stdin.take().unwrap();
    for request in requests {
        writeln!(input, "{request}").unwrap();
    }
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let messages: Vec<Value> = text
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[1]["result"]["tools"].as_array().unwrap().len(), 37);
    assert_eq!(messages[2]["result"]["structuredContent"]["valid"], true);
}

#[test]
fn inverse_positions_reject_surrogate_and_utf8_splits() {
    use wirecraft_runtime::lsp::offset;
    let text = "a\r\nb😀é";
    for encoding in ["utf-8", "utf-16", "utf-32"] {
        let byte = "a\r\nb😀".len();
        let p = position(text, byte, encoding).unwrap();
        assert_eq!(
            offset(
                text,
                p["line"].as_u64().unwrap() as usize,
                p["character"].as_u64().unwrap() as usize,
                encoding
            )
            .unwrap(),
            byte
        );
    }
    assert!(offset(text, 1, 2, "utf-16").is_err());
    assert!(offset(text, 1, 2, "utf-8").is_err());
    assert!(offset(text, 9, 0, "utf-16").is_err());
}
#[tokio::test]
async fn lsp_cancellation_stale_results_and_abrupt_crash() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("peer.c");
    std::fs::write(&path, "").unwrap();
    let uri = url::Url::from_file_path(&path).unwrap().to_string();
    let peer = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lsp_peer.py");
    let session = LanguageSession::start("python3", &[peer.to_string_lossy().into()], temp.path())
        .await
        .unwrap();
    assert_eq!(session.encoding, "utf-8");
    session.open(&uri, "c", "int a;", 1).await.unwrap();
    let mut events = session.subscribe();
    let pending = session.clone();
    let task = tokio::spawn(async move {
        pending
            .request_id(900, "workspace/symbol", json!({"query":"slow"}))
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let event = events.recv().await.unwrap();
            if event["method"] == "fixture/pending" && event["data"]["id"] == 900 {
                break;
            }
        }
    })
    .await
    .unwrap();
    session.cancel(900).await.unwrap();
    assert_eq!(task.await.unwrap().unwrap_err().code, "cancelled");
    let pending = session.clone();
    let document = uri.clone();
    let task = tokio::spawn(async move {
        pending
            .request_id(
                901,
                "textDocument/documentSymbol",
                json!({"textDocument":{"uri":document}}),
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let event = events.recv().await.unwrap();
            if event["method"] == "fixture/pending" && event["data"]["id"] == 901 {
                break;
            }
        }
    })
    .await
    .unwrap();
    session.change(&uri, "int b;", 2).await.unwrap();
    assert_eq!(task.await.unwrap().unwrap_err().code, "stale_result");
    let until = std::time::Instant::now() + Duration::from_secs(2);
    while session.diagnostics()[&uri]["version"] != 2 {
        assert!(std::time::Instant::now() < until);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        session.diagnostics()[&uri]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let pending = session.clone();
    let task = tokio::spawn(async move {
        pending
            .request_id(902, "workspace/symbol", json!({"query":"aborted"}))
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let event = events.recv().await.unwrap();
            if event["method"] == "fixture/pending" && event["data"]["id"] == 902 {
                break;
            }
        }
    })
    .await
    .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let event = events.recv().await.unwrap();
            if event["method"] == "fixture/cancelled" && event["data"]["id"] == 902 {
                break;
            }
        }
    })
    .await
    .unwrap();
    session.shutdown().await;
    let crashed = LanguageSession::start("clangd", &[], temp.path())
        .await
        .unwrap();
    #[cfg(unix)]
    {
        let pid = crashed.pid().unwrap();
        let status = std::process::Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status()
            .unwrap();
        assert!(status.success());
        tokio::time::timeout(Duration::from_secs(2), async {
            while crashed.alive() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            crashed
                .request("workspace/symbol", json!({"query":""}))
                .await
                .unwrap_err()
                .code,
            "process_failed"
        );
    }
    crashed.shutdown().await;
}

#[tokio::test]
async fn failed_save_reports_committed_revision_and_preserves_recovery() {
    let temp = tempfile::tempdir().unwrap();
    let recovery = temp.path().join("recovery.json");
    let app = Application::new(service(), "fixture".into(), Some(recovery.clone()));
    let failed = app
        .execute(Request::new(
            "file.commit_and_save",
            json!({"path":temp.path(),"nodeId":"root","patch":{"name":"retained edit"}}),
        ))
        .await;
    assert!(failed.is_err());
    assert_eq!(app.revision(), 1);
    let restored = Service::read_recovery(&recovery).unwrap();
    assert_eq!(restored.document.project_name(), "retained edit");
    assert!(restored.dirty());
    let status = app
        .execute(Request::new("session.status", json!({})))
        .await
        .unwrap();
    assert_eq!(status["revision"], 1);
    assert_eq!(status["dirty"], true);
    app.shutdown().await;
}
#[tokio::test]
async fn recovery_failures_are_visible_without_repeating_committed_mutations() {
    let temp = tempfile::tempdir().unwrap();
    let blocker = temp.path().join("file");
    std::fs::write(&blocker, b"existing").unwrap();
    let app = Application::new(
        service(),
        "fixture".into(),
        Some(blocker.join("recovery.json")),
    );
    let result = app
        .execute(Request::new(
            "threadwork.create_entity",
            json!({"name":"committed once"}),
        ))
        .await
        .unwrap();
    assert_eq!(result["_wirecraftWarnings"][0]["code"], "recovery_failed");
    assert_eq!(app.revision(), 1);
    assert_eq!(app.service.lock().unwrap().document.nodes.len(), 2);
    app.shutdown().await;
}

#[tokio::test]
async fn shutdown_closes_live_event_subscriptions() {
    let app = Application::new(service(), "temporary-test-token".into(), None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/events", listener.local_addr().unwrap());
    let server_app = app.clone();
    let server = tokio::spawn(async move {
        server_app.serve(listener).await.unwrap();
    });
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .bearer_auth("temporary-test-token")
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    app.execute(Request::new("system.shutdown", json!({})))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
    drop(response);
}
