use serde_json::json;
use wirecraft_core::{store, viewport::*, *};
#[test]
fn source_fixture_roundtrips_without_payload_loss() {
    let text = include_str!("fixtures/threadworks.orch");
    let d = store::decode(text).unwrap();
    assert!(d.nodes.len() > 1);
    let encoded = store::encode(&d).unwrap();
    let restored = store::decode(&encoded).unwrap();
    assert_eq!(
        serde_json::to_value(d).unwrap(),
        serde_json::to_value(restored).unwrap()
    );
}
#[test]
fn legacy_map_unknown_fields_binary_and_text_survive() {
    let mut d = Document::new("legacy");
    d.extra.insert("futureContract".into(), json!({"v":42}));
    let root = d.nodes.get_mut("root").unwrap();
    root.binary_content = Some("AAEC/w==".into());
    root.text
        .extra
        .insert("futureSection".into(), json!("keep"));
    let restored = store::decode(&serde_json::to_string(&d).unwrap()).unwrap();
    let restored = store::decode(&store::encode(&restored).unwrap()).unwrap();
    assert_eq!(restored.extra["futureContract"]["v"], 42);
    assert_eq!(
        restored.nodes["root"].binary_content.as_deref(),
        Some("AAEC/w==")
    );
    assert_eq!(restored.nodes["root"].text.extra["futureSection"], "keep");
}
#[test]
fn link_indexes_boundaries_moves_self_links_and_cascade() {
    let mut d = Document::new("flow");
    let g = d.create("root", "group", NodeKind::Group).unwrap();
    let a = d.create(&g, "producer", NodeKind::Processor).unwrap();
    let b = d.create("root", "consumer", NodeKind::Processor).unwrap();
    let l = d
        .create_link(
            "packet",
            Link {
                source_node_id: a.clone(),
                target_node_id: b.clone(),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(d.nodes[&l].parent_id.as_deref(), Some("root"));
    assert_eq!(
        d.nodes[&l].link.as_ref().unwrap().composite_boundary_ids,
        vec![g.clone()]
    );
    d.move_node(&b, Some(&g)).unwrap();
    assert_eq!(d.nodes[&l].parent_id.as_deref(), Some(g.as_str()));
    d.ensure_valid().unwrap();
    let self_link = d
        .create_link(
            "feedback",
            Link {
                source_node_id: a.clone(),
                target_node_id: a.clone(),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(d.nodes[&a].incoming_links.contains(&self_link));
    assert!(d.move_node(&g, Some(&a)).is_err());
    d.delete(&g).unwrap();
    assert_eq!(d.nodes.len(), 1);
    assert!(d.delete("root").is_err());
    d.ensure_valid().unwrap();
}
#[test]
fn corrupt_and_duplicate_documents_fail() {
    let mut v = serde_json::to_value(Document::new("bad")).unwrap();
    let root = v["nodes"]["root"].clone();
    v["nodes"] = json!([root.clone(), root]);
    assert!(store::decode(&v.to_string()).is_err());
    let mut d = Document::new("cycle");
    let a = d.create("root", "a", NodeKind::Group).unwrap();
    let b = d.create(&a, "b", NodeKind::Group).unwrap();
    d.nodes.get_mut(&a).unwrap().parent_id = Some(b);
    assert!(store::decode(&serde_json::to_string(&d).unwrap()).is_err());
}
#[test]
fn save_is_atomic_and_can_be_reopened() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("design.orch");
    let d = Document::new("persist");
    store::save(&d, &path).unwrap();
    assert_eq!(store::load(&path).unwrap().name, "persist");
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
}
#[test]
fn cursor_zoom_is_invariant_and_culling_is_local() {
    let mut v = Viewport::default();
    let p = [670., 345.];
    let old = v.world(p);
    v.zoom_at(p, 1.5);
    assert_eq!(v.world(p), old);
    let mut index = SceneIndex::default();
    for i in 0..10_000 {
        index.insert(
            i.to_string(),
            Rect {
                x: (i % 100) as f64 * 250.,
                y: (i / 100) as f64 * 100.,
                width: 200.,
                height: 70.,
            },
        );
    }
    let visible = index.query(Rect {
        x: 0.,
        y: 0.,
        width: 1000.,
        height: 700.,
    });
    assert!(visible.len() < 50);
    assert!(!visible.is_empty());
}
