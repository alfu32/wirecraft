use serde_json::json;
use std::time::Duration;
use wirecraft_core::{Document, Link, NodeKind, c17::C17, generation::GeneratedProject};
use wirecraft_runtime::{
    backend::Application,
    service::{Request, Service},
};
fn project() -> Document {
    let mut d = Document::new("C network");
    d.patch("root",&json!({"technology":{"languageId":"c","technologyId":"c-native"},"fileLayoutStrategyId":"single-file"})).unwrap();
    d
}
fn processor(d: &mut Document, name: &str, source: &str) -> String {
    let id = d.create("root", name, NodeKind::Processor).unwrap();
    d.patch(&id, &json!({"text":{"source":source}})).unwrap();
    id
}
async fn build_run(d: Document) -> (GeneratedProject, String) {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("generated");
    let app = Application::new(Service::new(d, None), "fixture".into(), None);
    let generated = app
        .execute(Request::new("compile.c17", json!({"outputDir":output})))
        .await
        .unwrap();
    let project: GeneratedProject = serde_json::from_value(generated).unwrap();
    let exe = temp.path().join("network");
    let job = app
        .jobs
        .start(
            "clang",
            &[
                "-std=c17".into(),
                "-Wall".into(),
                "-Wextra".into(),
                "-Werror".into(),
                "-Wno-unused-function".into(),
                output.join(&project.files[0].path).to_string_lossy().into(),
                "-o".into(),
                exe.to_string_lossy().into(),
            ],
            temp.path(),
            Duration::from_secs(10),
        )
        .await
        .unwrap();
    let wait = |id: String, app: Application| async move {
        tokio::time::timeout(Duration::from_secs(12), async {
            loop {
                let status = app.jobs.get(&id).await.unwrap();
                if status["status"] != "running" {
                    break status;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap()
    };
    let done = wait(job["id"].as_str().unwrap().into(), app.clone()).await;
    assert_eq!(done["status"], "succeeded", "{done}");
    let job = app
        .jobs
        .start(
            exe.to_str().unwrap(),
            &[],
            temp.path(),
            Duration::from_secs(3),
        )
        .await
        .unwrap();
    let done = wait(job["id"].as_str().unwrap().into(), app.clone()).await;
    assert_eq!(done["status"], "succeeded", "{done}");
    app.shutdown().await;
    (project, done["stdout"].as_str().unwrap().into())
}
#[tokio::test]
async fn modeled_producer_typed_link_consumer_shutdown_and_multi_packet_transport() {
    let mut d = project();
    let packet = d.create("root", "Packet", NodeKind::Type).unwrap();
    d.patch(&packet,&json!({"typeDefinition":{"fields":[{"name":"value","typeId":"int"}],"qualifier":"object","genericTypeIds":[]}})).unwrap();
    let producer = processor(
        &mut d,
        "producer",
        r#"static int emitted=0;
if (!emitted) {
    for (int i=1; i<=3; i++) { Packet item={i}; if(push(packet,&item)!=THREADWORK_OK) return THREADWORK_ERROR; }
    emitted=1;
} else { threadwork_runner__shutdown_request(&threadwork_runner); }"#,
    );
    let consumer = processor(
        &mut d,
        "consumer",
        r#"while(threadwork_buffer_count(packet)>0U) { Packet item; if(pop(packet,&item)!=THREADWORK_OK) return THREADWORK_ERROR; printf("%d\n",item.value); }"#,
    );
    d.patch(
        &consumer,
        &json!({"text":{"initialization":"packet->capacity=1;"}}),
    )
    .unwrap();
    d.create_link(
        "packet",
        Link {
            source_node_id: producer.clone(),
            target_node_id: consumer,
            type_definition_id: packet,
            ..Default::default()
        },
    )
    .unwrap();
    let (project, output) = build_run(d).await;
    assert_eq!(output, "1\n2\n3\n");
    assert_eq!(project.capability, "c17-topology");
    let mapped = project.files[0]
        .source_map
        .iter()
        .find(|m| m.node_id == producer && m.section == "source")
        .unwrap();
    let line = project.files[0]
        .content
        .lines()
        .nth(mapped.generated_start_line - 1)
        .unwrap();
    assert!(line.contains("static int emitted=0"));
}
#[tokio::test]
async fn nested_composites_payload_types_library_aliases_and_initialization() {
    let mut d = project();
    let library = processor(
        &mut d,
        "math-library",
        "int twice(int value) { return value*2; }",
    );
    let group = d.create("root", "nested", NodeKind::Group).unwrap();
    let producer = processor(
        &mut d,
        "producer",
        r#"static int emitted=0; if(!emitted) { Packet p={math__twice(21)}; push(result,&p); emitted=1; } else { threadwork_runner__shutdown_request(&threadwork_runner); }"#,
    );
    d.move_node(&producer, Some(&group)).unwrap();
    let sink = processor(
        &mut d,
        "sink",
        r#"if(threadwork_buffer_count(result)>0U) { Packet p; pop(result,&p); printf("answer=%d\n",p.value); }"#,
    );
    d.create_link(
        "math",
        Link {
            source_node_id: library,
            target_node_id: producer,
            interaction_kind: "lib".into(),
            ..Default::default()
        },
    )
    .unwrap();
    d.create_link(
        "result",
        Link {
            source_node_id: d
                .nodes
                .values()
                .find(|n| n.name == "producer")
                .unwrap()
                .id
                .clone(),
            target_node_id: sink,
            type_name: "Packet".into(),
            payload_definition: "typedef struct Packet { int value; } Packet;".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let (_, output) = build_run(d).await;
    assert_eq!(output, "answer=42\n");
}
#[tokio::test]
async fn self_feedback_uses_explicit_in_out_arguments() {
    let mut d = project();
    let worker = processor(
        &mut d,
        "feedback",
        r#"if(threadwork_buffer_count(loop_in)>0U) { int value; pop(loop_in,&value); printf("%d\n",value); if(value>0) { value--; push(loop_out,&value); } else { threadwork_runner__shutdown_request(&threadwork_runner); } }"#,
    );
    d.patch(&worker,&json!({"text":{"initialization":"int initial=2; if(push(loop_out,&initial)!=THREADWORK_OK) return THREADWORK_ERROR;"}})).unwrap();
    d.create_link(
        "loop",
        Link {
            source_node_id: worker.clone(),
            target_node_id: worker,
            type_definition_id: "int".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let (_, output) = build_run(d).await;
    assert_eq!(output, "2\n1\n0\n");
}
#[test]
fn unsupported_capabilities_and_conflicting_types_fail_before_writing() {
    let mut d = project();
    let a = processor(&mut d, "a", "");
    let b = processor(&mut d, "b", "");
    let link = d
        .create_link(
            "data",
            Link {
                source_node_id: a,
                target_node_id: b,
                type_name: "Packet".into(),
                payload_definition: "typedef struct Packet { int value; } Packet;".into(),
                ..Default::default()
            },
        )
        .unwrap();
    d.patch(&link, &json!({"link":{"interactionKind":"run"}}))
        .unwrap();
    assert_eq!(C17::generate(&d, "root").err().unwrap().code, "unsupported");
    d.patch(&link, &json!({"link":{"interactionKind":"data"}}))
        .unwrap();
    d.patch("root", &json!({"fileLayoutStrategyId":"source-set"}))
        .unwrap();
    assert!(C17::generate(&d, "root").is_err());
}

#[tokio::test]
async fn generated_clangd_diagnostics_map_back_to_node_section_and_line() {
    let mut d = project();
    let worker = processor(
        &mut d,
        "worker",
        "int ok=1;\n\nint value=missing_identifier;\n(void)ok; (void)value;",
    );
    let app = Application::new(Service::new(d, None), "fixture".into(), None);
    let result = app
        .execute(Request::new("editor.analyze_generated", json!({})))
        .await
        .unwrap();
    let diagnostic = result["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| {
            d["message"]
                .as_str()
                .unwrap_or("")
                .contains("missing_identifier")
        })
        .expect("clangd error on the authored body");
    assert_eq!(diagnostic["source"]["nodeId"], worker);
    assert_eq!(diagnostic["source"]["section"], "source");
    assert_eq!(diagnostic["source"]["line"], 3);
    assert_eq!(diagnostic["source"]["column"], 11);
    let pid = result["pid"].as_u64().unwrap();
    app.execute(Request::new(
        "threadwork.update_entity",
        json!({"nodeId":worker,"patch":{"text":{"source":"int value=1; (void)value;"}}}),
    ))
    .await
    .unwrap();
    let fixed = app
        .execute(Request::new("editor.analyze_generated", json!({})))
        .await
        .unwrap();
    assert!(
        fixed["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["severity"] != 1),
        "{fixed}"
    );
    app.shutdown().await;
    #[cfg(target_os = "linux")]
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
}

#[test]
fn conflicting_payload_definitions_fail_and_identical_definitions_are_emitted_once() {
    let mut d = project();
    let a = processor(&mut d, "source", "");
    let b = processor(&mut d, "first", "");
    let c = processor(&mut d, "second", "");
    d.create_link(
        "first",
        Link {
            source_node_id: a.clone(),
            target_node_id: b,
            type_name: "Packet".into(),
            payload_definition: "typedef struct Packet { int value; } Packet;".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let second = d
        .create_link(
            "second",
            Link {
                source_node_id: a,
                target_node_id: c,
                type_name: "Packet".into(),
                payload_definition: "typedef struct Packet { double value; } Packet;".into(),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        C17::generate(&d, "root")
            .err()
            .unwrap()
            .message
            .contains("Conflicting wire payload")
    );
    d.patch(
        &second,
        &json!({"link":{"payloadDefinition":"typedef struct Packet {  int value; } Packet;"}}),
    )
    .unwrap();
    let compiled = C17::generate(&d, "root").unwrap();
    assert_eq!(
        compiled.files[0]
            .content
            .matches("typedef struct Packet")
            .count(),
        1
    );
}

#[test]
fn cli_compile_preserves_the_original_project_and_emits_a_c_translation_unit() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("graph.orch");
    let output = temp.path().join("generated");
    let mut d = project();
    processor(
        &mut d,
        "stop",
        "threadwork_runner__shutdown_request(&threadwork_runner);",
    );
    wirecraft_core::store::save(&d, &path).unwrap();
    let before = std::fs::read(&path).unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_wirecraft"))
        .arg("compile")
        .arg(&path)
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(output.join("C_network.c").exists());
}

#[tokio::test]
async fn source_capability_materializes_provider_declaration_for_its_consumer_only() {
    let mut d = project();
    let provider_source = "/* emit ${greeting}, \"quoted\" \\ ü?? ${greeting} */\n\t(void)0;";
    let provider = processor(&mut d, "producer", provider_source);
    let consumer = processor(
        &mut d,
        "consumer",
        r#"static int materialized = 0;
threadwork_build_parameter parameters[] = { { "greeting", "hello" } };
if (materialized) return THREADWORK_OK;
materialized = 1;
char *product = producer_source->getSource(producer_source, parameters, 1U);
if (product == NULL) return THREADWORK_ERROR;
printf("%s\n", product);
free(product);
if (threadwork_runner__shutdown_request(&threadwork_runner) != THREADWORK_OK) return THREADWORK_ERROR;"#,
    );
    let link = d
        .create_link(
            "producer_source",
            Link {
                source_node_id: provider.clone(),
                source_port_name: "source".into(),
                target_node_id: consumer.clone(),
                target_port_name: "builder".into(),
                ..Default::default()
            },
        )
        .unwrap();
    d.patch(&link, &json!({"link":{"interactionKind":"src"}}))
        .unwrap();
    let original = serde_json::to_value(&d).unwrap();

    let generated = C17::generate(&d, "root").unwrap();
    let unit = &generated.files[0].content;
    assert!(unit.contains("typedef struct Producer_source1Capability {"));
    assert!(unit.contains("static Producer_source1Capability producer_source1 = {"));
    assert!(unit.contains("Producer_source1Capability *producer_source)"));
    assert!(unit.contains("&producer_source1)"));
    assert!(!unit.contains("producer_source1_a_port"));
    let provider_header = unit
        .lines()
        .find(|l| l.starts_with("static threadwork_error_t tw_run_producer_"))
        .unwrap();
    assert!(!provider_header.contains("Capability"), "{provider_header}");
    // Authored bodies keep their origin maps despite the extra declarations.
    let mapped = generated.files[0]
        .source_map
        .iter()
        .find(|m| m.node_id == consumer && m.section == "source")
        .unwrap();
    assert!(
        unit.lines()
            .nth(mapped.generated_start_line - 1)
            .unwrap()
            .contains("static int materialized = 0;")
    );

    let app = Application::new(Service::new(d.clone(), None), "fixture".into(), None);
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
            .contains("Producer_source1Capability *producer_source"),
        "{context}"
    );
    assert!(
        context["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["label"] == "producer_source"
                && s["documentation"].as_str().unwrap().contains("producer"))
    );
    let text = "producer_source->getS";
    let completion = app
        .execute(Request::new(
            "editor.complete",
            json!({"nodeId":consumer,"section":"source","text":text,"offset":text.len(),"generation":3}),
        ))
        .await
        .unwrap();
    assert!(
        completion["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["filterText"] == "getSource"
                || i["label"].as_str().is_some_and(|l| l.contains("getSource"))),
        "{completion}"
    );
    let text = "(void)producer_source;";
    let hover = app
        .execute(Request::new(
            "editor.hover",
            json!({"nodeId":consumer,"section":"source","text":text,"offset":text.find("producer_source").unwrap()+2}),
        ))
        .await
        .unwrap();
    assert!(
        hover["hover"]["contents"]
            .to_string()
            .contains("Producer_source1Capability"),
        "{hover}"
    );
    let broken = "char *p = producer_source->getSource(producer_source);\n(void)p;";
    app.execute(Request::new(
        "threadwork.update_entity",
        json!({"nodeId":consumer,"patch":{"text":{"source":broken}}}),
    ))
    .await
    .unwrap();
    let analysis = app
        .execute(Request::new("editor.analyze_generated", json!({})))
        .await
        .unwrap();
    let diagnostic = analysis["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["severity"] == 1)
        .expect("too few getSource arguments is reported");
    assert_eq!(diagnostic["source"]["nodeId"], consumer);
    assert_eq!(diagnostic["source"]["line"], 1);
    app.shutdown().await;

    let (_, output) = build_run(d.clone()).await;
    assert_eq!(
        output,
        "/* emit hello, \"quoted\" \\ ü?? hello */\n\t(void)0;\n"
    );
    assert_eq!(serde_json::to_value(&d).unwrap(), original);
}

#[test]
fn runnable_capabilities_are_rejected_with_the_baseline_reason() {
    let mut d = project();
    let a = processor(&mut d, "a", "");
    let b = processor(&mut d, "b", "");
    let link = d
        .create_link(
            "runner",
            Link {
                source_node_id: a,
                target_node_id: b,
                ..Default::default()
            },
        )
        .unwrap();
    d.patch(&link, &json!({"link":{"interactionKind":"run"}}))
        .unwrap();
    let error = C17::generate(&d, "root").err().unwrap();
    assert_eq!(error.code, "unsupported");
    assert!(
        error.message.contains("runnable capability"),
        "{}",
        error.message
    );
}

#[tokio::test]
async fn dependency_links_lend_one_library_instance_to_each_consumer_by_link_name() {
    let mut d = project();
    let library = processor(
        &mut d,
        "lib_math",
        r#"#include <limits.h>
static int clamp_value(int value) {
    if (value > INT_MAX / 2) {
        return INT_MAX / 2;
    }
    return value;
}
int maximum(int left, int right) {
    return clamp_value(left > right ? left : right);
}"#,
    );
    let first = processor(
        &mut d,
        "first",
        r#"static int done = 0;
if (!done) { printf("first %d\n", math->maximum(2, 4)); done = 1; }"#,
    );
    let group = d.create("root", "group", NodeKind::Processor).unwrap();
    let second = d.create(&group, "second", NodeKind::Processor).unwrap();
    d.patch(
        &second,
        &json!({"text":{"source":"static int done = 0;\nif (!done) { printf(\"second %d\\n\", numbers->maximum(7, 3)); done = 1; }\nif (threadwork_runner__shutdown_request(&threadwork_runner) != THREADWORK_OK) return THREADWORK_ERROR;"}}),
    )
    .unwrap();
    for (name, target) in [("math", &first), ("numbers", &second)] {
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
    let unit = C17::generate(&d, "root").unwrap().files[0].content.clone();
    // One exporter and one instance for the library, regardless of the number of links.
    assert_eq!(unit.matches("typedef struct Lib_mathLibrary {").count(), 1);
    assert_eq!(
        unit.matches("static const Lib_mathLibrary lib_math_library = {")
            .count(),
        1
    );
    assert!(unit.contains("    int (*maximum)(int left, int right);"));
    assert!(unit.contains("    int (*clamp_value)(int value);"));
    // Only file-scope declarations are exported, not statements inside bodies.
    assert!(!unit.contains("(*return)") && !unit.contains("(*if)"));
    assert!(unit.contains("const Lib_mathLibrary *math)"));
    assert!(unit.contains("const Lib_mathLibrary *numbers)"));
    assert!(unit.contains("&lib_math_library)"));
    assert!(!unit.contains("math__maximum") && !unit.contains("numbers__maximum"));

    let app = Application::new(Service::new(d.clone(), None), "fixture".into(), None);
    let context = app
        .execute(Request::new(
            "editor.context",
            json!({"nodeId":first,"section":"source"}),
        ))
        .await
        .unwrap();
    let symbol = context["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["label"] == "math")
        .cloned()
        .expect("library instance symbol");
    assert_eq!(symbol["detail"], "const Lib_mathLibrary *math");
    assert!(
        symbol["documentation"]
            .as_str()
            .unwrap()
            .contains("int maximum(int left, int right)")
    );
    let text = "math->max";
    let completion = app
        .execute(Request::new(
            "editor.complete",
            json!({"nodeId":first,"section":"source","text":text,"offset":text.len(),"generation":1}),
        ))
        .await
        .unwrap();
    assert!(
        completion["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["filterText"] == "maximum"
                || i["label"].as_str().is_some_and(|l| l.contains("maximum"))),
        "{completion}"
    );
    app.shutdown().await;

    let (_, output) = build_run(d).await;
    assert_eq!(output, "first 4\nsecond 7\n");
}

#[test]
fn libraries_are_emitted_after_the_libraries_they_depend_on() {
    let mut d = project();
    // ID order would place "a_http" before "z_base"; the library link must win.
    let http = processor(
        &mut d,
        "a_http_lib",
        "static int http_ready(void) { return EAGAIN_LIKE; }",
    );
    let base = processor(
        &mut d,
        "z_base_lib",
        "#include <errno.h>\n#define EAGAIN_LIKE 11",
    );
    let link = d
        .create_link(
            "base",
            Link {
                source_node_id: base,
                target_node_id: http.clone(),
                ..Default::default()
            },
        )
        .unwrap();
    d.patch(&link, &json!({"link":{"interactionKind":"lib"}}))
        .unwrap();
    let worker = processor(&mut d, "worker", "(void)0;");
    let used = d
        .create_link(
            "http",
            Link {
                source_node_id: http,
                target_node_id: worker,
                ..Default::default()
            },
        )
        .unwrap();
    d.patch(&used, &json!({"link":{"interactionKind":"lib"}}))
        .unwrap();
    let unit = C17::generate(&d, "root").unwrap().files[0].content.clone();
    assert!(unit.find("#define EAGAIN_LIKE").unwrap() < unit.find("http_ready(void)").unwrap());
}

#[test]
fn standard_header_members_follow_exact_include_directives() {
    let mut d = project();
    // <sys/time.h> and a comment mentioning time.h must not add time/localtime_r/strftime.
    let library = processor(
        &mut d,
        "lib_clock",
        "/* see time.h */\n#include <sys/time.h>\nint ticks(void) { return 1; }",
    );
    let worker = processor(&mut d, "worker", "(void)clock->ticks();");
    let link = d
        .create_link(
            "clock",
            Link {
                source_node_id: library.clone(),
                target_node_id: worker,
                ..Default::default()
            },
        )
        .unwrap();
    d.patch(&link, &json!({"link":{"interactionKind":"lib"}}))
        .unwrap();
    let unit = C17::generate(&d, "root").unwrap().files[0].content.clone();
    assert!(unit.contains("int (*ticks)(void);"));
    assert!(!unit.contains("(*time)") && !unit.contains("(*strftime)"));
    d.patch(
        &library,
        &json!({"text":{"source":"#include <time.h>\nint ticks(void) { return 1; }"}}),
    )
    .unwrap();
    let unit = C17::generate(&d, "root").unwrap().files[0].content.clone();
    assert!(unit.contains("(*strftime)"));
}

/// The bundled WebServer example compiles under strict C17 and serves its specified routes.
#[test]
fn webserver_example_serves_routes_fallbacks_and_shuts_down_after_its_limit() {
    use std::io::{Read, Write};
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/webserver.orch");
    let d = wirecraft_core::store::load(&path).unwrap();
    let unit = C17::generate(&d, "root").unwrap();
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join(&unit.files[0].path);
    std::fs::write(&source, &unit.files[0].content).unwrap();
    let exe = temp.path().join("webserver");
    let built = std::process::Command::new("clang")
        .args(["-std=c17", "-Wall", "-Wextra", "-pedantic", "-Werror"])
        .arg(&source)
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut server = std::process::Command::new(&exe)
        .env("WEBSERVER_PORT", port.to_string())
        .env("WEBSERVER_MAX_REQUESTS", "5")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let exchange = |raw: &str| {
        for _ in 0..100 {
            if let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
                stream.write_all(raw.as_bytes()).unwrap();
                let mut reply = String::new();
                stream.read_to_string(&mut reply).unwrap();
                return reply;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("server did not accept connections");
    };
    let about = exchange("GET /home/about?x=1 HTTP/1.1\r\nHost: test\r\n\r\n");
    assert!(about.starts_with("HTTP/1.1 200 OK\r\n"), "{about}");
    assert!(about.contains("Content-Length: 90\r\n") && about.ends_with("Wirecraft model.\n"));
    let user = exchange("GET /api/user HTTP/1.1\r\n\r\n");
    assert!(
        user.contains("Content-Type: application/json\r\n") && user.contains("\"Ada Lovelace\"")
    );
    let missing = exchange("GET /missing HTTP/1.1\r\n\r\n");
    assert!(missing.starts_with("HTTP/1.1 404") && missing.ends_with("404 Not Found: /missing\n"));
    let post = exchange("POST /home/version HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc");
    assert!(post.starts_with("HTTP/1.1 405") && post.contains("Allow: GET, HEAD\r\n"));
    let head = exchange("HEAD /home/version HTTP/1.1\r\n\r\n");
    assert!(
        head.contains("Content-Length: 32\r\n") && head.ends_with("\r\n\r\n"),
        "{head}"
    );
    let status = server.wait().unwrap();
    let mut log = String::new();
    server
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut log)
        .unwrap();
    assert!(status.success(), "{log}");
    assert_eq!(
        log.lines().filter(|l| l.starts_with("req-")).count(),
        5,
        "{log}"
    );
    assert!(log.contains("req-4 POST /home/version 405 "), "{log}");
}
