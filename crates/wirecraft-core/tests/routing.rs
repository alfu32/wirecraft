use serde_json::json;
use wirecraft_core::{Document, Link, NodeKind, drawing::Drawing, viewport::Rect};
fn box_at(d: &mut Document, parent: &str, name: &str, x: f64, y: f64) -> String {
    let id = d.create(parent, name, NodeKind::Processor).unwrap();
    d.patch(
        &id,
        &json!({"layout":{"x":x,"y":y,"width":100.,"height":100.}}),
    )
    .unwrap();
    id
}
fn wire(d: &mut Document, a: &str, b: &str, name: &str) -> String {
    d.create_link(
        name,
        Link {
            source_node_id: a.into(),
            target_node_id: b.into(),
            interaction_kind: "data".into(),
            ..Default::default()
        },
    )
    .unwrap()
}
fn assert_routes(drawing: &Drawing) {
    let mut segments: Vec<(&str, [f64; 2], [f64; 2])> = vec![];
    for (id, edge) in &drawing.edges {
        if edge.annotation_only {
            assert!(edge.points.is_empty());
            continue;
        }
        assert!(edge.points.len() >= 2, "Unrouted {}", edge.name);
        for pair in edge.points.windows(2) {
            let a = pair[0];
            let b = pair[1];
            let dx = (a[0] - b[0]).abs();
            let dy = (a[1] - b[1]).abs();
            assert!(
                dx < 1e-6 || dy < 1e-6 || (dx - dy).abs() < 1e-6,
                "Non snapped segment {id}: {pair:?}"
            );
            for (node, shape) in &drawing.shapes {
                if shape.composite && shape.expanded {
                    continue;
                }
                // Endpoints may touch the box boundary, but never its interior.
                let r = shape.rect;
                let inside = Rect {
                    x: r.x + 0.01,
                    y: r.y + 0.01,
                    width: r.width - 0.02,
                    height: r.height - 0.02,
                };
                assert!(
                    !inside.intersects_segment(a, b),
                    "{} pierces {} at {pair:?}",
                    edge.name,
                    node
                );
            }
            for (other, c, e) in &segments {
                if *other == id {
                    continue;
                }
                let cross = (b[0] - a[0]) * (e[1] - c[1]) - (b[1] - a[1]) * (e[0] - c[0]);
                let collinear = (c[0] - a[0]) * (b[1] - a[1]) - (c[1] - a[1]) * (b[0] - a[0]);
                if cross.abs() < 1e-6 && collinear.abs() < 1e-6 {
                    let axis = if dx > dy { 0 } else { 1 };
                    let overlap = a[axis].max(b[axis]).min(c[axis].max(e[axis]))
                        - a[axis].min(b[axis]).max(c[axis].min(e[axis]));
                    assert!(
                        overlap < 1e-6,
                        "Channels overlap {id} {other}: {pair:?} {c:?} {e:?}"
                    );
                }
            }
            segments.push((id, a, b));
        }
    }
}
#[test]
fn imported_webserver_routes_avoid_boxes_separate_lanes_and_use_boundary_ports() {
    let d = wirecraft_core::store::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/webserver.orch"),
    )
    .unwrap();
    let drawing = Drawing::new(&d, "root");
    assert_routes(&drawing);
    assert_eq!(drawing.boundary_ports.len(), 6);
    let group = drawing
        .shapes
        .iter()
        .find(|(_, s)| s.name == "routes")
        .unwrap();
    let mut ys = drawing
        .boundary_ports
        .iter()
        .map(|p| {
            assert_eq!(p.owner, *group.0);
            assert_eq!(p.point[0], group.1.rect.x);
            p.point[1]
        })
        .collect::<Vec<_>>();
    ys.sort_by(f64::total_cmp);
    for p in ys.windows(2) {
        assert!(p[1] - p[0] > 10.);
    }
    // Non-emitting link layout metadata must not block C editor context.
    let id = d
        .nodes
        .values()
        .find(|n| n.name == "router")
        .unwrap()
        .id
        .clone();
    wirecraft_core::editor::body_context(&d, &id, "source", None).unwrap();
}
#[test]
fn nearer_side_ports_follow_angular_order_and_detour_around_obstacles() {
    let mut d = Document::new("angles");
    let source = box_at(&mut d, "root", "Source", 500., 200.);
    let top = box_at(&mut d, "root", "Top", 0., 0.);
    let bottom = box_at(&mut d, "root", "Bottom", 0., 400.);
    let middle = box_at(&mut d, "root", "Middle", 0., 200.);
    box_at(&mut d, "root", "Obstacle", 250., 180.);
    let lower = wire(&mut d, &source, &bottom, "created-first");
    let upper = wire(&mut d, &source, &top, "created-second");
    let mid = wire(&mut d, &source, &middle, "created-third");
    let drawing = Drawing::new(&d, "root");
    assert_routes(&drawing);
    for id in [&lower, &upper, &mid] {
        assert_eq!(drawing.edges[id].points[0][0], 500.);
        assert_eq!(drawing.edges[id].points.last().unwrap()[0], 100.);
    }
    assert!(drawing.edges[&upper].points[0][1] < drawing.edges[&mid].points[0][1]);
    assert!(drawing.edges[&mid].points[0][1] < drawing.edges[&lower].points[0][1]);
}

#[test]
fn dependency_callouts_are_separate_from_data_routes_and_issue_dots_include_warnings_and_exclude_info()
 {
    let d = wirecraft_core::store::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/migration/threadworks/example-c.orch"),
    )
    .unwrap();
    let drawing = Drawing::new(&d, "root");
    let includes = d.nodes.values().find(|n| n.name == "includes_lib").unwrap();
    let sink = d
        .nodes
        .values()
        .find(|n| n.name == "write_append_file")
        .unwrap();
    let boxes = drawing
        .annotations
        .values()
        .filter(|a| a.owner == includes.id && !a.flag)
        .collect::<Vec<_>>();
    assert_eq!(boxes.len(), 4);
    for a in boxes {
        assert!(
            a.rect.x
                > drawing.shapes[&includes.id].rect.x + drawing.shapes[&includes.id].rect.width
        );
        assert!(a.label.ends_with(": includes_lib"));
    }
    let flags = drawing
        .annotations
        .values()
        .filter(|a| a.owner == sink.id && a.flag)
        .collect::<Vec<_>>();
    assert_eq!(flags.len(), 2);
    assert!(flags.iter().any(|a| a.label == "files: file_lib"));
    assert!(
        flags
            .iter()
            .any(|a| a.label == "argslib: proc_self_argv_lib")
    );
    assert!(
        flags
            .iter()
            .all(|a| a.rect.y < drawing.shapes[&sink.id].rect.y)
    );
    for edge in drawing.edges.values().filter(|e| e.annotation_only) {
        assert!(edge.points.is_empty());
    }
    assert_routes(&drawing);
    let mut d = Document::new("diagnostics");
    let id = box_at(&mut d, "root", "Body", 0., 0.);
    d.nodes.get_mut(&id).unwrap().diagnostics =
        vec![json!({"severity":"Info"}), json!({"severity":3})];
    assert_eq!(Drawing::new(&d, "root").shapes[&id].diagnostics, 0);
    d.nodes
        .get_mut(&id)
        .unwrap()
        .diagnostics
        .push(json!({"severity":"Error","textSection":"Source"}));
    assert_eq!(Drawing::new(&d, "root").shapes[&id].diagnostics, 1);
    d.nodes
        .get_mut(&id)
        .unwrap()
        .diagnostics
        .extend([json!({"severity":"Warning"}), json!({"severity":2})]);
    assert_eq!(Drawing::new(&d, "root").shapes[&id].diagnostics, 3);
}

#[test]
fn obstructed_endpoints_report_failure_instead_of_silently_drawing_through_a_box() {
    let mut d = Document::new("overlap");
    let a = box_at(&mut d, "root", "A", 0., 0.);
    let b = box_at(&mut d, "root", "B", 0., 0.);
    d.patch(
        &b,
        &json!({"layout":{"x":-100.,"y":-100.,"width":400.,"height":400.}}),
    )
    .unwrap();
    let link = wire(&mut d, &a, &b, "overlap");
    let drawing = Drawing::new(&d, "root");
    assert!(drawing.edges[&link].route_error.is_some());
    assert!(
        wirecraft_core::export::drawing_svg(&d, "root")
            .unwrap()
            .contains("routing-error")
    );
}
