//! Safe live JVM calls recorded on 2026-10-27; never open/mutate its project.
use serde_json::{Value, json};
use wirecraft_core::store;
use wirecraft_runtime::service::{Request, Service};
#[test]
fn webserver_network_and_link_contracts_match_live_jvm_read_results() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let baseline: Value = serde_json::from_slice(
        &std::fs::read(
            root.join("docs/migration/baseline/jvm-live-2026-10-27/readonly-comparison.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let d = store::load(&root.join("crates/wirecraft-core/tests/fixtures/webserver.orch")).unwrap();
    for reference in baseline["links"].as_array().unwrap() {
        let node = d
            .nodes
            .values()
            .find(|n| n.name == reference["name"].as_str().unwrap())
            .unwrap();
        let link = node.link.as_ref().unwrap();
        assert_eq!(link.interaction_kind, reference["interactionKind"]);
        assert_eq!(link.transport_kind, reference["transportKind"]);
        assert_eq!(
            node.file_layout_strategy_id,
            reference["fileLayoutStrategyId"]
        );
    }
    let mut service = Service::new(d, None);
    let actual = service
        .execute(Request::new("threadwork.analyze_network", json!({})))
        .unwrap();
    for key in [
        "entities",
        "links",
        "types",
        "composites",
        "ports",
        "projectName",
    ] {
        assert_eq!(actual[key], baseline["analysis"][key], "{key}: {actual}");
    }
    assert_eq!(
        service
            .execute(Request::new("threadwork.validate_design", json!({})))
            .unwrap()["valid"],
        baseline["validation"]["valid"]
    );
}
