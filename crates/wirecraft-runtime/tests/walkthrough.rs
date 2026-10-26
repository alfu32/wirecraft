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
