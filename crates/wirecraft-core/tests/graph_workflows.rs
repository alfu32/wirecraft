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
    let child_rect = open.shapes[&child].rect;
    // The output cap sits 20 units outside the auto-sized child.
    assert_eq!(
        open.edges[&link].points[0][0],
        child_rect.x + child_rect.width + 20.
    );
    // The expanded composite encloses its child with the default padding and header.
    let group = open.shapes[&composite].rect;
    assert_eq!(group.x, child_rect.x - drawing::COMPOSITE_PADDING);
    assert_eq!(
        group.x + group.width,
        child_rect.x + child_rect.width + drawing::COMPOSITE_PADDING
    );
    assert!(group.y + 60. < child_rect.y);
    assert_eq!(
        group.y + group.height,
        child_rect.y + child_rect.height + drawing::COMPOSITE_PADDING
    );
    d.patch(&composite, &json!({"layout":{"isExpanded":false}}))
        .unwrap();
    let closed = Drawing::new(&d, "root");
    assert!(!closed.shapes.contains_key(&child));
    let closed_rect = closed.shapes[&composite].rect;
    assert_eq!(
        closed.edges[&link].points[0][0],
        closed_rect.x + closed_rect.width + 20.
    );
    d.patch(
        &composite,
        &json!({"layout":{"isExpanded":true,"padding":10.}}),
    )
    .unwrap();
    // 0.8.0 stored one number for every side.
    let tight = Drawing::new(&d, "root");
    assert_eq!(tight.shapes[&composite].rect.x, child_rect.x - 10.);
    d.patch(
        &composite,
        &json!({"layout":{"padding":{"top":5.,"right":30.,"bottom":7.,"left":100.}}}),
    )
    .unwrap();
    let sides = Drawing::new(&d, "root").shapes[&composite].rect;
    assert_eq!(sides.x, child_rect.x - 100.);
    assert_eq!(sides.x + sides.width, child_rect.x + child_rect.width + 30.);
    assert_eq!(sides.y, child_rect.y - 5. - 64.);
    assert_eq!(
        sides.y + sides.height,
        child_rect.y + child_rect.height + 7.
    );
    // Missing sides fall back to the default.
    d.patch(&composite, &json!({"layout":{"padding":{"left":0.}}}))
        .unwrap();
    let partial = Drawing::new(&d, "root").shapes[&composite].rect;
    assert_eq!(partial.x, child_rect.x);
    assert_eq!(
        partial.x + partial.width,
        child_rect.x + child_rect.width + drawing::COMPOSITE_PADDING
    );
    let focused = Drawing::new(&d, &composite);
    assert!(focused.shapes.contains_key(&child));
    assert!(!focused.shapes.contains_key(&peer));
}

#[test]
fn type_usage_callouts_reference_entities_and_are_exported() {
    let mut d = Document::new("Types");
    let packet = d.create("root", "Packet", NodeKind::Type).unwrap();
    let envelope = d.create("root", "Envelope", NodeKind::Type).unwrap();
    let a = d.create("root", "producer", NodeKind::Processor).unwrap();
    let b = d.create("root", "consumer", NodeKind::Processor).unwrap();
    d.patch(&envelope,&json!({"typeDefinition":{"fields":[{"name":"payload","typeId":packet}],"qualifier":"object"}})).unwrap();
    let link = d
        .create_link(
            "messages",
            Link {
                source_node_id: a,
                target_node_id: b,
                type_definition_id: packet.clone(),
                ..Default::default()
            },
        )
        .unwrap();
    let drawing = wirecraft_core::drawing::Drawing::new(&d, "root");
    let usages = drawing
        .annotations
        .values()
        .filter(|a| a.owner == packet)
        .collect::<Vec<_>>();
    assert_eq!(usages.len(), 2);
    assert!(
        usages
            .iter()
            .any(|a| a.target == link && a.label == "messages")
    );
    assert!(
        usages
            .iter()
            .any(|a| a.target == envelope && a.label == "Envelope:payload")
    );
    let svg = export::svg(&d, "root").unwrap();
    assert!(svg.contains("Envelope:payload"));
    assert!(svg.contains("messages"));
}

#[test]
fn dependency_matrix_reverses_capability_direction_and_ignores_source_capabilities() {
    let mut d = Document::new("Layers");
    let lib = d.create("root", "shared_lib", NodeKind::Processor).unwrap();
    let worker = d.create("root", "worker", NodeKind::Processor).unwrap();
    d.create_link(
        "library",
        Link {
            source_node_id: lib.clone(),
            target_node_id: worker.clone(),
            interaction_kind: "lib".into(),
            ..Default::default()
        },
    )
    .unwrap();
    d.create_link(
        "source",
        Link {
            source_node_id: lib.clone(),
            target_node_id: worker.clone(),
            interaction_kind: "src".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let r = analysis::report(&d);
    assert_eq!(
        r["graphs"]["dependency"]["adjacency"][&worker],
        json!([lib])
    );
    assert_eq!(r["graphs"]["structural"]["edges"], 1);
    assert_eq!(r["graphs"]["principal"]["edges"], 0);
    assert!(
        r["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["title"] == "Select library candidates")
    );
}

#[test]
fn routed_link_index_culls_empty_interiors_and_matches_segment_bounds() {
    use wirecraft_core::viewport::{Rect, RouteIndex};
    let names = (0..200)
        .map(|i| format!("route-{i:03}"))
        .collect::<Vec<_>>();
    let routes = names
        .iter()
        .enumerate()
        .map(|(i, id)| {
            (
                id.clone(),
                vec![
                    [i as f64 * 100., -50.],
                    [i as f64 * 100. + 300., -50.],
                    [i as f64 * 100. + 300., 250.],
                ],
            )
        })
        .collect::<Vec<_>>();
    let index = RouteIndex::new(routes.iter().map(|(id, p)| (id.as_str(), p.as_slice())));
    for i in 0..300 {
        let query = Rect {
            x: i as f64 * 73. - 100.,
            y: (i % 9) as f64 * 47. - 90.,
            width: 80.,
            height: 60.,
        };
        let actual = index
            .query(query)
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let expected = routes
            .iter()
            .filter(|(_, p)| {
                p.windows(2).any(|pair| {
                    Rect {
                        x: pair[0][0].min(pair[1][0]) - 6.,
                        y: pair[0][1].min(pair[1][1]) - 24.,
                        width: (pair[0][0] - pair[1][0]).abs() + 12.,
                        height: (pair[0][1] - pair[1][1]).abs() + 48.,
                    }
                    .intersects(query)
                })
            })
            .map(|(id, _)| id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual, expected);
    }
    let points = [[0., 0.], [1_000_000., 0.], [1_000_000., 1_000_000.]];
    let index = RouteIndex::new(std::iter::once(("perimeter", points.as_slice())));
    assert!(
        index
            .query(Rect {
                x: 499_000.,
                y: 499_000.,
                width: 2000.,
                height: 2000.
            })
            .is_empty()
    );
    assert_eq!(
        index.query(Rect {
            x: 999_995.,
            y: -10.,
            width: 20.,
            height: 40.
        }),
        vec!["perimeter"]
    );
}

#[test]
fn sheet_world_contours_cover_the_drawing_and_follow_split_tiles() {
    let mut d = Document::new("sheets");
    for (i, name) in ["a", "b", "c"].iter().enumerate() {
        let id = d.create("root", name, NodeKind::Processor).unwrap();
        d.patch(
            &id,
            &json!({"layout":{"x":i as f64 * 900.,"y":i as f64 * 500.,"width":180.,"height":90.}}),
        )
        .unwrap();
    }
    let drawing = drawing::Drawing::new(&d, "root");
    let single = sheets::plan(&d, "root", &sheets::SheetOptions::default()).unwrap();
    assert_eq!(single.world_tiles.len(), 1);
    let page = single.world_tiles[0];
    for shape in drawing.shapes.values() {
        let r = shape.rect;
        assert!(
            r.x >= page.x
                && r.y >= page.y
                && r.x + r.width <= page.x + page.width
                && r.y + r.height <= page.y + page.height,
            "shape outside contour"
        );
    }
    let split = sheets::plan(
        &d,
        "root",
        &sheets::SheetOptions {
            format: "A4".into(),
            split: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(split.world_tiles.len() > 1);
    assert_eq!(split.world_tiles.len(), split.tiles.len());
    // Neighbouring pages overlap by exactly the configured millimetres (4 world units/mm).
    let step = split.world_tiles[1].x - split.world_tiles[0].x;
    assert!((split.world_tiles[0].width - step - 5. * 4.).abs() < 1e-6);
}

#[test]
fn simple_boxes_fit_their_text_and_lateral_port_rows() {
    use wirecraft_core::{Link, drawing::Drawing};
    let mut d = Document::new("Sizing");
    let hub = d.create("root", "hub", NodeKind::Processor).unwrap();
    let long = d
        .create(
            "root",
            "a_processor_with_a_rather_long_descriptive_name",
            NodeKind::Processor,
        )
        .unwrap();
    d.patch(
        &hub,
        &json!({"layout":{"x":0.,"y":0.,"width":500.,"height":30.}}),
    )
    .unwrap();
    d.patch(
        &long,
        &json!({"layout":{"x":-900.,"y":0.,"width":50.,"height":500.}}),
    )
    .unwrap();
    let mut sinks = vec![];
    for i in 0..6 {
        let sink = d
            .create("root", &format!("sink{i}"), NodeKind::Processor)
            .unwrap();
        d.patch(&sink, &json!({"layout":{"x":900.,"y":i as f64 * 200.}}))
            .unwrap();
        d.create_link(
            &format!("out{i}"),
            Link {
                source_node_id: hub.clone(),
                target_node_id: sink.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        sinks.push(sink);
    }
    let drawing = Drawing::new(&d, "root");
    let hub_rect = drawing.shapes[&hub].rect;
    let long_rect = drawing.shapes[&long].rect;
    // Width follows the text, not the stored 500/50 units.
    assert!(hub_rect.width < 200., "{}", hub_rect.width);
    assert!(long_rect.width > 14. * 0.6 * 40., "{}", long_rect.width);
    // Six right-side ports stay within the box with at least one pitch between rows.
    assert!(hub_rect.height >= 2. * 12. + 7. * drawing::PORT_PITCH);
    for id in d.nodes[&hub].outgoing_links.clone() {
        let y = drawing.edges[&id].points[0][1];
        assert!(y > hub_rect.y && y < hub_rect.y + hub_rect.height);
    }
    // A box without ports keeps the minimum text height, regardless of the stored 500.
    assert_eq!(long_rect.height, 73.);
    // The stored model layout is untouched by the projection.
    assert_eq!(d.nodes[&hub].layout.width, 500.);
}
