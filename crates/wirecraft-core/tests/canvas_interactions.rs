use std::collections::BTreeSet;
use wirecraft_core::viewport::{Rect, SelectionOperation, Viewport};
fn ids(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}
#[test]
fn selection_add_remove_replace_are_idempotent_and_preserve_unhit_items() {
    let base = ids(&["a", "b"]);
    let hits = ids(&["b", "c"]);
    assert_eq!(
        SelectionOperation::Add.apply(&base, hits.clone()),
        ids(&["a", "b", "c"])
    );
    assert_eq!(
        SelectionOperation::Remove.apply(&base, hits.clone()),
        ids(&["a"])
    );
    assert_eq!(
        SelectionOperation::Replace.apply(&base, hits),
        ids(&["b", "c"])
    );
    assert_eq!(SelectionOperation::Add.apply(&base, ids(&["a"])), base);
    assert!(
        SelectionOperation::Remove
            .apply(&base, base.clone())
            .is_empty()
    );
    assert_eq!(
        SelectionOperation::Remove.apply(&base, BTreeSet::new()),
        base
    );
}
#[test]
fn window_containment_and_crossing_include_boundary_without_bounding_box_false_hits() {
    let window = Rect::from_points([10., 10.], [30., 30.]);
    assert!(window.encloses(Rect::from_points([10., 10.], [20., 20.])));
    let partial = Rect::from_points([25., 25.], [40., 40.]);
    assert!(window.intersects(partial));
    assert!(!window.encloses(partial));
    assert!(window.intersects_segment([0., 20.], [40., 20.]));
    assert!(window.intersects_segment([10., 0.], [10., 40.]));
    assert!(window.intersects_segment([20., 20.], [20., 20.]));
    assert!(!window.intersects_segment([0., 0.], [0., 0.]));
    assert!(!window.intersects_segment([0., 15.], [15., 0.]));
    assert_eq!(window.union(partial).width, 30.);
}
#[test]
fn wheel_zoom_preserves_world_point_at_cursor_including_clamps() {
    let mut view = Viewport {
        x: -205.,
        y: 140.,
        zoom: 0.7,
    };
    let cursor = [315., 234.];
    let world = view.world(cursor);
    for factor in [1.4, 0.8, 1000., 0.00001] {
        view.zoom_at(cursor, factor);
        for (actual, expected) in view.screen(world).into_iter().zip(cursor) {
            assert!((actual - expected).abs() < 1e-9);
        }
        assert!((0.1..=4.).contains(&view.zoom));
    }
}
