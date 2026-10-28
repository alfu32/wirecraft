use serde_json::json;
use std::{path::Path, time::Duration};
use wirecraft_core::{
    Document, Link, NodeKind,
    compiler::{self, CompileOptions},
};
use wirecraft_runtime::jobs::Jobs;

fn graph(set: &str, layout: &str) -> Document {
    let (language, technology, emit, consume) = match set {
        "go" => (
            "go",
            "go",
            "if threadworkRunner.IsRunning() { push(packet, 42) }",
            "var item int; if pop(packet, &item) { fmt.Println(\"RESULT\", item); threadworkRunner.ShutdownRequest() }",
        ),
        "rust" => (
            "rust",
            "rust",
            "if threadwork_runner.is_running() { push(packet, 42i64); }",
            "if let Some(item) = pop::<i64>(packet) { println!(\"RESULT {item}\"); threadwork_runner.shutdown_request(); }",
        ),
        "nodejs" | "systemjs" => (
            "javascript",
            set,
            "if(threadworkRunner.isRunning()) packet.push(42);",
            "if(packet.length) { console.log('RESULT',packet.shift()); threadworkRunner.shutdownRequest(); }",
        ),
        "kotlin" => (
            "kotlin",
            "kotlin-jvm",
            "if(threadworkRunner.isRunning()) packet.addLast(42.0)",
            "if(packet.isNotEmpty()) { println(\"RESULT ${packet.removeFirst()}\"); threadworkRunner.shutdownRequest() }",
        ),
        "php" => (
            "php",
            "php",
            "if(threadwork_runner()->isRunning()) $packet[] = 42;",
            "if(count($packet)) { echo 'RESULT ' . array_shift($packet) . PHP_EOL; threadwork_runner()->shutdownRequest(); }",
        ),
        _ => unreachable!(),
    };
    let mut d = Document::new("network");
    d.nodes.get_mut("root").unwrap().kind = NodeKind::Node;
    d.patch("root",&json!({"technology":{"languageId":language,"technologyId":technology},"fileLayoutStrategyId":layout})).unwrap();
    let producer = d.create("root", "producer", NodeKind::Processor).unwrap();
    let consumer = d.create("root", "consumer", NodeKind::Processor).unwrap();
    d.patch(&producer, &json!({"text":{"source":emit}}))
        .unwrap();
    d.patch(&consumer, &json!({"text":{"source":consume}}))
        .unwrap();
    d.create_link(
        "packet",
        Link {
            source_node_id: producer,
            target_node_id: consumer,
            type_name: "number".into(),
            ..Default::default()
        },
    )
    .unwrap();
    d
}
async fn run(jobs: &Jobs, program: &str, args: Vec<String>, cwd: &Path) -> serde_json::Value {
    let job = jobs
        .start(program, &args, cwd, Duration::from_secs(60))
        .await
        .unwrap();
    let done = tokio::time::timeout(Duration::from_secs(65), async {
        loop {
            let state = jobs.get(job["id"].as_str().unwrap()).await.unwrap();
            if state["status"] != "running" {
                break state;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(done["status"], "succeeded", "{program}: {done}");
    done
}
#[tokio::test]
async fn real_go_and_node_generated_dataflows() {
    workflows(&[
        ("go", "single-file"),
        ("nodejs", "single-file"),
        ("nodejs", "direct-file-system-homomorphism"),
    ])
    .await;
}
/// rustc ships with the toolchain that runs these tests (override: WIRECRAFT_TEST_RUSTC).
#[tokio::test]
async fn real_rust_dataflows_libraries_and_boundary_channels() {
    workflows(&[("rust", "single-file")]).await;
    library_workflows(&[("rust", "single-file")]).await;
    run_cases(&[("rust", "single-file")], nested_graph).await;
}
/// Rust library instances forward generics, `where` clauses, elided lifetimes, patterns
/// and `unsafe`; functions inside `impl` blocks and repeated file-scope imports are left out.
fn rust_signature_graph(set: &str, layout: &str) -> Document {
    let mut d = library_graph(set, layout);
    let find = |d: &Document, name: &str| {
        d.nodes
            .values()
            .find(|n| n.name == name)
            .unwrap()
            .id
            .clone()
    };
    let (lib, consumer) = (find(&d, "lib_math"), find(&d, "consumer"));
    let library = r#"use std::fmt::Display;
use std::fmt::Display;

fn maximum(left: i64, right: i64) -> i64 {
    if left > right { left } else { right }
}
pub fn label<T: Display>(value: T) -> String {
    format!("<{value}>")
}
fn first_word(text: &str) -> &str {
    text.split(' ').next().unwrap_or("")
}
fn sum_pair((a, b): (i64, i64), mut scale: i64, _: bool) -> i64 {
    scale += 0;
    (a + b) * scale
}
fn apply<F>(f: F, x: i64) -> i64
where
    F: Fn(i64) -> i64,
{
    f(x)
}
unsafe fn raw(x: i64) -> i64 {
    x
}
fn table() -> std::collections::HashMap<String, Vec<(i64, i64)>> {
    Default::default()
}
struct Scale;
impl Scale {
    fn inner(&self) -> i64 {
        1
    }
}"#;
    d.patch(&lib, &json!({"text":{"source":library}})).unwrap();
    let consume = "if let Some(item) = pop::<i64>(packet) {\n    let value = math.apply(|x| x + 1, math.sum_pair((math.maximum(item, 42), 0), 1, true) - 1);\n    let raw = unsafe { math.raw(value) };\n    println!(\"RESULT {raw} {} {} {}\", math.label(1), math.first_word(\"a b\"), math.table().len());\n    threadwork_runner.shutdown_request();\n}";
    d.patch(&consumer, &json!({"text":{"source":consume}}))
        .unwrap();
    d
}
/// Link and field names that are Rust keywords become raw identifiers, and every modeled
/// field type maps to a Rust type the derived traits accept.
#[test]
fn rust_keywords_and_field_types_compile() {
    let mut d = graph("rust", "single-file");
    let packet = d
        .nodes
        .values()
        .find(|n| n.name == "packet")
        .unwrap()
        .id
        .clone();
    d.patch(&packet, &json!({"name":"type"})).unwrap();
    for name in ["producer", "consumer"] {
        let id = d
            .nodes
            .values()
            .find(|n| n.name == name)
            .unwrap()
            .id
            .clone();
        let source = d.nodes[&id].text.source.replace("packet", "r#type");
        d.patch(&id, &json!({"text":{"source":source}})).unwrap();
    }
    let inner = d.create("root", "Inner", NodeKind::Type).unwrap();
    let record = d.create("root", "Record", NodeKind::Type).unwrap();
    let fields = [
        "string", "char *", "number", "double", "float", "int", "date", "boolean", "void *",
    ]
    .iter()
    .enumerate()
    .map(|(i, ty)| json!({"name":format!("f{i}"),"typeId":ty}))
    .chain([
        json!({"name":"type","typeId":inner}),
        json!({"name":"self","typeId":record,"isReference":true}),
    ])
    .collect::<Vec<_>>();
    d.patch(
        &record,
        &json!({"typeDefinition":{"fields":fields,"qualifier":"object","genericTypeIds":[]}}),
    )
    .unwrap();
    let project = compiler::generate(&d, "root", CompileOptions::default()).unwrap();
    let main = &project.files[0].content;
    assert!(main.contains(", r#type: &ThreadworkBuffer)"), "{main}");
    assert!(
        main.contains("static type1_a_port: ThreadworkBuffer"),
        "{main}"
    );
    assert!(main.contains("pub r#type: Inner,"), "{main}");
    assert!(main.contains("pub self_: Option<Box<Record>>,"), "{main}");
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("main.rs");
    std::fs::write(&source, main).unwrap();
    let output = std::process::Command::new(
        std::env::var("WIRECRAFT_TEST_RUSTC").unwrap_or_else(|_| "rustc".into()),
    )
    .args([
        "--edition",
        "2021",
        "-D",
        "warnings",
        "--emit=metadata",
        "-o",
    ])
    .arg(temp.path().join("main.rmeta"))
    .arg(&source)
    .output()
    .unwrap();
    assert!(
        output.status.success(),
        "{}\n{main}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[tokio::test]
async fn real_rust_library_signatures_forward_through_the_instance() {
    let project = compiler::generate(
        &rust_signature_graph("rust", "single-file"),
        "root",
        CompileOptions::default(),
    )
    .unwrap();
    let main = &project.files[0].content;
    assert!(
        main.contains("// use std::fmt::Display; (duplicate)"),
        "{main}"
    );
    assert!(!main.contains("fn inner(self"), "{main}");
    run_cases(&[("rust", "single-file")], rust_signature_graph).await;
}
#[tokio::test]
#[ignore = "requires Kotlin 2.2+ and Java 21; kotlinc on PATH or WIRECRAFT_TEST_KOTLINC"]
async fn real_kotlin_generated_dataflows() {
    workflows(&[("kotlin", "single-file"), ("kotlin", "source-set")]).await;
}
#[tokio::test]
#[ignore = "requires external PHP and Bun; set WIRECRAFT_TEST_PHP and WIRECRAFT_TEST_BUN"]
async fn real_php_and_systemjs_generated_dataflows() {
    workflows(&[
        ("php", "single-file"),
        ("php", "direct-file-system-homomorphism"),
        ("systemjs", "single-file"),
    ])
    .await;
}
/// Producer and consumer reach one library through differently named dependency links.
fn library_graph(set: &str, layout: &str) -> Document {
    let (library, emit, consume) = match set {
        "go" => (
            "func maximum(left int, right int) int {\n\tif left > right {\n\t\treturn left\n\t}\n\treturn right\n}",
            "if threadworkRunner.IsRunning() { push(packet, numbers.maximum(39, 40)) }",
            "var item int; if pop(packet, &item) { fmt.Println(\"RESULT\", math.maximum(item, 42)); threadworkRunner.ShutdownRequest() }",
        ),
        "rust" => (
            "fn maximum(left: i64, right: i64) -> i64 {\n    if left > right { left } else { right }\n}",
            "if threadwork_runner.is_running() { push(packet, numbers.maximum(39, 40)); }",
            "if let Some(item) = pop::<i64>(packet) { println!(\"RESULT {}\", math.maximum(item, 42)); threadwork_runner.shutdown_request(); }",
        ),
        "nodejs" | "systemjs" => (
            "function maximum(left, right) {\n  return left > right ? left : right;\n}",
            "if(threadworkRunner.isRunning()) packet.push(numbers.maximum(39, 40));",
            "if(packet.length) { console.log('RESULT', math.maximum(packet.shift(), 42)); threadworkRunner.shutdownRequest(); }",
        ),
        "kotlin" => (
            "fun maximum(left: Double, right: Double): Double = if (left > right) left else right",
            "if(threadworkRunner.isRunning()) packet.addLast(numbers.maximum(39.0, 40.0))",
            "if(packet.isNotEmpty()) { println(\"RESULT ${math.maximum(packet.removeFirst(), 42.0)}\"); threadworkRunner.shutdownRequest() }",
        ),
        "php" => (
            "function maximum($left, $right)\n{\n    return max($left, $right);\n}",
            "if(threadwork_runner()->isRunning()) $packet[] = $numbers->maximum(39, 40);",
            "if(count($packet)) { echo 'RESULT ' . $math->maximum(array_shift($packet), 42) . PHP_EOL; threadwork_runner()->shutdownRequest(); }",
        ),
        _ => unreachable!(),
    };
    let mut d = graph(set, layout);
    let id = |d: &Document, name: &str| {
        d.nodes
            .values()
            .find(|n| n.name == name)
            .unwrap()
            .id
            .clone()
    };
    let (producer, consumer) = (id(&d, "producer"), id(&d, "consumer"));
    d.patch(&producer, &json!({"text":{"source":emit}}))
        .unwrap();
    d.patch(&consumer, &json!({"text":{"source":consume}}))
        .unwrap();
    let lib = d.create("root", "lib_math", NodeKind::Processor).unwrap();
    d.patch(&lib, &json!({"text":{"source":library}})).unwrap();
    for (name, target) in [("numbers", &producer), ("math", &consumer)] {
        let link = d
            .create_link(
                name,
                Link {
                    source_node_id: lib.clone(),
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
    d
}
#[test]
fn library_links_lend_one_shared_instance_named_by_each_link() {
    for (set, layout) in [
        ("go", "single-file"),
        ("nodejs", "single-file"),
        ("nodejs", "direct-file-system-homomorphism"),
        ("kotlin", "single-file"),
        ("kotlin", "source-set"),
        ("php", "single-file"),
        ("php", "direct-file-system-homomorphism"),
        ("systemjs", "single-file"),
        ("rust", "single-file"),
    ] {
        let project = compiler::generate(
            &library_graph(set, layout),
            "root",
            CompileOptions::default(),
        )
        .unwrap();
        let all = project
            .files
            .iter()
            .map(|f| f.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let declaration = match set {
            "go" => "var lib_math_library = &Lib_mathLibrary{",
            "rust" => "static lib_math_library: Lib_mathLibrary = Lib_mathLibrary;",
            "kotlin" => "val lib_math_library = Lib_mathLibrary",
            "php" => "$lib_math_library = new class {",
            _ => "const lib_math_library = Object.freeze({ maximum });",
        };
        assert_eq!(
            all.matches(declaration).count(),
            1,
            "{set}/{layout}:\n{all}"
        );
        // No per-link library values or prefixed aliases remain.
        for stale in ["numbers1", "math1", "numbers__", "math__"] {
            assert!(!all.contains(stale), "{set}/{layout} still has {stale}");
        }
        let parameter = match set {
            "go" => "math *Lib_mathLibrary",
            "rust" => "math: &Lib_mathLibrary",
            "kotlin" => "math: Lib_mathLibrary",
            "php" => "mixed $math",
            _ => ", math)",
        };
        assert!(
            all.contains(parameter),
            "{set}/{layout} lacks {parameter}:\n{all}"
        );
    }
}
#[tokio::test]
async fn real_go_and_node_library_instances() {
    library_workflows(&[
        ("go", "single-file"),
        ("nodejs", "single-file"),
        ("nodejs", "direct-file-system-homomorphism"),
    ])
    .await;
}
#[tokio::test]
#[ignore = "requires Kotlin 2.2+ and Java 21; kotlinc on PATH or WIRECRAFT_TEST_KOTLINC"]
async fn real_kotlin_library_instances() {
    library_workflows(&[("kotlin", "single-file"), ("kotlin", "source-set")]).await;
}
#[tokio::test]
#[ignore = "requires external PHP and Bun; set WIRECRAFT_TEST_PHP and WIRECRAFT_TEST_BUN"]
async fn real_php_and_systemjs_library_instances() {
    library_workflows(&[
        ("php", "single-file"),
        ("php", "direct-file-system-homomorphism"),
        ("systemjs", "single-file"),
    ])
    .await;
}
/// The producer stays at the root while the consumer moves into a nested composite, so
/// the data link crosses the composite boundary and the root owns and transports it.
fn nested_graph(set: &str, layout: &str) -> Document {
    let mut d = graph(set, layout);
    let stage = d.create("root", "stage", NodeKind::Processor).unwrap();
    let consumer = d
        .nodes
        .values()
        .find(|n| n.name == "consumer")
        .unwrap()
        .id
        .clone();
    d.move_node(&consumer, Some(&stage)).unwrap();
    d
}
#[tokio::test]
async fn real_go_and_node_composite_boundary_channels() {
    run_cases(
        &[("go", "single-file"), ("nodejs", "single-file")],
        nested_graph,
    )
    .await;
    // A file cannot see another file's buffers: file-based layouts refuse explicitly.
    let error = compiler::generate(
        &nested_graph("nodejs", "direct-file-system-homomorphism"),
        "root",
        CompileOptions::default(),
    )
    .err()
    .unwrap();
    assert!(
        error.message.contains("single-file layout"),
        "{}",
        error.message
    );
}
#[tokio::test]
#[ignore = "requires Kotlin 2.2+ and Java 21; kotlinc on PATH or WIRECRAFT_TEST_KOTLINC"]
async fn real_kotlin_composite_boundary_channels() {
    run_cases(&[("kotlin", "single-file")], nested_graph).await;
}
#[tokio::test]
#[ignore = "requires external PHP and Bun; set WIRECRAFT_TEST_PHP and WIRECRAFT_TEST_BUN"]
async fn real_php_and_systemjs_composite_boundary_channels() {
    run_cases(
        &[("php", "single-file"), ("systemjs", "single-file")],
        nested_graph,
    )
    .await;
}
async fn library_workflows(cases: &[(&str, &str)]) {
    run_cases(cases, library_graph).await;
}
async fn workflows(cases: &[(&str, &str)]) {
    run_cases(cases, graph).await;
}
async fn run_cases(cases: &[(&str, &str)], build: fn(&str, &str) -> Document) {
    let jobs = Jobs::new(tokio::sync::broadcast::channel(64).0);
    for &(set, layout) in cases {
        let d = build(set, layout);
        let project = compiler::generate(&d, "root", CompileOptions::default()).unwrap();

        let temp = tempfile::tempdir().unwrap();
        let out = temp.path().join("generated");
        wirecraft_runtime::generation::write(&project, &out).unwrap();
        let files = project
            .files
            .iter()
            .filter(|f| {
                f.path.ends_with(if set == "php" {
                    ".php"
                } else if set == "systemjs" {
                    ".js"
                } else if set == "go" {
                    ".go"
                } else if set == "rust" {
                    ".rs"
                } else if set == "nodejs" {
                    ".js"
                } else {
                    ".kt"
                })
            })
            .map(|f| out.join(&f.path).to_string_lossy().to_string())
            .collect::<Vec<_>>();
        let done = match set {
            "go" => {
                let executable = temp.path().join("network").to_string_lossy().to_string();
                run(
                    &jobs,
                    "go",
                    vec![
                        "build".into(),
                        "-o".into(),
                        executable.clone(),
                        files[0].clone(),
                    ],
                    &out,
                )
                .await;
                run(&jobs, &executable, vec![], &out).await
            }
            "rust" => {
                let executable = temp.path().join("network").to_string_lossy().to_string();
                run(
                    &jobs,
                    &std::env::var("WIRECRAFT_TEST_RUSTC").unwrap_or_else(|_| "rustc".into()),
                    vec![
                        "--edition".into(),
                        "2021".into(),
                        "-o".into(),
                        executable.clone(),
                        files[0].clone(),
                    ],
                    &out,
                )
                .await;
                run(&jobs, &executable, vec![], &out).await
            }
            "php" => {
                run(
                    &jobs,
                    &std::env::var("WIRECRAFT_TEST_PHP").unwrap_or_else(|_| "php".into()),
                    vec![files[0].clone()],
                    &out,
                )
                .await
            }
            // systemJS scripts are engine-neutral; the tests run them on Bun.
            "systemjs" => {
                run(
                    &jobs,
                    &std::env::var("WIRECRAFT_TEST_BUN").unwrap_or_else(|_| "bun".into()),
                    vec!["run".into(), files[0].clone()],
                    &out,
                )
                .await
            }
            "nodejs" => run(&jobs, "node", vec![files[0].clone()], &out).await,
            "kotlin" => {
                let jar = temp
                    .path()
                    .join("network.jar")
                    .to_string_lossy()
                    .to_string();
                let mut args = files;
                args.extend(["-include-runtime".into(), "-d".into(), jar.clone()]);
                run(
                    &jobs,
                    &std::env::var("WIRECRAFT_TEST_KOTLINC").unwrap_or_else(|_| "kotlinc".into()),
                    args,
                    &out,
                )
                .await;
                run(&jobs, "java", vec!["-jar".into(), jar], &out).await
            }
            _ => unreachable!(),
        };
        assert!(
            done["stdout"].as_str().unwrap().contains("RESULT 42"),
            "{set}/{layout}: {done}"
        );
        assert!(
            project
                .files
                .iter()
                .any(|f| f.source_map.iter().any(|m| m.section == "source")),
            "Missing authored source maps: {set}"
        );
    }
}
#[test]
fn php_systemjs_generate_and_bundle_roundtrip_with_graph_overrides() {
    for set in ["php", "systemjs"] {
        let project = compiler::generate(
            &graph(set, "single-file"),
            "root",
            CompileOptions::default(),
        )
        .unwrap();
        assert!(project.files[0].content.contains("RESULT"));
    }
    let mut d = graph("nodejs", "single-file");
    let id = d
        .create("root", "@ProcessorDeclaration", NodeKind::Processor)
        .unwrap();
    d.patch(&id, &json!({"text":{"source":"// custom {{node.name}}"}}))
        .unwrap();
    let mut bundle = wirecraft_core::templates::TemplateSet::builtin("nodejs").unwrap();
    bundle.overlay(&d).unwrap();
    let bundle = serde_json::from_str(&serde_json::to_string(&bundle).unwrap()).unwrap();
    let project = compiler::generate(
        &d,
        "root",
        CompileOptions {
            bundle: Some(bundle),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(project.files[0].content.contains("// custom consumer"));
}

#[tokio::test]
async fn http_mcp_template_catalog_compile_export_and_reuse_share_services() {
    use wirecraft_runtime::{
        backend::Application,
        service::{Request, Service},
    };
    let app = Application::new(
        Service::new(graph("nodejs", "single-file"), None),
        "compiler-test-token".into(),
        None,
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/mcp", listener.local_addr().unwrap());
    let server_app = app.clone();
    let server = tokio::spawn(async move { server_app.serve(listener).await.unwrap() });
    let client = reqwest::Client::new();
    let temp = tempfile::tempdir().unwrap();
    for (operation, args) in [
        ("compiler_templates", json!({})),
        (
            "compile_project",
            json!({"outputDir":temp.path().join("generated")}),
        ),
        (
            "export_compiler",
            json!({"outputDir":temp.path().join("bundle")}),
        ),
    ] {
        let response:serde_json::Value=client.post(&endpoint).bearer_auth("compiler-test-token").json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"wirecraft.workbench","arguments":{"operation":operation,"arguments":args}}})).send().await.unwrap().json().await.unwrap();
        assert_eq!(response["result"]["isError"], false, "{response}");
        if operation == "compiler_templates" {
            assert_eq!(
                response["result"]["structuredContent"]["templateSets"]
                    .as_array()
                    .unwrap()
                    .len(),
                7
            );
        }
    }
    let bundle: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(temp.path().join("bundle/compiler.wirecompiler.json")).unwrap(),
    )
    .unwrap();
    let reused = app
        .execute(Request::new(
            "compile.project",
            json!({"bundle":bundle,"outputDir":temp.path().join("reused")}),
        ))
        .await
        .unwrap();
    assert_eq!(reused["capability"], "template-set:nodejs");
    assert!(
        app.execute(Request::new(
            "compile.project",
            json!({"outputDir":temp.path().join("reused")})
        ))
        .await
        .is_err()
    );
    let tools: serde_json::Value = client
        .post(&endpoint)
        .bearer_auth("compiler-test-token")
        .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let tool = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "wirecraft.workbench")
        .unwrap();
    assert!(
        tool["inputSchema"]["properties"]["operation"]["enum"]
            .as_array()
            .unwrap()
            .contains(&json!("compile_project"))
    );
    app.shutdown().await;
    server.await.unwrap();
}

#[test]
fn template_failures_paths_static_companions_and_scope_are_explicit() {
    use wirecraft_core::templates::{ProjectTemplate, TemplateSet};
    let mut d = graph("nodejs", "single-file");
    let static_id = d
        .create("root", "@StaticFile", NodeKind::Processor)
        .unwrap();
    d.patch(&static_id,&json!({"metadata":{"path":"assets/config.json"},"text":{"source":"{\"literal\":\"{{do_not_render}}\"}"}})).unwrap();
    let project = compiler::generate(&d, "root", CompileOptions::default()).unwrap();
    assert!(
        project
            .files
            .iter()
            .any(|f| f.path == "assets/config.json" && f.content.contains("{{do_not_render}}"))
    );
    let producer = d.nodes.values().find(|n| n.name == "producer").unwrap();
    assert_eq!(
        compiler::generate(&d, &producer.id, CompileOptions::default())
            .err()
            .unwrap()
            .code,
        "unsupported"
    );
    for path in ["../escape", "/absolute", "network.js"] {
        let mut bundle = TemplateSet::builtin("nodejs").unwrap();
        bundle.project_files.push(ProjectTemplate {
            path: path.into(),
            content: "x".into(),
            layouts: vec![],
        });
        assert!(
            compiler::generate(
                &d,
                "root",
                CompileOptions {
                    bundle: Some(bundle),
                    ..Default::default()
                }
            )
            .is_err(),
            "{path}"
        );
    }
    let mut bundle = TemplateSet::builtin("nodejs").unwrap();
    bundle
        .roles
        .insert("processor.declaration".into(), "{% if broken %}".into());
    assert_eq!(
        compiler::generate(
            &d,
            "root",
            CompileOptions {
                bundle: Some(bundle),
                ..Default::default()
            }
        )
        .err()
        .unwrap()
        .code,
        "template_error"
    );
}

#[tokio::test]
async fn cli_custom_compiler_bundle_export_and_roundtrip() {
    use wirecraft_core::store;
    let mut d = Document::new("custom");
    d.patch("root",&json!({"technology":{"technologyId":"compiler-template-set","fileExtension":"txt"},"fileLayoutStrategyId":"single-file"})).unwrap();
    let body = d.create("root", "worker", NodeKind::Processor).unwrap();
    d.patch(&body, &json!({"text":{"source":"authored"}}))
        .unwrap();
    for (name, source) in [
        ("@ProcessorDeclaration", "body={{ declaration }}"),
        ("@CompositeSingleFile", "{{ childDeclarations }}"),
        ("@ProjectFile", "project={{projectName}}"),
    ] {
        let id = d.create("root", name, NodeKind::Processor).unwrap();
        d.patch(
            &id,
            &json!({"text":{"source":source},"metadata":{"path":"README.txt"}}),
        )
        .unwrap();
    }
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("custom.orch");
    store::save(&d, &source).unwrap();
    let jobs = Jobs::new(tokio::sync::broadcast::channel(64).0);
    let binary = env!("CARGO_BIN_EXE_wirecraft");
    run(
        &jobs,
        binary,
        vec![
            "export-compiler".into(),
            source.to_string_lossy().into(),
            temp.path().join("bundle").to_string_lossy().into(),
        ],
        temp.path(),
    )
    .await;
    run(
        &jobs,
        binary,
        vec![
            "compile".into(),
            source.to_string_lossy().into(),
            temp.path().join("generated").to_string_lossy().into(),
            "--compiler".into(),
            temp.path()
                .join("bundle/compiler.wirecompiler.json")
                .to_string_lossy()
                .into(),
        ],
        temp.path(),
    )
    .await;
    let product = std::fs::read_to_string(temp.path().join("generated/custom.txt")).unwrap();
    assert!(product.contains("authored"));
    assert!(!product.contains("{{ declaration }}"));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("generated/README.txt")).unwrap(),
        "project=custom"
    );
    assert_eq!(store::load(&source).unwrap().nodes.len(), d.nodes.len());
}
