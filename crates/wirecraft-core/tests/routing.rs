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
            let route = &drawing.edges[&p.edge].points;
            assert!(
                route.windows(2).any(|s| {
                    (s[0][1] - p.point[1]).abs() < 1e-6
                        && (s[1][1] - p.point[1]).abs() < 1e-6
                        && s[0][0].min(s[1][0]) <= p.point[0] - 45.9
                        && s[0][0].max(s[1][0]) >= p.point[0] + 45.9
                }),
                "Composite crossing lost its aligned port stubs: {route:?}"
            );
            p.point[1]
        })
        .collect::<Vec<_>>();
    ys.sort_by(f64::total_cmp);
    for p in ys.windows(2) {
        assert!(p[1] - p[0] > 10.);
    }
    let r = group.1.rect;
    let mut fan = drawing
        .boundary_ports
        .iter()
        .map(|port| {
            let edge = &drawing.edges[&port.edge];
            let inside = if drawing.shapes[&edge.source].depth > group.1.depth {
                edge.points.first().unwrap()
            } else {
                edge.points.last().unwrap()
            };
            (
                port.point[1],
                (inside[1] - r.y - r.height / 2.).atan2((inside[0] - r.x - r.width / 2.).abs()),
            )
        })
        .collect::<Vec<_>>();
    fan.sort_by(|a, b| a.0.total_cmp(&b.0));
    for pair in fan.windows(2) {
        assert!(
            pair[0].1 <= pair[1].1 + 1e-9,
            "Composite fan out of angular order: {fan:?}"
        );
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
        assert_eq!(drawing.edges[id].points[0][0], 480.);
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
fn obstructed_layout_keeps_a_best_effort_route_without_error_labels() {
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
    let edge = &drawing.edges[&link];
    assert!(edge.route_error.is_none());
    assert!(edge.points.len() >= 2);
    assert!(
        !wirecraft_core::export::drawing_svg(&d, "root")
            .unwrap()
            .contains("routing-error")
    );
}

#[test]
fn open_routes_choose_substantial_diagonals_in_all_quadrants_and_vector_exports() {
    for (x, y) in [
        (600., 400.),
        (600., -400.),
        (-600., 400.),
        (-600., -400.),
        (400., 600.),
    ] {
        let mut d = Document::new("Diagonal routing");
        let a = box_at(&mut d, "root", "A", 0., 0.);
        let b = box_at(&mut d, "root", "B", x, y);
        let id = wire(&mut d, &a, &b, "channel");
        let drawing = Drawing::new(&d, "root");
        assert_routes(&drawing);
        let points = &drawing.edges[&id].points;
        let mut total = 0.;
        let mut diagonal = 0.;
        for p in points.windows(2) {
            let dx = (p[1][0] - p[0][0]).abs();
            let dy = (p[1][1] - p[0][1]).abs();
            let len = dx.hypot(dy);
            total += len;
            if dx > 1e-6 && dy > 1e-6 {
                diagonal += len;
            }
        }
        let first = points.first().unwrap();
        let last = points.last().unwrap();
        let dx = (last[0] - first[0]).abs() - 92.;
        let dy = (last[1] - first[1]).abs();
        let possible = dx.min(dy) * std::f64::consts::SQRT_2;
        assert!(
            (diagonal - possible).abs() < 1e-6,
            "Diagonal does not span the available displacement: {points:?}"
        );
        let optimum = 92. + dx.max(dy) - dx.min(dy) + possible;
        assert!(
            (total - optimum).abs() < 1e-6,
            "Route has unnecessary H/V detours: {points:?}"
        );
        let svg = wirecraft_core::export::drawing_svg(&d, "root").unwrap();
        let polyline = points
            .iter()
            .map(|p| format!("{},{}", p[0], p[1]))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            svg.contains(&polyline),
            "Vector export must use the shared route"
        );
    }
}

#[test]
fn feedback_uses_the_less_crowded_side_before_adding_both_ports_and_prefers_right_on_ties() {
    for (left_count, right_count) in [(0, 0), (1, 0), (0, 1), (1, 1), (2, 1), (1, 2)] {
        let mut d = Document::new("Same-side feedback");
        let a = box_at(&mut d, "root", "A", 500., 300.);
        let left = box_at(&mut d, "root", "Left", 0., 300.);
        let right = box_at(&mut d, "root", "Right", 1100., 300.);
        let feedback = wire(&mut d, &a, &a, "feedback");
        d.nodes.get_mut(&feedback).unwrap().layout.y = 10000.;
        for i in 0..left_count {
            wire(&mut d, &left, &a, &format!("incoming{i}"));
        }
        for i in 0..right_count {
            wire(&mut d, &a, &right, &format!("outgoing{i}"));
        }
        let drawing = Drawing::new(&d, "root");
        assert_routes(&drawing);
        let r = drawing.shapes[&a].rect;
        let points = &drawing.edges[&feedback].points;
        let on_right = right_count <= left_count;
        let dir = if on_right { 1. } else { -1. };
        let side = if on_right { r.x + r.width } else { r.x };
        assert_eq!(points[0][0], side + dir * 20.);
        assert_eq!(points.last().unwrap()[0], side);
        assert!(
            points[0][1] > points.last().unwrap()[1],
            "Input must be above output"
        );
        for p in points {
            assert!(
                (p[0] - side) * dir >= -1e-6 && (p[0] - side) * dir <= 88. + 1e-6,
                "Feedback escaped its side envelope: {points:?}"
            );
            assert!(p[1] >= points.last().unwrap()[1] && p[1] <= points[0][1]);
        }
        for p in [points[..2].to_vec(), points[points.len() - 2..].to_vec()] {
            assert!((p[0][1] - p[1][1]).abs() < 1e-6);
            assert!((p[0][0] - p[1][0]).abs() >= 46. - 1e-6);
        }
        // Feedback port pairs precede ordinary channels and remain adjacent.
        for (id, edge) in &drawing.edges {
            if id == &feedback {
                continue;
            }
            for endpoint in [edge.points.first(), edge.points.last()]
                .into_iter()
                .flatten()
            {
                if (endpoint[0] - side).abs() < 21. {
                    assert!(endpoint[1] > points[0][1]);
                }
            }
        }
        let svg = wirecraft_core::export::drawing_svg(&d, "root").unwrap();
        assert!(svg.contains("class=\"output-port\""));
    }
}
#[test]
fn feedback_counts_right_provider_boxes_but_not_top_dependency_flags() {
    for provider in [false, true] {
        let mut d = Document::new("Capability occupancy");
        let a = box_at(&mut d, "root", "A", 500., 300.);
        let peer = box_at(&mut d, "root", "Peer", 1100., 300.);
        d.create_link(
            "capability",
            Link {
                source_node_id: if provider { a.clone() } else { peer.clone() },
                target_node_id: if provider { peer } else { a.clone() },
                interaction_kind: "lib".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let feedback = wire(&mut d, &a, &a, "feedback");
        let drawing = Drawing::new(&d, "root");
        assert_routes(&drawing);
        let r = drawing.shapes[&a].rect;
        assert_eq!(
            drawing.edges[&feedback].points[0][0] > r.x + r.width,
            !provider
        );
    }
}
#[test]
fn successive_feedback_pairs_balance_occupancy_without_overlapping_channels() {
    let mut d = Document::new("Feedback pairs");
    let a = box_at(&mut d, "root", "A", 500., 300.);
    let loops = (0..4)
        .map(|i| wire(&mut d, &a, &a, &format!("feedback{i}")))
        .collect::<Vec<_>>();
    let drawing = Drawing::new(&d, "root");
    assert_routes(&drawing);
    let r = drawing.shapes[&a].rect;
    let right = loops
        .iter()
        .filter(|id| drawing.edges[*id].points[0][0] > r.x + r.width)
        .count();
    assert_eq!(right, 2);
    for id in &loops {
        let p = &drawing.edges[id].points;
        assert_eq!(
            p[0][0] > r.x + r.width / 2.,
            p.last().unwrap()[0] > r.x + r.width / 2.
        );
        assert!(p.iter().all(|v| v[1] >= r.y && v[1] <= r.y + r.height));
    }
}
#[test]
fn endpoint_caps_match_original_templates_and_horizontal_leads_are_46_units() {
    use wirecraft_core::drawing::port_glyphs;
    let (output, input) = port_glyphs(&[[120., 50.], [600., 50.]]);
    assert_eq!(
        output,
        vec![
            [100.2, 45.2],
            [115.2, 45.2],
            [120., 50.],
            [115.2, 54.8],
            [100.2, 54.8]
        ]
    );
    assert_eq!(
        input,
        vec![
            [580.2, 45.2],
            [585., 50.],
            [580.2, 54.8],
            [600., 54.8],
            [600., 45.2]
        ]
    );
    let (reverse_output, reverse_input) = port_glyphs(&[[120., 50.], [0., 50.]]);
    for (right, left) in output.iter().zip(reverse_output) {
        assert!((left[0] - (240. - right[0])).abs() < 1e-6);
        assert!((left[1] - (100. - right[1])).abs() < 1e-6);
    }
    assert_eq!(
        reverse_input,
        vec![
            [19.8, 54.8],
            [15., 50.],
            [19.8, 45.2],
            [0., 45.2],
            [0., 54.8]
        ]
    );
    let mut d = Document::new("Lead lengths");
    let a = box_at(&mut d, "root", "A", 0., 0.);
    let b = box_at(&mut d, "root", "B", 600., 400.);
    let edge = wire(&mut d, &a, &b, "channel");
    let drawing = Drawing::new(&d, "root");
    let points = &drawing.edges[&edge].points;
    for p in [points[..2].to_vec(), points[points.len() - 2..].to_vec()] {
        assert!((p[0][1] - p[1][1]).abs() < 1e-6);
        assert!((p[0][0] - p[1][0]).abs() >= 46. - 1e-6);
    }
}
#[test]
fn pcb_crossings_have_two_hollow_vias_with_a_backside_connector_and_unbroken_peer() {
    use wirecraft_core::drawing::{flow_arrow, trace_geometry};
    let horizontal = [[0., 100.], [200., 100.]];
    let trace = trace_geometry(&horizontal, &[[100., 100.]]);
    assert_eq!(trace.vias.len(), 1);
    let pair = &trace.vias[0];
    assert_eq!(pair.before, [88., 100.]);
    assert_eq!(pair.after, [112., 100.]);
    assert_eq!(pair.radius, 2.5);
    assert_eq!(
        trace.fragments,
        vec![
            vec![[0., 100.], [85.5, 100.]],
            vec![[114.5, 100.], [200., 100.]]
        ]
    );
    assert_eq!(pair.backside, [[90.5, 100.], [109.5, 100.]]);
    let vertical = [[100., 0.], [100., 200.]];
    let peer = trace_geometry(&vertical, &[]);
    assert_eq!(peer.fragments, vec![vertical.to_vec()]);
    assert!(peer.vias.is_empty());
    let arrow = flow_arrow(&trace.fragments);
    assert_eq!(arrow.len(), 3);
    assert!(arrow.iter().all(|p| p[0] > 114.5));
    let bend = trace_geometry(&[[0., 0.], [100., 0.], [100., 100.]], &[[100., 0.]]);
    assert_eq!(bend.vias[0].before, [88., 0.]);
    assert_eq!(bend.vias[0].after, [100., 12.]);
    assert_eq!(
        bend.vias[0].backside,
        vec![[90.5, 0.], [100., 0.], [100., 9.5]]
    );
    let diagonal = trace_geometry(&[[0., 0.], [200., 200.]], &[[100., 100.]]);
    assert_eq!(diagonal.vias.len(), 1);
    for p in [diagonal.vias[0].before, diagonal.vias[0].after] {
        assert!((p[0] - p[1]).abs() < 1e-6);
    }
}
