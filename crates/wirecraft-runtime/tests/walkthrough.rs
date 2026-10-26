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
