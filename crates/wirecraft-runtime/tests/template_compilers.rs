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
        "nodejs" | "quickjs" => (
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
#[tokio::test]
#[ignore = "requires Kotlin 2.2.0 and Java 21; set WIRECRAFT_TEST_KOTLINC"]
async fn real_kotlin_generated_dataflows() {
    workflows(&[("kotlin", "single-file"), ("kotlin", "source-set")]).await;
}
#[tokio::test]
#[ignore = "requires external PHP and QuickJS; set WIRECRAFT_TEST_PHP and WIRECRAFT_TEST_QJS"]
async fn real_php_and_quickjs_generated_dataflows() {
    workflows(&[
        ("php", "single-file"),
        ("php", "direct-file-system-homomorphism"),
        ("quickjs", "single-file"),
    ])
    .await;
}
async fn workflows(cases: &[(&str, &str)]) {
    let jobs = Jobs::new(tokio::sync::broadcast::channel(64).0);
    for &(set, layout) in cases {
        let d = graph(set, layout);
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
                } else if set == "quickjs" {
                    ".js"
                } else if set == "go" {
                    ".go"
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
            "php" => {
                run(
                    &jobs,
                    &std::env::var("WIRECRAFT_TEST_PHP").unwrap_or_else(|_| "php".into()),
                    vec![files[0].clone()],
                    &out,
                )
                .await
            }
            "quickjs" => {
                run(
                    &jobs,
                    &std::env::var("WIRECRAFT_TEST_QJS").unwrap_or_else(|_| "qjs".into()),
                    vec![files[0].clone()],
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
fn php_quickjs_generate_and_bundle_roundtrip_with_graph_overrides() {
    for set in ["php", "quickjs"] {
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
                6
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
