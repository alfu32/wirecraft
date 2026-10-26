//! Bounding-rectangle false positives for routes surrounding an empty viewport.
use wirecraft_core::viewport::{Rect, RouteIndex, SceneIndex};
fn main() {
    let mut index = SceneIndex::default();
    for i in 0..10_000 {
        index.insert(
            format!("wire-{i}"),
            Rect {
                x: 0.,
                y: 0.,
                width: 1_000_000.,
                height: 1_000_000.,
            },
        );
    }
    let view = Rect {
        x: 499_000.,
        y: 499_000.,
        width: 2000.,
        height: 2000.,
    };
    let start = std::time::Instant::now();
    let mut count = 0;
    for _ in 0..100 {
        count += index.query(view).len();
    }
    println!(
        "100 empty-interior route queries: {:?}; mean bounding-box candidates={} (actual segments visible=0)",
        start.elapsed(),
        count / 100
    );
    let names = (0..10_000).map(|i| format!("wire-{i}")).collect::<Vec<_>>();
    let points = [[0., 0.], [1_000_000., 0.], [1_000_000., 1_000_000.]];
    let start = std::time::Instant::now();
    let segments = RouteIndex::new(names.iter().map(|id| (id.as_str(), points.as_slice())));
    let built = start.elapsed();
    let start = std::time::Instant::now();
    let mut count = 0;
    for _ in 0..100 {
        count += segments.query(view).len();
    }
    println!(
        "Segment BVH: build={built:?}; 100 empty-interior queries={:?}; mean routes={}",
        start.elapsed(),
        count / 100
    );
}
