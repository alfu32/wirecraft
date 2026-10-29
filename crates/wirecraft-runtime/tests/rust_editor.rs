//! Rust body editing: the compiler body context of the Rust template program and its
//! rust-analyzer completion, hover, coloring and diagnostics.
//!
//! The rust-analyzer tests need `rust-analyzer` on PATH (`rustup component add
//! rust-analyzer`) or `WIRECRAFT_RUST_ANALYZER`, and cargo for its `cargo check`.
use serde_json::{Value, json};
use wirecraft_core::{Document, Link, NodeKind, editor::body_context};
use wirecraft_runtime::{
    backend::Application,
    service::{Request, Service},
};

const PRODUCER: &str =
    "if threadwork_runner.is_running() { push(packet, numbers.maximum(39, 40)); }";
const CONSUMER: &str = "if let Some(item) = pop::<i64>(packet) {\n    println!(\"RESULT {}\", math.maximum(item, 42));\n    threadwork_runner.shutdown_request();\n}";

/// A producer and consumer joined by `packet`, both borrowing one library instance.
fn network() -> (Document, String, String, String) {
    let mut d = Document::new("network");
    d.nodes.get_mut("root").unwrap().kind = NodeKind::Node;
    d.patch(
        "root",
        &json!({"technology":{"languageId":"rust","technologyId":"rust"},"fileLayoutStrategyId":"single-file"}),
    )
    .unwrap();
    let producer = d.create("root", "producer", NodeKind::Processor).unwrap();
    let consumer = d.create("root", "consumer", NodeKind::Processor).unwrap();
    d.patch(&producer, &json!({"text":{"source":PRODUCER}}))
        .unwrap();
    d.patch(&consumer, &json!({"text":{"source":CONSUMER}}))
        .unwrap();
    d.create_link(
        "packet",
        Link {
            source_node_id: producer.clone(),
            target_node_id: consumer.clone(),
            type_name: "number".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let library = d.create("root", "lib_math", NodeKind::Processor).unwrap();
    d.patch(
        &library,
        &json!({"text":{"source":"/// The larger of two numbers.\nfn maximum(left: i64, right: i64) -> i64 {\n    if left > right { left } else { right }\n}"}}),
    )
    .unwrap();
    for (name, target) in [("numbers", &producer), ("math", &consumer)] {
        let link = d
            .create_link(
                name,
                Link {
                    source_node_id: library.clone(),
                    source_port_name: "service".into(),
                    target_node_id: target.clone(),
                    target_port_name: name.into(),
                    ..Default::default()
                },
            )
            .unwrap();
        d.patch(&link, &json!({"link":{"interactionKind":"lib"}}))
            .unwrap();
    }
    (d, producer, consumer, library)
}

fn symbol<'a>(symbols: &'a [Value], label: &str) -> &'a Value {
    symbols
        .iter()
        .find(|s| s["label"] == label)
        .unwrap_or_else(|| panic!("no symbol {label}: {symbols:?}"))
}

#[test]
fn rust_bodies_map_into_the_generated_program_with_model_symbols() {
    let (d, producer, consumer, library) = network();
    let context = body_context(&d, &consumer, "source", None).unwrap();
    assert_eq!(context.language, "rust");
    assert!(context.file.path.ends_with(".rs"), "{}", context.file.path);
    assert!(
        context
            .companions
            .iter()
            .any(|f| f.path == "Cargo.toml" && f.content.contains(&context.file.path)),
        "rust-analyzer needs the manifest naming the program"
    );
    // Every authored line sits at the mapped line, shifted by the column offset.
    let lines = context.file.content.lines().collect::<Vec<_>>();
    for (i, authored) in CONSUMER.lines().enumerate() {
        let generated = lines[context.source.generated_start_line - 1 + i];
        assert_eq!(
            &generated[context.source.generated_column_offset..],
            authored
        );
    }
    assert!(
        context.header.starts_with("fn run_consumer_")
            && context.header.contains("packet: &ThreadworkBuffer")
            && context.header.contains("math: &Lib_mathLibrary"),
        "{}",
        context.header
    );
    let input = symbol(&context.symbols, "packet");
    assert!(
        input["documentation"]
            .as_str()
            .unwrap()
            .starts_with("Input buffer of number: pop::<T>(packet) takes the next value"),
        "{input}"
    );
    let math = symbol(&context.symbols, "math");
    assert_eq!(math["detail"], "math: &Lib_mathLibrary");
    assert!(math["documentation"].as_str().unwrap().contains("lib_math"));
    symbol(&context.symbols, "context");

    let output = body_context(&d, &producer, "source", None).unwrap();
    assert!(
        symbol(&output.symbols, "packet")["documentation"]
            .as_str()
            .unwrap()
            .starts_with("Output buffer of number: push(packet, value) sends a value")
    );
    // A draft replaces the stored body; an empty draft still has a position.
    let draft = body_context(&d, &producer, "source", Some("let x = 1;")).unwrap();
    assert_eq!(
        draft
            .file
            .content
            .lines()
            .nth(draft.source.generated_start_line - 1),
        Some("    let x = 1;")
    );
    let empty = body_context(&d, &producer, "initialization", Some("")).unwrap();
    assert!(empty.header.starts_with("fn init_producer_"));
    // Library bodies are file-scope items without a function header.
    let items = body_context(&d, &library, "source", None).unwrap();
    assert_eq!(items.source.generated_column_offset, 0);
    assert!(items.header.starts_with("// "), "{}", items.header);
    // Other sections and technologies are rejected explicitly.
    assert_eq!(
        body_context(&d, &consumer, "specification", None)
            .err()
            .unwrap()
            .code,
        "unsupported"
    );
}

fn items(completion: &Value) -> Vec<&Value> {
    completion["items"].as_array().unwrap().iter().collect()
}
fn has(completion: &Value, name: &str) -> bool {
    items(completion)
        .iter()
        .any(|i| i["filterText"] == name || i["label"] == name)
}

#[tokio::test]
#[ignore = "requires rust-analyzer on PATH or WIRECRAFT_RUST_ANALYZER, and cargo"]
async fn rust_analyzer_completes_hovers_and_colors_rust_bodies() {
    let (d, _, consumer, _) = network();
    let app = Application::new(Service::new(d, None), "fixture".into(), None);
    let ask = |command: &str, text: &str, offset: usize| {
        Request::new(
            command,
            json!({"nodeId":consumer,"section":"source","text":text,"offset":offset,"generation":5}),
        )
    };
    let context = app
        .execute(Request::new(
            "editor.context",
            json!({"nodeId":consumer,"section":"source"}),
        ))
        .await
        .unwrap();
    assert!(
        context["header"]
            .as_str()
            .unwrap()
            .starts_with("fn run_consumer_")
    );

    // Library members through the lent instance, with their signature.
    let text = "let best = math.max";
    let completion = app
        .execute(ask("editor.complete", text, text.len()))
        .await
        .unwrap();
    let maximum = items(&completion)
        .into_iter()
        .find(|i| i["filterText"] == "maximum" || i["label"] == "maximum")
        .unwrap_or_else(|| panic!("{completion}"));
    assert!(
        maximum["detail"].as_str().unwrap().contains("i64"),
        "{maximum}"
    );
    // The replaced range is the typed prefix, in authored-body coordinates.
    assert_eq!(
        maximum["textEdit"]["range"]["start"],
        json!({"line":0,"character":16})
    );
    // Runtime methods and model arguments, the latter with the model's description.
    let text = "threadwork_runner.is_r";
    let completion = app
        .execute(ask("editor.complete", text, text.len()))
        .await
        .unwrap();
    // The server's unfiltered member list is narrowed to the typed prefix.
    let first = &items(&completion)[0];
    assert!(
        first["filterText"] == "is_running" || first["label"] == "is_running",
        "{completion}"
    );
    assert!(!has(&completion, "shutdown_request"), "{completion}");
    let text = "let next = pac";
    let completion = app
        .execute(ask("editor.complete", text, text.len()))
        .await
        .unwrap();
    let packet = items(&completion)
        .into_iter()
        .find(|i| i["label"] == "packet" || i["filterText"] == "packet")
        .unwrap_or_else(|| panic!("{completion}"));
    assert_eq!(packet["detail"], "packet: &ThreadworkBuffer");

    // Hover: a library function's documentation, and the model provenance of `math`.
    let offset = CONSUMER.find("maximum").unwrap() + 2;
    let hover = app
        .execute(ask("editor.hover", CONSUMER, offset))
        .await
        .unwrap();
    let contents = hover["hover"]["contents"].to_string();
    assert!(
        contents.contains("fn maximum(self, left: i64, right: i64) -> i64")
            && contents.contains("The larger of two numbers."),
        "{hover}"
    );
    let offset = CONSUMER.find("math").unwrap() + 1;
    let hover = app
        .execute(ask("editor.hover", CONSUMER, offset))
        .await
        .unwrap();
    assert_eq!(hover["identifier"], "math");
    let contents = hover["hover"]["contents"]["value"].as_str().unwrap();
    assert!(
        contents.starts_with("```rust\nmath: &Lib_mathLibrary") && contents.contains("lib_math"),
        "{contents}"
    );

    // Coloring and native diagnostics, both in authored-body coordinates.
    let broken = format!("{CONSUMER}\nlet wrong: u32 = \"text\";");
    let tokens = app.execute(ask("editor.tokens", &broken, 0)).await.unwrap();
    let spans = tokens["tokens"].as_array().unwrap();
    let start = broken.find("maximum").unwrap();
    assert!(
        spans.iter().any(|t| t["startByte"] == start
            && ["function", "method"].contains(&t["kind"].as_str().unwrap())),
        "{tokens}"
    );
    assert!(
        spans
            .iter()
            .all(|t| t["endByte"].as_u64().unwrap() as usize <= broken.len())
    );
    assert_eq!(tokens["diagnosticsCurrent"], true);
    let mismatch = tokens["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["severity"] == 1)
        .unwrap_or_else(|| panic!("{tokens}"));
    assert_eq!(mismatch["source"]["nodeId"], consumer);
    assert_eq!(mismatch["source"]["line"], 5);
    app.shutdown().await;
}

#[tokio::test]
#[ignore = "requires rust-analyzer on PATH or WIRECRAFT_RUST_ANALYZER, and cargo"]
async fn rust_analyzer_cargo_check_maps_graph_errors_to_bodies() {
    let (d, _, consumer, _) = network();
    let app = Application::new(Service::new(d, None), "fixture".into(), None);
    let analyze = || Request::new("editor.analyze_generated", json!({}));
    let clean = app.execute(analyze()).await.unwrap();
    assert_eq!(clean["checker"], "cargo check", "{clean}");
    assert!(
        clean["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["severity"] != 1),
        "{clean}"
    );
    // A borrow error is found by rustc only, not by rust-analyzer's own analysis.
    let broken = format!(
        "{CONSUMER}\nlet text = String::new();\nlet moved = text;\nprintln!(\"{{text}}\");"
    );
    app.execute(Request::new(
        "threadwork.update_entity",
        json!({"nodeId":consumer,"patch":{"text":{"source":broken}}}),
    ))
    .await
    .unwrap();
    let analysis = app.execute(analyze()).await.unwrap();
    let moved = analysis["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["severity"] == 1)
        .unwrap_or_else(|| panic!("{analysis}"));
    assert_eq!(moved["code"], "E0382", "{moved}");
    assert_eq!(moved["source"]["nodeId"], consumer);
    assert_eq!(moved["source"]["line"], 7);
    // The column is the authored one (`text` in `"{text}"`), without the generated indentation.
    assert_eq!(moved["source"]["column"], 12);
    app.shutdown().await;
}
