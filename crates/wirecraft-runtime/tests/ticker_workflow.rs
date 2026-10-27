//! Supplied reference stays immutable. Only generated output and a documented draft correction run.
use base64::Engine;
use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use wirecraft_core::{Document, c17::C17, store};
use wirecraft_runtime::{
    backend::Application,
    service::{Request, Service},
};
fn reference() -> Document {
    store::load(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/migration/threadworks/example-c.orch"),
    )
    .unwrap()
}
async fn finished(app: &Application, id: &str) -> Value {
    tokio::time::timeout(Duration::from_secs(12), async {
        loop {
            let job = app.jobs.get(id).await.unwrap();
            if job["status"] != "running" {
                return job;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn supplied_ticker_preserves_exports_reports_draft_error_and_runs_corrected_copy() {
    let d = reference();
    let original = serde_json::to_value(&d).unwrap();
    let project = C17::generate(&d, "root").unwrap();
    assert_eq!(project.files.len(), 7);
    let uuid = d.nodes.values().find(|n| n.name == "gen_uuid").unwrap();
    assert!(uuid.text.source.ends_with("\npu"));
    let range = project.files[0]
        .source_map
        .iter()
        .find(|m| m.node_id == uuid.id && m.section == "source")
        .unwrap();
    assert_eq!(
        project.files[0]
            .source_location(range.generated_end_line, 5)
            .unwrap()["nodeId"],
        uuid.id
    );
    for n in d
        .nodes
        .values()
        .filter(|n| d.effective(&n.id, "technologyId") == "file-export")
    {
        let file = project.files.iter().find(|f| f.path == n.name).unwrap();
        assert_eq!(file.binary_content, n.binary_content);
        if n.binary_content.is_none() {
            assert_eq!(file.content, n.text.source);
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("original");
    wirecraft_runtime::generation::write(&project, &output).unwrap();
    let app = Application::new(
        Service::new(d.clone(), None),
        "isolated-ticker".into(),
        None,
    );
    let args = |source: &Path, exe: &Path| {
        vec![
            "-std=c17".into(),
            "-D_DEFAULT_SOURCE".into(),
            "-Wall".into(),
            "-Wextra".into(),
            "-Werror".into(),
            "-Wno-unused-function".into(),
            source.to_string_lossy().into(),
            "-luuid".into(),
            "-o".into(),
            exe.to_string_lossy().into(),
        ]
    };
    let exe = temp.path().join("ticker");
    let job = app
        .jobs
        .start(
            "clang",
            &args(&output.join("ticker.c"), &exe),
            temp.path(),
            Duration::from_secs(8),
        )
        .await
        .unwrap();
    let failed = finished(&app, job["id"].as_str().unwrap()).await;
    assert_eq!(failed["status"], "failed", "{failed}");
    assert!(
        failed["stderr"]
            .as_str()
            .unwrap()
            .contains("undeclared identifier 'pu'"),
        "{failed}"
    );
    // This is an explicit test-fixture correction, never production sanitization or a reference edit.
    let corrected = uuid.text.source.strip_suffix("pu").unwrap();
    app.execute(Request::new(
        "threadwork.update_entity",
        json!({"nodeId":uuid.id,"patch":{"text":{"source":corrected}}}),
    ))
    .await
    .unwrap();
    let corrected_dir = temp.path().join("corrected");
    app.execute(Request::new(
        "compile.c17",
        json!({"outputDir":corrected_dir}),
    ))
    .await
    .unwrap();
    let job = app
        .jobs
        .start(
            "clang",
            &args(&corrected_dir.join("ticker.c"), &exe),
            temp.path(),
            Duration::from_secs(8),
        )
        .await
        .unwrap();
    let built = finished(&app, job["id"].as_str().unwrap()).await;
    assert_eq!(built["status"], "succeeded", "{built}");
    // Do not execute run.sh (legacy TCC), qjs binaries, or write outside this owned directory.
    let job = app
        .jobs
        .start(
            exe.to_str().unwrap(),
            &["--sleep".into(), "10000".into()],
            temp.path(),
            Duration::from_secs(5),
        )
        .await
        .unwrap();
    let csv = temp.path().join("ticker_out.csv");
    let result = tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            if let Ok(text) = std::fs::read_to_string(&csv) {
                let lines = text.lines().collect::<Vec<_>>();
                if lines.len() >= 3 {
                    return text;
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await;
    app.jobs.cancel(job["id"].as_str().unwrap()).await.unwrap();
    assert_eq!(
        finished(&app, job["id"].as_str().unwrap()).await["status"],
        "cancelled"
    );
    let text = result.expect("ticker did not produce three rows");
    assert!(
        text.lines()
            .take(3)
            .all(|line| line.ends_with(",true") && line.contains('T')),
        "{text}"
    );
    for file in &project.files {
        if let Some(binary) = &file.binary_content {
            assert_eq!(
                std::fs::read(corrected_dir.join(&file.path)).unwrap(),
                base64::engine::general_purpose::STANDARD
                    .decode(binary)
                    .unwrap()
            );
        }
    }
    app.shutdown().await;
    assert_eq!(serde_json::to_value(reference()).unwrap(), original);
}

#[tokio::test]
async fn compiler_context_completes_runtime_channels_libraries_and_members_without_committing() {
    let d = reference();
    let id = d
        .nodes
        .values()
        .find(|n| n.name == "write_append_file")
        .unwrap()
        .id
        .clone();
    let original = serde_json::to_value(&d).unwrap();
    let app = Application::new(Service::new(d, None), "context-fixture".into(), None);
    let context = app
        .execute(Request::new(
            "editor.context",
            json!({"nodeId":id,"section":"source"}),
        ))
        .await
        .unwrap();
    assert!(
        context["header"]
            .as_str()
            .unwrap()
            .contains("threadwork_buffer *ip_word")
    );
    assert!(
        context["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["label"] == "ip_word")
    );
    let mut completion_pid = None;
    for (text, expected) in [
        (
            "threadwork_runner__shut",
            "threadwork_runner__shutdown_request",
        ),
        ("ip_w", "ip_word"),
        ("/*🙂*/ argslib__op", "argslib__option"),
        ("Packet item;\nitem.tim", "timestamp"),
    ] {
        let result=app.execute(Request::new("editor.complete",json!({"nodeId":id,"section":"source","text":text,"offset":text.len(),"generation":7}))).await.unwrap();
        completion_pid = result["pid"].as_u64();
        assert_eq!(result["generation"], 7);
        assert_eq!(result["positionEncoding"], "utf-32");
        let item = result["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| {
                i["filterText"]
                    .as_str()
                    .or(i["label"].as_str())
                    .is_some_and(|s| s.trim() == expected)
            })
            .unwrap_or_else(|| panic!("{expected} missing: {result}"));
        if expected == "argslib__option" {
            assert!(
                item["detail"].as_str().is_some_and(|s| !s.is_empty()),
                "Missing library signature: {item}"
            );
        }
        let range = &item["textEdit"]["range"];
        let byte = |position: &Value| {
            wirecraft_runtime::lsp::offset(
                text,
                position["line"].as_u64().unwrap() as usize,
                position["character"].as_u64().unwrap() as usize,
                "utf-32",
            )
            .unwrap()
        };
        let mut edited = text.to_string();
        edited.replace_range(
            byte(&range["start"])..byte(&range["end"]),
            item["textEdit"]["newText"].as_str().unwrap(),
        );
        assert!(edited.ends_with(expected), "{edited}");
        if expected == "ip_word" {
            assert!(
                item["detail"]
                    .as_str()
                    .unwrap()
                    .contains("input buffer of Packet")
            );
        }
        assert_eq!(result["revision"], 0);
    }
    let result = app
        .execute(Request::new(
            "editor.complete",
            json!({"nodeId":id,"section":"initialization","text":"","offset":0}),
        ))
        .await
        .unwrap();
    assert!(!result["items"].as_array().unwrap().is_empty());
    let text = "uuids__";
    let result = app
        .execute(Request::new(
            "editor.complete",
            json!({"nodeId":id,"section":"source","text":text,"offset":text.len()}),
        ))
        .await
        .unwrap();
    assert!(
        !result["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["label"]
                .as_str()
                .unwrap_or("")
                .trim()
                .starts_with("uuids__")),
        "Other node's dependency leaked"
    );
    let bad = app
        .execute(Request::new(
            "editor.complete",
            json!({"nodeId":id,"section":"source","text":"🙂","offset":1}),
        ))
        .await
        .unwrap_err();
    assert_eq!(bad.code, "invalid_input");
    let mut stale = Request::new(
        "editor.complete",
        json!({"nodeId":id,"section":"source","text":"ip_w","offset":4}),
    );
    stale.expected_revision = Some(999);
    assert_eq!(app.execute(stale).await.unwrap_err().code, "conflict");
    assert_eq!(
        serde_json::to_value(&app.service.lock().unwrap().document).unwrap(),
        original
    );
    app.shutdown().await;
    #[cfg(target_os = "linux")]
    assert!(!Path::new(&format!("/proc/{}", completion_pid.unwrap())).exists());
}

#[tokio::test]
async fn full_generated_ast_maps_semantic_tokens_into_unicode_body_and_stages_companions() {
    let d = reference();
    let id = d
        .nodes
        .values()
        .find(|n| n.name == "write_append_file")
        .unwrap()
        .id
        .clone();
    let app = Application::new(Service::new(d, None), "semantic-fixture".into(), None);
    let text = "/*🙂*/ Packet item = {0};\nitem.timestamp = NULL;\nprintf(\"%s\", argslib__option(\"file\"));\npush(ip_word, &item);";
    let result = app
        .execute(Request::new(
            "editor.tokens",
            json!({"nodeId":id,"section":"source","text":text,"generation":42}),
        ))
        .await
        .unwrap();
    assert_eq!(result["generation"], 42);
    assert_eq!(result["revision"], 0);
    assert_eq!(result["generatedFiles"], 7);
    assert!(result["generatedBytes"].as_u64().unwrap() > text.len() as u64 * 20);
    let tokens = result["tokens"].as_array().unwrap();
    for (identifier, kind) in [
        ("Packet", "type"),
        ("timestamp", "property"),
        ("printf", "function"),
        ("argslib__option", "function"),
        ("push", "function"),
        ("ip_word", "parameter"),
    ] {
        assert!(
            tokens.iter().any(|t| {
                let a = t["startByte"].as_u64().unwrap() as usize;
                let b = t["endByte"].as_u64().unwrap() as usize;
                assert!(
                    a < b
                        && b <= text.len()
                        && text.is_char_boundary(a)
                        && text.is_char_boundary(b)
                );
                &text[a..b] == identifier
                    && (t["kind"] == kind || (kind == "type" && t["kind"] == "class"))
            }),
            "Missing {identifier}:{kind}: {tokens:?}"
        );
    }
    let mut stale = Request::new(
        "editor.tokens",
        json!({"nodeId":id,"section":"source","text":text}),
    );
    stale.expected_revision = Some(999);
    assert_eq!(app.execute(stale).await.unwrap_err().code, "conflict");
    assert_eq!(app.service.lock().unwrap().revision, 0);
    let pid = result["pid"].as_u64().unwrap();
    app.shutdown().await;
    #[cfg(target_os = "linux")]
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
}

#[tokio::test]
async fn real_mcp_editor_context_and_vector_documentation_use_shared_services() {
    let d = reference();
    let id = d
        .nodes
        .values()
        .find(|n| n.name == "read_time")
        .unwrap()
        .id
        .clone();
    let app = Application::new(Service::new(d, None), "ticker-mcp-token".into(), None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let server_app = app.clone();
    let server = tokio::spawn(async move {
        server_app.serve(listener).await.unwrap();
    });
    let client = reqwest::Client::new();
    for (operation, arguments) in [
        ("editor_context", json!({"nodeId":id,"section":"source"})),
        (
            "complete",
            json!({"nodeId":id,"section":"source","text":"tm__str","offset":7}),
        ),
    ] {
        let value:Value=client.post(&url).bearer_auth("ticker-mcp-token").json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"wirecraft.workbench","arguments":{"operation":operation,"arguments":arguments}}})).send().await.unwrap().json().await.unwrap();
        assert_eq!(value["result"]["isError"], false, "{value}");
        if operation == "complete" {
            assert!(
                value["result"]["structuredContent"]["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|i| i["label"].as_str().unwrap_or("").trim() == "tm__strftime"),
                "{value}"
            );
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("documentation.pdf");
    app.execute(Request::new(
        "export.write",
        json!({"kind":"documentation","format":"pdf","path":path}),
    ))
    .await
    .unwrap();
    let images = std::process::Command::new("pdfimages")
        .arg("-list")
        .arg(&path)
        .output()
        .unwrap();
    assert!(images.status.success());
    let image_lines = String::from_utf8(images.stdout).unwrap();
    assert_eq!(
        image_lines.lines().count(),
        2,
        "Vector dossier embedded an image: {image_lines}"
    );
    let text = std::process::Command::new("pdftotext")
        .arg(&path)
        .arg("-")
        .output()
        .unwrap();
    assert!(text.status.success());
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(
        text.contains("Composition")
            && text.contains("read_time")
            && text.contains("argslib__option")
    );
    assert!(!text.contains("flowchart LR"));
    let svg = app
        .execute(Request::new(
            "export.preview",
            json!({"kind":"documentation","page":1}),
        ))
        .await
        .unwrap();
    assert!(svg["svg"].as_str().unwrap().contains("<path"));
    assert!(!svg["svg"].as_str().unwrap().contains("data:image"));
    app.execute(Request::new("system.shutdown", json!({})))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
}
