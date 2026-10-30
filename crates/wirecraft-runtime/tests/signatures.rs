//! Read-only generated signatures shown with the editor: body headers as generated (with a
//! one-parameter-per-line form), Type declarations and link code, for C17 and every
//! template technology, component designs included.
use serde_json::{Value, json};
use wirecraft_core::{Document, editor::signature, store};
use wirecraft_runtime::{
    backend::Application,
    service::{Request, Service},
};

/// Technology, generated language and function keyword of each WebServer example.
const TEMPLATES: [(&str, &str, &str); 6] = [
    ("rust", "rust", "fn "),
    ("go", "go", "func "),
    ("kotlin", "kotlin", "fun "),
    ("nodejs", "javascript", "function "),
    ("systemjs", "javascript", "function "),
    ("php", "php", "function "),
];

fn load(path: &str) -> Document {
    let path = format!("{}/{path}", env!("CARGO_MANIFEST_DIR"));
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
fn dependency(d: &Document, name: &str, consumer: &str) -> String {
    let consumer = named(d, consumer);
    d.nodes
        .values()
        .find(|n| n.name == name && n.link.as_ref().unwrap().target_node_id == consumer)
        .unwrap()
        .id
        .clone()
}

#[test]
fn template_signatures_show_headers_types_and_links() {
    for (technology, language, function) in TEMPLATES {
        let d = load(&format!("../../examples/webserver-{technology}.orch"));
        let respond = signature(&d, &named(&d, "socket_respond"), "source").unwrap();
        assert_eq!(respond.language, language);
        // The one header line the generator emits, brace included.
        assert!(
            respond
                .text
                .starts_with(&format!("{function}run_socket_respond_"))
                && !respond.text.contains('\n')
                && respond.text.contains("libhttp")
                && respond.text.ends_with('{'),
            "{technology}: {}",
            respond.text
        );
        // Every argument on its own indented line.
        let wrapped = respond.wrapped.unwrap();
        let lines = wrapped.lines().collect::<Vec<_>>();
        assert!(lines[0].ends_with('('), "{technology}: {wrapped}");
        assert_eq!(lines.len(), 9, "{technology}: {wrapped}");
        assert!(lines[1..8].iter().all(|l| l.starts_with("    ")));
        assert!(lines[7].contains("libhttp") && !lines[7].ends_with(','));
        assert!(lines[8].starts_with(')') && lines[8].ends_with('{'));
        let init = signature(&d, &named(&d, "socket_respond"), "initialization").unwrap();
        assert!(
            init.text
                .starts_with(&format!("{function}init_socket_respond_"))
        );
        let library = signature(&d, &named(&d, "lib_http_server"), "source").unwrap();
        assert!(library.text.contains("service library items (file scope)"));
        assert!(library.wrapped.is_none());

        let request = signature(&d, &named(&d, "Request"), "source").unwrap();
        let id = named(&d, "Request");
        assert!(request.text.starts_with(&format!("// Request [{id}]")));
        assert!(request.text.contains("header") && request.text.contains("body"));
        assert!(
            !request.text.contains("Response"),
            "{technology}: {}",
            request.text
        );

        // The declaration block of the link, not the scheduler's call of its transport.
        let link = signature(&d, &named(&d, "routeHomeAboutRequest"), "source").unwrap();
        assert!(link.text.starts_with("// Link routeHomeAboutRequest\n"));
        for symbol in [
            "routeHomeAboutRequest3_a_port",
            "routeHomeAboutRequest3_b_port",
        ] {
            assert!(link.text.contains(symbol), "{technology}: {}", link.text);
        }
        assert!(
            link.text
                .contains(&format!("{function}transport_routeHomeAboutRequest3(")),
            "{technology}: {}",
            link.text
        );
        assert!(!link.text.contains("// Link routeHomeVersionRequest"));

        let lent = signature(&d, &dependency(&d, "libhttp", "socket_respond"), "source").unwrap();
        assert!(lent.text.starts_with(
            "// libhttp lends library lib_http_server to socket_respond as the argument\n"
        ));
        assert!(lent.text.lines().nth(1).unwrap().contains("libhttp"));
        let shared =
            signature(&d, &dependency(&d, "platform", "lib_http_server"), "source").unwrap();
        assert!(shared.text.contains("no argument is passed"));

        let specification = signature(&d, &named(&d, "socket_respond"), "specification");
        assert_eq!(specification.err().unwrap().code, "unsupported");
    }
}

#[test]
fn c17_signatures_show_prototypes_typedefs_and_transports() {
    let d = load("tests/fixtures/walkthrough.orch");
    let respond = signature(&d, &named(&d, "socket_respond"), "source").unwrap();
    assert_eq!(respond.language, "c");
    assert!(
        respond
            .text
            .starts_with("static threadwork_error_t tw_run_socket_respond_")
            && respond.text.ends_with(") {")
    );
    let wrapped = respond.wrapped.unwrap();
    let lines = wrapped.lines().collect::<Vec<_>>();
    assert_eq!(lines[1], "    threadwork_context *context,");
    assert_eq!(*lines.last().unwrap(), ") {");
    // C has no trailing comma after the last parameter.
    assert!(!lines[lines.len() - 2].ends_with(','));

    let ty = signature(&d, &named(&d, "Request"), "source").unwrap();
    assert!(ty.text.starts_with("typedef struct Request {\n"));
    assert!(ty.text.ends_with("} Request;"));

    let link = signature(&d, &named(&d, "routeHomeAboutRequest"), "source").unwrap();
    let lines = link.text.lines().collect::<Vec<_>>();
    assert_eq!(lines[0], "/* Transport routeHomeAboutRequest */");
    assert_eq!(lines.len(), 4);
    assert!(lines[3].starts_with("static threadwork_error_t transport_routeHomeAboutRequest"));

    let lent = signature(&d, &named(&d, "base_lib -> socket_accept"), "source").unwrap();
    assert!(
        lent.text.lines().nth(1).unwrap().starts_with("const "),
        "{}",
        lent.text
    );
}

#[test]
fn component_bodies_have_headers_with_their_exterior_ports() {
    let d = load("../../examples/webserver-component.orch");
    let respond = signature(&d, &named(&d, "socket_respond"), "source").unwrap();
    assert!(
        respond
            .text
            .starts_with("static threadwork_error_t tw_run_socket_respond_")
            && respond.text.ends_with(") {"),
        "{}",
        respond.text
    );
    // A published port is a buffer parameter, as in the program that references it.
    let exterior = d
        .nodes
        .values()
        .find(|n| wirecraft_core::components::is_exterior(n))
        .unwrap();
    let port = d
        .nodes
        .values()
        .find(|l| {
            l.link
                .as_ref()
                .is_some_and(|l| l.source_node_id == exterior.id || l.target_node_id == exterior.id)
        })
        .unwrap();
    let consumer = d
        .nodes
        .values()
        .find(|n| {
            !n.is_link()
                && port.link.as_ref().is_some_and(|l| {
                    (l.source_node_id == n.id || l.target_node_id == n.id) && n.id != exterior.id
                })
        })
        .unwrap();
    let header = signature(&d, &consumer.id, "source").unwrap();
    assert!(header.text.contains(port.name.as_str()), "{}", header.text);
    // The Exterior placement itself has no editable body and no header of its own.
    assert!(d.nodes[&exterior.id].text.source.trim().is_empty());
}

#[tokio::test]
async fn the_backend_serves_signatures_for_the_selected_revision() {
    let d = load("../../examples/webserver-go.orch");
    let link = named(&d, "routeHomeAboutRequest");
    let app = Application::new(Service::new(d, None), "fixture".into(), None);
    let found = app
        .execute(Request::new(
            "editor.signature",
            json!({"nodeId":link,"section":"source"}),
        ))
        .await
        .unwrap();
    assert_eq!(found["language"], "go");
    assert_eq!(found["nodeId"], link.as_str());
    assert!(found["revision"].is_u64());
    assert!(
        found["text"]
            .as_str()
            .unwrap()
            .contains("func transport_routeHomeAboutRequest3(")
    );
    assert_eq!(found["section"], Value::from("source"));
}
