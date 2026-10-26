use serde_json::json;
use wirecraft_core::{analysis, export, fragment::Fragment, *};
#[test]
fn clipboard_remaps_types_and_internal_links_without_rewriting_user_text() {
    let mut d = Document::new("source");
    let a = d.create("root", "a", NodeKind::Processor).unwrap();
    let b = d.create("root", "b", NodeKind::Processor).unwrap();
    let external = d.create("root", "external", NodeKind::Processor).unwrap();
    let t = d.create("root", "Packet", NodeKind::Type).unwrap();
    d.patch(&a, &json!({"text":{"source":t},"typeDefinition":null}))
        .unwrap();
    let wire = d
        .create_link(
            "packet",
            Link {
                source_node_id: a.clone(),
                target_node_id: b.clone(),
                type_definition_id: t.clone(),
                ..Default::default()
            },
        )
        .unwrap();
    d.create_link(
        "outside",
        Link {
            source_node_id: b.clone(),
            target_node_id: external,
            ..Default::default()
        },
    )
    .unwrap();
    let fragment = Fragment::copy(&d, &[a, b, t.clone()]).unwrap();
    assert_eq!(fragment.nodes.len(), 4);
    let mut target = Document::new("target");
    let roots = fragment.paste(&mut target, "root", [30., 40.]).unwrap();
    assert_eq!(roots.len(), 3);
    target.ensure_valid().unwrap();
    assert_eq!(target.nodes.len(), 5);
    assert!(!target.nodes.contains_key(&wire));
    let copied_a = target.nodes.values().find(|n| n.name == "a").unwrap();
    assert_eq!(copied_a.text.source, t);
    assert_eq!(copied_a.layout.x, 30.);
    let copied_type = target.nodes.values().find(|n| n.name == "Packet").unwrap();
    let copied_link = target.nodes.values().find_map(|n| n.link.as_ref()).unwrap();
    assert_eq!(copied_link.type_definition_id, copied_type.id);
}
#[test]
fn analysis_separates_dependencies_errors_and_feedback() {
    let mut d = Document::new("network");
    let a = d.create("root", "generator", NodeKind::Processor).unwrap();
    let b = d.create("root", "worker", NodeKind::Processor).unwrap();
    let err = d
        .create("root", "error-handler", NodeKind::Processor)
        .unwrap();
    let lib = d
        .create("root", "math-library", NodeKind::Processor)
        .unwrap();
    for (name, src, dst, kind) in [
        ("data", &a, &b, "data"),
        ("feedback", &b, &a, "data"),
        ("failure", &b, &err, "data"),
        ("use", &lib, &b, "lib"),
    ] {
        d.create_link(
            name,
            Link {
                source_node_id: src.clone(),
                target_node_id: dst.clone(),
                interaction_kind: kind.into(),
                ..Default::default()
            },
        )
        .unwrap();
    }
    let report = analysis::report(&d);
    assert_eq!(report["graphs"]["principal"]["edges"], 2);
    assert_eq!(report["graphs"]["error"]["edges"], 1);
    assert_eq!(report["graphs"]["dependency"]["edges"], 1);
    assert_eq!(
        report["graphs"]["principal"]["cycles"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        analysis::stereotype(&d, d.node(&lib).unwrap()),
        "ServiceLibrary"
    );
}
#[test]
fn validation_inheritance_and_status_history() {
    let mut d = Document::new("types");
    d.patch("root",&json!({"technology":{"languageId":"c","technologyId":"c-native"},"fileLayoutStrategyId":"single-file","text":{"sourceLanguageId":"cpp"}})).unwrap();
    let a = d.create("root", "a", NodeKind::Processor).unwrap();
    assert_eq!(d.effective(&a, "languageId"), "c");
    assert_eq!(d.effective_text_language(&a, "source"), "cpp");
    assert_eq!(d.effective_text_language(&a, "specification"), "markdown");
    assert_eq!(d.nodes[&a].status_changes.len(), 1);
    d.patch(
        &a,
        &json!({"status":"TESTING","technology":{"technologyId":"file-export"}}),
    )
    .unwrap();
    assert_eq!(d.effective(&a, "languageId"), "plain");
    assert_eq!(d.effective(&a, "fileLayoutStrategyId"), "none");
    assert_eq!(d.nodes[&a].status_changes.len(), 2);
    let t = d.create("root", "map", NodeKind::Type).unwrap();
    d.patch(
        &t,
        &json!({"typeDefinition":{"qualifier":"map","genericTypeIds":["string"],"fields":[]}}),
    )
    .unwrap();
    assert!(d.ensure_valid().is_err());
}
#[test]
fn vector_and_markdown_exports_escape_untrusted_content() {
    let mut d = Document::new("<project & title>");
    let a = d
        .create("root", "<script>alert(1)</script>", NodeKind::Processor)
        .unwrap();
    d.patch(&a, &json!({"text":{"source":"```\nint a;\n```"}}))
        .unwrap();
    let svg = export::svg(&d, "root").unwrap();
    assert!(svg.contains("&lt;script&gt;"));
    assert!(!svg.contains("<script>"));
    let md = export::markdown(&d, &[a]).unwrap();
    assert!(md.contains("````plain"));
    assert!(md.contains("int a;"));
}

#[test]
fn repaired_endpoint_port_ids_match_the_source_naming_contract() {
    let mut d = Document::new("ports");
    let a = d.create("root", "a", NodeKind::Processor).unwrap();
    let b = d.create("root", "b", NodeKind::Processor).unwrap();
    d.create_link(
        "data",
        Link {
            source_node_id: a.clone(),
            target_node_id: b.clone(),
            source_port_name: "packet out".into(),
            target_port_name: "packet in".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(d.nodes[&a].ports[0].id, "output_packet_out");
    assert_eq!(d.nodes[&b].ports[0].id, "input_packet_in");
}

#[test]
fn composite_projection_keeps_absolute_children_and_collapses_boundary_links() {
    use wirecraft_core::{Link, drawing::Drawing};
    let mut d = Document::new("Composite");
    let composite = d.create("root", "Group", NodeKind::Processor).unwrap();
    let child = d.create(&composite, "Child", NodeKind::Processor).unwrap();
    let peer = d.create("root", "Peer", NodeKind::Processor).unwrap();
    d.patch(&composite,&json!({"layout":{"x":100.,"y":100.,"width":600.,"height":400.,"closedWidth":200.,"closedHeight":70.}})).unwrap();
    d.patch(&child, &json!({"layout":{"x":200.,"y":200.}}))
        .unwrap();
    d.patch(&peer, &json!({"layout":{"x":900.,"y":200.}}))
        .unwrap();
    let link = d
        .create_link(
            "result",
            Link {
                source_node_id: child.clone(),
                target_node_id: peer.clone(),
                ..Default::default()
            },
        )
        .unwrap();
    let open = Drawing::new(&d, "root");
    assert_eq!(open.shapes[&child].rect.x, 200.);
    assert_eq!(open.shapes[&child].depth, 1);
    assert_eq!(open.edges[&link].points[0][0], 400.);
    d.patch(&composite, &json!({"layout":{"isExpanded":false}}))
        .unwrap();
    let closed = Drawing::new(&d, "root");
    assert!(!closed.shapes.contains_key(&child));
    assert_eq!(closed.edges[&link].points[0][0], 300.);
    let focused = Drawing::new(&d, &composite);
    assert!(focused.shapes.contains_key(&child));
    assert!(!focused.shapes.contains_key(&peer));
}
