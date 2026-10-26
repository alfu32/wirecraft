use wirecraft_core::viewport::{Rect, SceneIndex};
fn main() {
    let start = std::time::Instant::now();
    let mut index = SceneIndex::default();
    for i in 0..100_000 {
        index.insert(
            i.to_string(),
            Rect {
                x: (i % 1000) as f64 * 250.,
                y: (i / 1000) as f64 * 100.,
                width: 200.,
                height: 70.,
            },
        );
    }
    let built = start.elapsed();
    let start = std::time::Instant::now();
    let mut total = 0;
    for i in 0..1000 {
        total += index
            .query(Rect {
                x: i as f64 * 10.,
                y: 500.,
                width: 1400.,
                height: 900.,
            })
            .len();
    }
    println!(
        "100000 primitives: build={built:?}; 1000 viewport queries={:?}; mean visible={}",
        start.elapsed(),
        total / 1000
    );
}
