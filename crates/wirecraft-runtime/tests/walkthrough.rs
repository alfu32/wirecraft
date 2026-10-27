use serde_json::json;
use wirecraft_core::Document;
use wirecraft_runtime::service::{Request, Service};
#[test]
fn archetype_preview_and_board_transitions_share_transactions() {
    let mut service = Service::new(Document::new("Walkthrough"), None);
    let run =
        |s: &mut Service, command: &str, arguments| s.execute(Request::new(command, arguments));
    let catalog = run(&mut service, "archetypes.list", json!({})).unwrap();
    let before = service.revision;
    let id = catalog["archetypes"][0]["id"].as_str().unwrap();
    let preview = run(&mut service, "archetypes.preview", json!({"id":id})).unwrap();
    let document: Document = serde_json::from_value(preview["document"].clone()).unwrap();
    assert!(!document.nodes.is_empty());
    assert_eq!(before, service.revision);
    assert!(run(&mut service, "archetypes.preview", json!({"id":"missing"})).is_err());
    let created = run(
        &mut service,
        "threadwork.create_entity",
        json!({"name":"Ticket"}),
    )
    .unwrap();
    let id = created["entity"]["id"].as_str().unwrap();
    run(
        &mut service,
        "threadwork.update_entity",
        json!({"nodeId":id,"patch":{"status":"TESTING","assignee":"local:dev"}}),
    )
    .unwrap();
    assert_eq!(
        service.document.nodes[id].status.as_deref(),
        Some("TESTING")
    );
    assert_eq!(
        service.document.nodes[id].status_changes.last().unwrap()["endStatus"],
        "TESTING"
    );
    run(
        &mut service,
        "threadwork.update_entity",
        json!({"nodeId":id,"patch":{"status":null}}),
    )
    .unwrap();
    assert!(service.document.nodes[id].status.is_none());
    run(&mut service, "edit.undo", json!({})).unwrap();
    assert_eq!(
        service.document.nodes[id].status.as_deref(),
        Some("TESTING")
    );
}
#[test]
fn structural_matrix_is_relationship_union_not_parenthood() {
    let mut service = Service::new(Document::new("Matrix"), None);
    let root = service.document.root_node_id.clone();
    let a = service
        .document
        .create(&root, "A", wirecraft_core::NodeKind::Processor)
        .unwrap();
    let b = service
        .document
        .create(&root, "B", wirecraft_core::NodeKind::Processor)
        .unwrap();
    service
        .document
        .create_link(
            "data",
            wirecraft_core::Link {
                source_node_id: a.clone(),
                target_node_id: b.clone(),
                ..Default::default()
            },
        )
        .unwrap();
    let report = service
        .execute(Request::new("analysis.report", json!({})))
        .unwrap();
    assert_eq!(report["graphs"]["structural"]["adjacency"][&a], json!([b]));
    assert_eq!(report["graphs"]["structural"]["edges"], 1);
    assert_eq!(report["graphs"]["containment"]["edges"], 2);
}

#[tokio::test]
async fn settings_are_validated_persisted_and_mcp_can_stop_without_stopping_ui() {
    use wirecraft_runtime::backend::Application;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("settings.json");
    let app = Application::new(
        Service::new(Document::new("Controls"), None),
        "walkthrough-secret-token".into(),
        None,
    )
    .with_settings(Some(path.clone()))
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let serving = app.clone();
    let server = tokio::spawn(async move { serving.serve(listener).await.unwrap() });
    let mut settings = app
        .execute(Request::new("settings.get", json!({})))
        .await
        .unwrap();
    settings["appearance"]["colors"]["canvasBackground"] = json!("#123456");
    settings["editorIndentSpaces"] = json!(2);
    app.execute(Request::new("settings.replace", settings.clone()))
        .await
        .unwrap();
    assert_eq!(
        wirecraft_runtime::config::Settings::load(&path)
            .unwrap()
            .editor_indent_spaces,
        2
    );
    settings["appearance"]["colors"]["canvasBackground"] = json!("#oops");
    assert!(
        app.execute(Request::new("settings.replace", settings))
            .await
            .is_err()
    );
    assert_eq!(
        wirecraft_runtime::config::Settings::load(&path)
            .unwrap()
            .appearance
            .colors["canvasBackground"],
        "#123456"
    );
    let http = reqwest::Client::new();
    let url = format!("http://{addr}/mcp");
    app.execute(Request::new("mcp.set_enabled", json!({"enabled":false})))
        .await
        .unwrap();
    assert_eq!(
        http.post(&url)
            .bearer_auth("walkthrough-secret-token")
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"ping"}))
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    assert_eq!(
        app.execute(Request::new("session.snapshot", json!({})))
            .await
            .unwrap()["document"]["name"],
        "Controls"
    );
    app.execute(Request::new("mcp.set_enabled", json!({"enabled":true})))
        .await
        .unwrap();
    assert_eq!(
        http.post(&url)
            .bearer_auth("walkthrough-secret-token")
            .json(&json!({"jsonrpc":"2.0","id":2,"method":"ping"}))
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    let status = app
        .execute(Request::new("mcp.status", json!({})))
        .await
        .unwrap();
    assert_eq!(status["log"].as_array().unwrap().len(), 2);
    assert!(!status.to_string().contains("walkthrough-secret-token"));
    app.execute(Request::new("system.shutdown", json!({})))
        .await
        .unwrap();
    server.await.unwrap();
}

#[test]
fn technical_sheets_and_documentation_export_real_paginated_content() {
    let temp = tempfile::tempdir().unwrap();
    let mut service = Service::new(Document::new("Walkthrough Report"), None);
    service.execute(Request::new("threadwork.create_entity",json!({"name":"Far Worker","attributes":{"layout":{"x":2500.,"y":300.},"text":{"source":"int value = 42;","specification":"Returns a value & preserves <constraints>."}}}))).unwrap();
    service
        .execute(Request::new(
            "threadwork.create_entity",
            json!({"name":"Near Worker","attributes":{"layout":{"x":0.,"y":0.}}}),
        ))
        .unwrap();
    let options = json!({"format":"A4","scale":1,"split":true,"overlapMm":5.});
    let preview = service
        .execute(Request::new(
            "export.preview",
            json!({"options":options,"generation":17}),
        ))
        .unwrap();
    assert!(preview["pages"].as_u64().unwrap() > 1);
    assert_eq!(preview["generation"], 17);
    use base64::Engine;
    let png = base64::engine::general_purpose::STANDARD
        .decode(preview["pngBase64"].as_str().unwrap())
        .unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(u32::from_be_bytes(png[16..20].try_into().unwrap()) <= 1200);
    assert!(u32::from_be_bytes(png[20..24].try_into().unwrap()) <= 1200);
    assert!(preview["svg"].as_str().unwrap().contains("210mm"));
    let path = temp.path().join("poster.pdf");
    let result = service
        .execute(Request::new(
            "export.write",
            json!({"options":options,"format":"pdf","path":path}),
        ))
        .unwrap();
    assert_eq!(result["pages"], preview["pages"]);
    assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF"));
    if let Ok(info) = std::process::Command::new("pdfinfo").arg(&path).output() {
        assert!(info.status.success());
        let text = String::from_utf8_lossy(&info.stdout);
        let count = text
            .lines()
            .find(|l| l.starts_with("Pages:"))
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap()
            .parse::<u64>()
            .unwrap();
        assert_eq!(count, preview["pages"].as_u64().unwrap());
        assert!(text.contains("595.276"), "{text}");
    }
    let svg_path = temp.path().join("poster.svg");
    assert!(
        service
            .execute(Request::new(
                "export.write",
                json!({"options":options,"format":"svg","path":svg_path})
            ))
            .is_err()
    );
    assert!(!svg_path.exists());
    for format in ["html", "markdown", "pdf"] {
        let path = temp.path().join(format!("docs.{format}"));
        service.execute(Request::new("export.write",json!({"kind":"documentation","options":{"format":"A4"},"format":format,"path":path}))).unwrap();
        if format != "pdf" {
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(text.contains("Far Worker"));
            if format == "html" {
                assert!(text.contains("&amp; preserves &lt;constraints&gt;"));
            }
        }
    }
    let preview = service
        .execute(Request::new(
            "export.preview",
            json!({"kind":"documentation","options":{}}),
        ))
        .unwrap();
    assert!(
        preview["svg"]
            .as_str()
            .unwrap()
            .contains("Functional and Technical Specification")
    );
}

#[test]
fn clipboard_assistance_contains_task_types_authored_text_and_compiler_contract() {
    let mut d = Document::new("Assistance");
    let id = d
        .create("root", "Worker", wirecraft_core::NodeKind::Processor)
        .unwrap();
    d.patch(&id,&json!({"text":{"specification":"Preserve six backticks: ``````","source":"int actual = 7;","testsLanguageId":"json"},"technology":{"languageId":"c","technologyId":"c-native"}})).unwrap();
    for task in ["source", "specification", "tests"] {
        let result =
            wirecraft_core::assistance::prompt(&d, std::slice::from_ref(&id), task).unwrap();
        let text = result["prompt"].as_str().unwrap();
        assert!(text.contains("int actual = 7;"));
        assert!(text.contains("```````markdown"));
        assert_eq!(result["section"], task);
        if task == "source" {
            assert!(text.contains("threadwork_buffer_count"));
        }
    }
    assert!(wirecraft_core::assistance::prompt(&d, &[], "source").is_err());
}

#[test]
fn raster_pdf_embeds_an_image_and_vector_pdf_keeps_searchable_text() {
    let temp = tempfile::tempdir().unwrap();
    let mut s = Service::new(Document::new("SearchableTitle"), None);
    let vector = temp.path().join("vector.pdf");
    let raster = temp.path().join("raster.pdf");
    for (path, rasterized) in [(&vector, false), (&raster, true)] {
        s.execute(Request::new("export.write",json!({"options":{"format":"A4","scale":2},"format":"pdf","path":path,"rasterized":rasterized}))).unwrap();
    }
    let bytes = std::fs::read(&raster).unwrap();
    assert!(
        bytes.len() < 500_000,
        "Blank A4 report should compress: {}",
        bytes.len()
    );
    assert!(
        bytes
            .windows(b"/FlateDecode".len())
            .any(|w| w == b"/FlateDecode")
    );
    assert!(
        bytes
            .windows(b"/Subtype /Image".len())
            .any(|w| w == b"/Subtype /Image")
    );
    if let Ok(output) = std::process::Command::new("pdftotext")
        .arg(&vector)
        .arg("-")
        .output()
    {
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("SearchableTitle"));
    }
}

#[test]
fn legacy_appearance_and_font_preferences_activate_without_importing_credentials() {
    let xml = r##"<preferences><root><node name="com"><node name="threadwork"><node name="app"><node name="appearance"><map><entry key="theme" value="Light"/><entry key="light.canvasbackground" value="#FAFCFD"/><entry key="scroll.rows.per.wheel.tick" value="5"/></map></node><node name="fonts"><map><entry key="codeFont" value="monaspace-neon-regular"/></map></node><node name="designer"><map><entry key="compositeTitleTargetScreenPx" value="22"/></map></node><node name="ai"><map><entry key="claude.token" value="never-import-this"/></map></node></node></node></node></root></preferences>"##;
    let settings = wirecraft_runtime::config::import_preferences(xml).unwrap();
    assert_eq!(settings.appearance.theme, "light");
    assert_eq!(settings.appearance.colors["canvasBackground"], "#FAFCFD");
    assert_eq!(settings.appearance.wheel_rows, 5.);
    assert_eq!(settings.appearance.composite_title_size, 22.);
    assert_eq!(settings.appearance.editor_font, "Monaspace Neon");
    assert!(
        !serde_json::to_string(&settings)
            .unwrap()
            .contains("never-import-this")
    );
}
#[test]
fn combined_pdf_includes_plan_and_documentation_and_auto_never_silently_crops() {
    let temp = tempfile::tempdir().unwrap();
    let mut s = Service::new(Document::new("CombinedPlan"), None);
    let path = temp.path().join("all.pdf");
    let result=s.execute(Request::new("export.write",json!({"kind":"all","sheetOptions":{"format":"A4","scale":2},"documentOptions":{"format":"A4"},"format":"pdf","path":path}))).unwrap();
    assert_eq!(result["summary"]["planPages"], 1);
    assert!(result["summary"]["documentationPages"].as_u64().unwrap() > 0);
    assert_eq!(
        result["pages"].as_u64().unwrap(),
        1 + result["summary"]["documentationPages"].as_u64().unwrap()
    );
    s.execute(Request::new(
        "threadwork.create_entity",
        json!({"name":"Near","attributes":{"layout":{"x":0.}}}),
    ))
    .unwrap();
    s.execute(Request::new(
        "threadwork.create_entity",
        json!({"name":"Outside","attributes":{"layout":{"x":100_000.}}}),
    ))
    .unwrap();
    let path = temp.path().join("auto.svg");
    assert!(
        s.execute(Request::new(
            "export.write",
            json!({"format":"svg","path":path})
        ))
        .is_err()
    );
    assert!(!path.exists());
    let preview = s
        .execute(Request::new(
            "export.preview",
            json!({"options":{"format":"A4","scale":1}}),
        ))
        .unwrap();
    assert_eq!(preview["summary"]["clipped"], true);
    assert!(
        preview["svg"]
            .as_str()
            .unwrap()
            .contains("Drawing exceeds this fixed sheet")
    );
}

#[test]
fn project_revision_updates_transactionally_and_node_edits_capture_it() {
    let mut s = Service::new(Document::new("Revisions"), None);
    let before = s.document.nodes["root"].modified.clone();
    s.execute(Request::new(
        "threadwork.update_entity",
        json!({"nodeId":"root","patch":{"masterRevision":{"name":"R1","date":"2026-10-26"}}}),
    ))
    .unwrap();
    assert_eq!(s.document.master_revision["name"], "R1");
    assert_eq!(s.document.nodes["root"].modified, before);
    assert!(s.document.nodes["root"].revision.is_none());
    let node = s
        .execute(Request::new(
            "threadwork.create_entity",
            json!({"name":"Worker"}),
        ))
        .unwrap();
    let id = node["entity"]["id"].as_str().unwrap();
    assert_eq!(node["effectiveRevision"]["name"], "R1");
    s.execute(Request::new(
        "threadwork.update_entity",
        json!({"nodeId":"root","patch":{"masterRevision":{"name":"R2"}}}),
    ))
    .unwrap();
    assert_eq!(s.document.effective_revision(id).unwrap()["name"], "R1");
    s.execute(Request::new(
        "threadwork.update_entity",
        json!({"nodeId":id,"patch":{"text":{"source":"int version = 2;"}}}),
    ))
    .unwrap();
    assert_eq!(s.document.effective_revision(id).unwrap()["name"], "R2");
    s.execute(Request::new("edit.undo", json!({}))).unwrap();
    assert_eq!(s.document.effective_revision(id).unwrap()["name"], "R1");
    let before = serde_json::to_value(&s.document).unwrap();
    assert!(
        s.execute(Request::new(
            "threadwork.update_entity",
            json!({"nodeId":"root","patch":{"masterRevision":{"name":17}}})
        ))
        .is_err()
    );
    assert_eq!(serde_json::to_value(&s.document).unwrap(), before);
}
