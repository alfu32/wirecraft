//! Measures the whole visible projection and index, not GPU frame time.
use wirecraft_core::{
    Document, Node, NodeKind,
    drawing::Drawing,
    viewport::{Rect, SceneIndex},
};
fn main() {
    let mut document = Document::new("Projection benchmark");
    for i in 0..100_000 {
        let id = format!("node-{i:06}");
        let mut node = Node::new(
            id.clone(),
            format!("Processor {i}"),
            NodeKind::Processor,
            Some("root".into()),
        );
        node.layout.x = (i % 1000) as f64 * 250.;
        node.layout.y = (i / 1000) as f64 * 100.;
        document
            .nodes
            .get_mut("root")
            .unwrap()
            .children
            .push(id.clone());
        document.nodes.insert(id, node);
    }
    let start = std::time::Instant::now();
    let drawing = Drawing::new(&document, "root");
    let projection = start.elapsed();
    let start = std::time::Instant::now();
    let mut index = SceneIndex::default();
    for (id, shape) in &drawing.shapes {
        index.insert(id.clone(), shape.rect);
    }
    let indexing = start.elapsed();
    let start = std::time::Instant::now();
    let mut visible = 0;
    for i in 0..1000 {
        visible += index
            .query(Rect {
                x: i as f64 * 10.,
                y: 500.,
                width: 1400.,
                height: 900.,
            })
            .len();
    }
    println!(
        "100000 graph nodes: projection={projection:?}; index={indexing:?}; 1000 viewport queries={:?}; mean visible={}; annotations={}",
        start.elapsed(),
        visible / 1000,
        drawing.annotations.len()
    );
}
