//! Go to definition and Find references in Go, Kotlin, JavaScript (Node.js, systemJS) and
//! PHP bodies of the WebServer examples, mapped from their template programs.
//!
//! The model alone resolves library calls, model arguments and modeled types; those tests
//! run by default. The language-server tests are ignored; they use `gopls`,
//! `kotlin-language-server`, `typescript-language-server` and `intelephense` from PATH, or
//! `WIRECRAFT_TEST_GOPLS`, `WIRECRAFT_TEST_KOTLIN_LANGUAGE_SERVER`,
//! `WIRECRAFT_TEST_JAVASCRIPT_LANGUAGE_SERVER` and `WIRECRAFT_TEST_PHP_LANGUAGE_SERVER`.
use serde_json::{Value, json};
use wirecraft_core::{
    Document,
    editor::{body_context, body_language, compiler_language},
    store,
};
use wirecraft_runtime::{
    backend::Application,
    service::{Request, Service},
};

/// The WebServer example of a technology, the LSP language and the definition keyword.
const LANGUAGES: [(&str, &str, &str); 5] = [
    ("go", "go", "func"),
    ("kotlin", "kotlin", "fun"),
    ("nodejs", "javascript", "function"),
    ("systemjs", "javascript", "function"),
    ("php", "php", "function"),
];

fn example(technology: &str) -> Document {
    let path = format!(
        "{}/../../examples/webserver-{technology}.orch",
        env!("CARGO_MANIFEST_DIR")
    );
    store::decode(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn named(d: &Document, name: &str) -> String {
    d.nodes
        .values()
        .find(|n| n.name == name)
        .unwrap()
        .id
        .clone()
}
/// Offset of the `nth` occurrence of `word` (plus one character) in a body.
fn at(text: &str, word: &str, nth: usize) -> usize {
    text.match_indices(word).nth(nth).unwrap().0 + 1
}
/// One-based line and column of `word` in the line of `text` that contains `line`.
fn position(text: &str, line: &str, word: &str) -> (u64, u64) {
    let (i, found) = text
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains(line))
        .unwrap();
    let column = found[..found.find(word).unwrap()].chars().count();
    (i as u64 + 1, column as u64 + 1)
}
fn located(location: &Value) -> (String, u64, u64) {
    (
        location["source"]["nodeId"].as_str().unwrap_or("").into(),
        location["source"]["line"].as_u64().unwrap_or(0),
        location["source"]["column"].as_u64().unwrap_or(0),
    )
}

#[test]
fn template_bodies_have_contexts_with_model_arguments() {
    for (technology, language, _) in LANGUAGES {
        let d = example(technology);
        let respond = named(&d, "socket_respond");
        let library = named(&d, "lib_http_server");
        assert_eq!(body_language(&d, &respond, "source"), Some(language));
        // Completion, hover, coloring and checks stay C17 and Rust only.
        assert_eq!(compiler_language(&d, &respond, "source"), None);
        let context = body_context(&d, &respond, "source", None).unwrap();
        assert_eq!(context.language, language);
        assert!(
            context.header.contains("run_socket_respond"),
            "{technology}"
        );
        let lent = context
            .symbols
            .iter()
            .find(|s| s["label"] == "libhttp")
            .unwrap_or_else(|| panic!("{technology}: {:?}", context.symbols));
        assert_eq!(lent["nodeId"], library.as_str(), "{technology}");
        let mapped = &context.file.content.lines().collect::<Vec<_>>()
            [context.source.generated_start_line - 1];
        assert!(
            mapped.trim_start().starts_with("for") || mapped.contains("answered"),
            "{technology}: {mapped}"
        );
        // Library bodies are file-scope items.
        let items = body_context(&d, &library, "source", None).unwrap();
        assert!(items.header.contains("file scope"), "{technology}");
    }
    let go = body_context(
        &example("go"),
        &named(&example("go"), "router"),
        "source",
        None,
    )
    .unwrap();
    assert!(go.companions.iter().any(|f| f.path == "go.mod"));
    let kotlin = example("kotlin");
    let kotlin = body_context(&kotlin, &named(&kotlin, "router"), "source", None).unwrap();
    assert!(
        !kotlin
            .companions
            .iter()
            .any(|f| f.path.ends_with(".gradle.kts"))
    );
}

#[test]
fn the_model_follows_library_calls_arguments_and_types_in_every_language() {
    for (technology, _, keyword) in LANGUAGES {
        let d = example(technology);
        let respond = named(&d, "socket_respond");
        let library = named(&d, "lib_http_server");
        let body = d.nodes[&respond].text.source.clone();
        let library_text = d.nodes[&library].text.source.clone();
        let context = body_context(&d, &respond, "source", None).unwrap();
        // The library function called through the lent instance.
        let found = context.model_definitions(&body, at(&body, "http_render_header", 0));
        let (line, column) = position(
            &library_text,
            &format!("{keyword} http_render_header"),
            "http_render_header",
        );
        assert_eq!(
            found.iter().map(located).collect::<Vec<_>>(),
            [(library.clone(), line, column)],
            "{technology}"
        );
        assert_eq!(found[0]["kind"], "body");
        // The argument is the library; the modeled payload type is its Type.
        let found = context.model_definitions(&body, at(&body, "libhttp", 0));
        assert_eq!(found[0]["kind"], "entity", "{technology}");
        assert_eq!(found[0]["source"]["nodeId"], library.as_str());
        let types = d.nodes.values().find(|n| n.name == "ReqRep").unwrap();
        let typed = format!("{body}\nReqRep");
        let found = context.model_definitions(&typed, typed.len() - 2);
        assert_eq!(
            found[0]["source"]["nodeId"],
            types.id.as_str(),
            "{technology}"
        );
        // Another member access is not a model symbol.
        let member = body.find(".response").or(body.find("->response")).unwrap();
        let found = context.model_definitions(&body, member + 4);
        assert!(found.is_empty(), "{technology}: {found:?}");
        // References from the call and from the definition: the definition and the call.
        let call = position(&body, "http_render_header", "http_render_header");
        let expected = [
            (library.clone(), line, column),
            (respond.clone(), call.0, call.1),
        ];
        let found = context.model_references(&body, at(&body, "http_render_header", 0));
        let mut got = found.iter().map(located).collect::<Vec<_>>();
        got.sort();
        let mut sorted = expected.to_vec();
        sorted.sort();
        assert!(
            sorted.iter().all(|e| got.contains(e)),
            "{technology}: {got:?}"
        );
        let own = body_context(&d, &library, "source", None).unwrap();
        let offset = at(&library_text, &format!("{keyword} http_render_header"), 0) + keyword.len();
        let found = own.model_references(&library_text, offset);
        let got = found.iter().map(located).collect::<Vec<_>>();
        assert!(
            sorted.iter().all(|e| got.contains(e)),
            "{technology}: {got:?}"
        );
        // The definition in the edited library is the current body.
        assert!(
            found
                .iter()
                .any(|l| l["current"] == true && located(l).1 == line),
            "{technology}"
        );
    }
}

/// Settings whose server for `language` is `executable`.
fn settings(
    directory: &std::path::Path,
    key: &str,
    executable: &str,
    args: &[&str],
) -> std::path::PathBuf {
    let path = directory.join("settings.json");
    std::fs::write(
        &path,
        json!({ key: {"executable":executable,"args":args} }).to_string(),
    )
    .unwrap();
    path
}
fn navigate(command: &str, node: &str, text: &str, offset: usize) -> Request {
    Request::new(
        command,
        json!({"nodeId":node,"section":"source","text":text,"offset":offset,"generation":7}),
    )
}

#[tokio::test]
async fn navigation_without_a_language_server_answers_from_the_model() {
    let d = example("php");
    let respond = named(&d, "socket_respond");
    let library = named(&d, "lib_http_server");
    let body = d.nodes[&respond].text.source.clone();
    let directory = tempfile::tempdir().unwrap();
    let path = settings(
        directory.path(),
        "phpLanguageServer",
        "/nonexistent/wirecraft-php-language-server",
        &[],
    );
    let app = Application::new(Service::new(d, None), "fixture".into(), None)
        .with_settings(Some(path))
        .unwrap();
    let found = app
        .execute(navigate(
            "editor.definition",
            &respond,
            &body,
            at(&body, "http_render_header", 0),
        ))
        .await
        .unwrap();
    assert_eq!(found["server"], Value::Null);
    assert!(
        found["serverError"]
            .as_str()
            .unwrap()
            .contains("Cannot start")
    );
    assert_eq!(found["generation"], 7);
    assert_eq!(found["identifier"], "http_render_header");
    assert_eq!(found["locations"][0]["source"]["nodeId"], library.as_str());
    let found = app
        .execute(navigate(
            "editor.references",
            &respond,
            &body,
            at(&body, "http_render_header", 0),
        ))
        .await
        .unwrap();
    let nodes = found["locations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["source"]["nodeId"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(
        nodes.contains(&library) && nodes.contains(&respond),
        "{found}"
    );
}

/// The configured server of a language, from the test variable or PATH.
fn server(language: &str) -> (String, &'static str, Vec<&'static str>) {
    let (variable, key, executable, args) = match language {
        "go" => ("WIRECRAFT_TEST_GOPLS", "gopls", "gopls", vec![]),
        "kotlin" => (
            "WIRECRAFT_TEST_KOTLIN_LANGUAGE_SERVER",
            "kotlinLanguageServer",
            "kotlin-language-server",
            vec![],
        ),
        "javascript" => (
            "WIRECRAFT_TEST_JAVASCRIPT_LANGUAGE_SERVER",
            "javascriptLanguageServer",
            "typescript-language-server",
            vec!["--stdio"],
        ),
        _ => (
            "WIRECRAFT_TEST_PHP_LANGUAGE_SERVER",
            "phpLanguageServer",
            "intelephense",
            vec!["--stdio"],
        ),
    };
    (
        std::env::var(variable).unwrap_or_else(|_| executable.into()),
        key,
        args,
    )
}
/// Through the language server: a local binding, the library call (through the instance
/// member the server reaches), the argument, and references to the library function.
async fn language_server_navigation(technology: &str) {
    let (_, language, keyword) = LANGUAGES.iter().find(|l| l.0 == technology).unwrap();
    let d = example(technology);
    let respond = named(&d, "socket_respond");
    let library = named(&d, "lib_http_server");
    let body = d.nodes[&respond].text.source.clone();
    let library_text = d.nodes[&library].text.source.clone();
    let (executable, key, args) = server(language);
    let directory = tempfile::tempdir().unwrap();
    let path = settings(directory.path(), key, &executable, &args);
    let app = Application::new(Service::new(d, None), "fixture".into(), None)
        .with_settings(Some(path))
        .unwrap();
    let definition = |word: &str, nth: usize| {
        navigate("editor.definition", &respond, &body, at(&body, word, nth))
    };
    // `item` is declared on the line that first names it in each body.
    let found = app.execute(definition("item", 1)).await.unwrap();
    assert!(found["pid"].as_u64().is_some(), "{technology}: {found}");
    let local = &found["locations"][0];
    assert_eq!(local["current"], true, "{technology}: {found}");
    let declared = body
        .lines()
        .position(|l| l.contains("item") && !l.trim_start().starts_with("for"))
        .unwrap() as u64
        + 1;
    assert_eq!(local["source"]["line"], declared, "{technology}: {found}");
    let found = app
        .execute(definition("http_render_header", 0))
        .await
        .unwrap();
    let (line, column) = position(
        &library_text,
        &format!("{keyword} http_render_header"),
        "http_render_header",
    );
    assert_eq!(
        located(&found["locations"][0]),
        (library.clone(), line, column),
        "{technology}: {found}"
    );
    let found = app.execute(definition("libhttp", 0)).await.unwrap();
    assert_eq!(
        found["locations"][0]["kind"], "entity",
        "{technology}: {found}"
    );
    assert_eq!(found["locations"][0]["source"]["nodeId"], library.as_str());
    let found = app
        .execute(navigate(
            "editor.references",
            &respond,
            &body,
            at(&body, "http_render_header", 0),
        ))
        .await
        .unwrap();
    let got = found["locations"]
        .as_array()
        .unwrap()
        .iter()
        .map(located)
        .collect::<Vec<_>>();
    let call = position(&body, "http_render_header", "http_render_header");
    assert!(
        got.contains(&(library.clone(), line, column))
            && got.contains(&(respond.clone(), call.0, call.1)),
        "{technology}: {found}"
    );
    app.execute(Request::new("editor.stop", json!({})))
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires gopls on PATH or WIRECRAFT_TEST_GOPLS, and Go"]
async fn gopls_definitions_and_references_map_to_the_model() {
    language_server_navigation("go").await;
}
#[tokio::test]
#[ignore = "requires kotlin-language-server on PATH or WIRECRAFT_TEST_KOTLIN_LANGUAGE_SERVER"]
async fn kotlin_language_server_definitions_and_references_map_to_the_model() {
    language_server_navigation("kotlin").await;
}
#[tokio::test]
#[ignore = "requires typescript-language-server on PATH or WIRECRAFT_TEST_JAVASCRIPT_LANGUAGE_SERVER"]
async fn typescript_language_server_navigates_nodejs_bodies() {
    language_server_navigation("nodejs").await;
}
#[tokio::test]
#[ignore = "requires typescript-language-server on PATH or WIRECRAFT_TEST_JAVASCRIPT_LANGUAGE_SERVER"]
async fn typescript_language_server_navigates_systemjs_bodies() {
    language_server_navigation("systemjs").await;
}
#[tokio::test]
#[ignore = "requires intelephense on PATH or WIRECRAFT_TEST_PHP_LANGUAGE_SERVER"]
async fn intelephense_definitions_and_references_map_to_the_model() {
    language_server_navigation("php").await;
}
