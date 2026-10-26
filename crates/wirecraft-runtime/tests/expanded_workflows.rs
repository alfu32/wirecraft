use serde_json::{Value, json};
use wirecraft_core::{Document, NodeKind, store};
use wirecraft_runtime::{
    archetypes, export,
    service::{Request, Service},
};
#[test]
fn all_bundled_archetypes_import_with_fresh_ids() {
    for (id, _) in archetypes::BUILTINS {
        let mut s = Service::new(Document::new("target"), None);
        let source = archetypes::load(id).unwrap();
        s.execute(Request::new(
            "archetypes.import",
            json!({"id":id,"parentId":"root"}),
        ))
        .unwrap();
        s.document.ensure_valid().unwrap();
        assert_eq!(s.document.nodes.len(), source.nodes.len() + 1, "{id}");
        for id in source.nodes.keys() {
            if id != "root" {
                assert!(!s.document.nodes.contains_key(id));
            }
        }
    }
}
#[test]
fn alignment_distribution_cut_paste_and_undo_are_transactions() {
    let mut s = Service::new(Document::new("flow"), None);
    let mut ids = vec![];
    for x in [0., 50., 1000.] {
        let v = s
            .execute(Request::new(
                "threadwork.create_entity",
                json!({"name":"node","attributes":{"layout":{"x":x,"y":x}}}),
            ))
            .unwrap();
        ids.push(v["entity"]["id"].as_str().unwrap().to_string());
    }
    s.execute(Request::new(
        "entities.align",
        json!({"nodeIds":ids,"operation":"distribute-horizontal"}),
    ))
    .unwrap();
    assert_eq!(s.document.nodes[&ids[1]].layout.x, 500.);
    let fragment = s
        .execute(Request::new(
            "entities.cut",
            json!({"nodeIds":[ids[0],ids[1]]}),
        ))
        .unwrap();
    assert_eq!(s.document.nodes.len(), 2);
    s.execute(Request::new(
        "entities.paste",
        json!({"fragment":fragment,"parentId":"root"}),
    ))
    .unwrap();
    assert_eq!(s.document.nodes.len(), 4);
    s.execute(Request::new("edit.undo", json!({}))).unwrap();
    assert_eq!(s.document.nodes.len(), 2);
    let before = serde_json::to_value(&s.document).unwrap();
    let malformed = json!({"format":"wirecraft-fragment-v1","roots":["missing"],"nodes":[]});
    assert!(
        s.execute(Request::new(
            "entities.paste",
            json!({"fragment":malformed})
        ))
        .is_err()
    );
    assert_eq!(serde_json::to_value(&s.document).unwrap(), before);
}
#[test]
fn real_svg_png_and_vector_pdf_exports() {
    let temp = tempfile::tempdir().unwrap();
    let mut d = Document::new("report");
    d.create("root", "worker", NodeKind::Processor).unwrap();
    for (format, magic) in [
        ("svg", b"<svg".as_slice()),
        ("png", b"\x89PNG\r\n\x1a\n".as_slice()),
        ("pdf", b"%PDF".as_slice()),
    ] {
        let path = temp.path().join(format!("report.{format}"));
        export::diagram(&d, "root", &path, format).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(magic), "{format}");
        assert!(bytes.len() > 100);
    }
    let svg = temp.path().join("report.svg");
    let mut s = Service::new(d, None);
    let result = s
        .execute(Request::new(
            "export.diagram",
            json!({"path":svg,"format":"svg"}),
        ))
        .unwrap();
    assert_eq!(result["exported"], true);
    let path = temp.path().join("still.orch");
    store::save(&s.document, &path).unwrap();
    let _: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
}

#[test]
fn explicit_settings_import_preserves_ui_values_and_excludes_credentials() {
    let xml = r#"<?xml version="1.0"?><!DOCTYPE preferences SYSTEM "http://java.sun.com/dtd/preferences.dtd"><preferences EXTERNAL_XML_VERSION="1.0"><root type="user"><map/><node name="com"><map/><node name="threadwork"><map/><node name="app"><map/><node name="editor"><map><entry key="indentSpaces" value="8"/></map></node><node name="ui"><map><entry key="divider" value="240"/></map></node><node name="identity"><map><entry key="emailAddress" value="private@example.test"/></map></node><node name="ai"><map><entry key="apiKey" value="do-not-import"/></map></node></node></node></node></root></preferences>"#;
    let s = wirecraft_runtime::config::import_preferences(xml).unwrap();
    assert_eq!(s.editor_indent_spaces, 8);
    assert_eq!(s.legacy_preferences.len(), 2);
    let text = serde_json::to_string(&s).unwrap();
    assert!(!text.contains("private@example.test"));
    assert!(!text.contains("do-not-import"));
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("settings.json");
    s.save(&path).unwrap();
    assert_eq!(
        wirecraft_runtime::config::Settings::load(&path)
            .unwrap()
            .editor_indent_spaces,
        8
    );
    assert!(wirecraft_runtime::config::import_preferences("<unrelated/>").is_err());
}
#[tokio::test]
async fn literal_generation_source_maps_external_compiler_and_path_safety() {
    use std::time::Duration;
    use wirecraft_runtime::backend::Application;
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("generated");
    let mut service = Service::new(Document::new("literal"), None);
    let created=service.execute(Request::new("threadwork.create_entity",json!({"name":"hello.c","attributes":{"technology":{"technologyId":"file-export","languageId":"c"},"text":{"source":"#include <stdio.h>\nint main(void) { puts(\"wirecraft-fixture\"); return 0; }\n"}}}))).unwrap();
    let id = created["entity"]["id"].as_str().unwrap().to_string();
    let generated = service
        .execute(Request::new("generate.files", json!({"outputDir":output})))
        .unwrap();
    assert_eq!(generated["files"][0]["sourceMap"][0]["nodeId"], id);
    assert!(output.join("hello.c").exists());
    assert!(output.join("wirecraft-source-map.json").exists());
    assert!(
        service
            .execute(Request::new("generate.files", json!({"outputDir":output})))
            .is_err()
    );
    let app = Application::new(service, "fixture".into(), None);
    let executable = temp.path().join("hello");
    let job = app
        .jobs
        .start(
            "clang",
            &[
                "-std=c17".into(),
                output.join("hello.c").to_string_lossy().into(),
                "-o".into(),
                executable.to_string_lossy().into(),
            ],
            temp.path(),
            Duration::from_secs(10),
        )
        .await
        .unwrap();
    let done = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let state = app.jobs.get(job["id"].as_str().unwrap()).await.unwrap();
            if state["status"] != "running" {
                break state;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(done["status"], "succeeded", "{done}");
    let run = std::process::Command::new(executable).output().unwrap();
    assert!(run.status.success());
    assert_eq!(run.stdout, b"wirecraft-fixture\n");
    app.shutdown().await;
    let mut s = Service::new(Document::new("malformed"), None);
    s.execute(Request::new(
        "threadwork.create_entity",
        json!({"name":"escape","attributes":{"metadata":{"path":"../escape"}}}),
    ))
    .unwrap();
    assert!(
        s.execute(Request::new(
            "generate.files",
            json!({"outputDir":temp.path().join("unsafe")})
        ))
        .is_err()
    );
    assert!(!temp.path().join("unsafe").exists());
}
