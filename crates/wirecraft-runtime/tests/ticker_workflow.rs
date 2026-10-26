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
