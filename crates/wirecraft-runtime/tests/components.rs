//! Pluggable components: Exterior ports, pinned References and their expansion.
use serde_json::{Value, json};
use std::path::Path;
use wirecraft_core::{Document, c17::C17, components, store};
use wirecraft_runtime::service::{Request, Service};

fn run(s: &mut Service, command: &str, arguments: Value) -> Value {
    s.execute(Request::new(command, arguments))
        .unwrap_or_else(|e| panic!("{command}: {}", e.message))
}
fn entity(s: &mut Service, name: &str, source: &str) -> String {
    let id = run(s, "threadwork.create_entity", json!({"name":name}))["entity"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    run(
        s,
        "threadwork.update_entity",
        json!({"nodeId":id,"patch":{"text":{"source":source},"technology":{"languageId":"c","technologyId":"c-native"}}}),
    );
    id
}
fn errors(d: &Document) -> Vec<String> {
    components::diagnostics(d)
        .into_iter()
        .filter(|e| e.severity == "Error")
        .map(|e| e.message)
        .collect()
}
fn link_named<'a>(d: &'a Document, name: &str) -> &'a wirecraft_core::Node {
    d.nodes
        .values()
        .find(|n| n.is_link() && n.name == name)
        .unwrap()
}

/// A component doubling every number it receives on `numbers` and publishing it on `doubled`.
fn component(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("doubler.orch");
    let mut s = Service::new(Document::new("Doubler"), Some(path.clone()));
    let root = s.document.root_node_id.clone();
    run(
        &mut s,
        "threadwork.update_entity",
        json!({"nodeId":root,"patch":{"technology":{"languageId":"c","technologyId":"c-native"}}}),
    );
    let double = entity(
        &mut s,
        "double",
        "int value;\nwhile (threadwork_buffer_count(numbers) > 0U) {\n    if (pop(numbers, &value) != THREADWORK_OK) return THREADWORK_ERROR;\n    value *= 2;\n    if (push(doubled, &value) != THREADWORK_OK) return THREADWORK_ERROR;\n}",
    );
    let out = run(
        &mut s,
        "components.publish",
        json!({"nodeId":double,"direction":"output","name":"doubled","typeDefinitionId":"int"}),
    );
    let input = run(
        &mut s,
        "components.publish",
        json!({"nodeId":double,"direction":"input","name":"numbers","typeDefinitionId":"int"}),
    );
    let d = &s.document;
    assert_eq!(
        d.nodes[out["exteriorId"].as_str().unwrap()].name,
        "doubledOut"
    );
    assert_eq!(
        d.nodes[input["exteriorId"].as_str().unwrap()].name,
        "numbersIn"
    );
    assert!(errors(d).is_empty(), "{:?}", errors(d));
    run(&mut s, "file.save", json!({}));
    path
}

#[test]
fn exteriors_take_their_name_from_their_link_and_require_a_type() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Service::new(Document::new("ports"), Some(dir.path().join("p.orch")));
    let a = entity(&mut s, "a", "");
    let published = run(
        &mut s,
        "components.publish",
        json!({"nodeId":a,"direction":"output","name":"result"}),
    );
    let exterior = published["exteriorId"].as_str().unwrap().to_string();
    let link = published["linkId"].as_str().unwrap().to_string();
    assert_eq!(s.document.nodes[&exterior].name, "resultOut");
    assert!(
        errors(&s.document)
            .iter()
            .any(|e| e.contains("needs an explicit link type"))
    );
    // Renaming the link renames the Exterior; renaming the Exterior itself does not stick.
    run(
        &mut s,
        "threadwork.update_entity",
        json!({"nodeId":link,"patch":{"name":"total"}}),
    );
    assert_eq!(s.document.nodes[&exterior].name, "totalOut");
    run(
        &mut s,
        "threadwork.update_entity",
        json!({"nodeId":exterior,"patch":{"name":"other"}}),
    );
    assert_eq!(s.document.nodes[&exterior].name, "totalOut");
    run(
        &mut s,
        "threadwork.update_link",
        json!({"nodeId":link,"patch":{"typeDefinitionId":"int"}}),
    );
    assert!(errors(&s.document).is_empty());
    // A second output with the same name gets a distinct one.
    let again = run(
        &mut s,
        "components.publish",
        json!({"nodeId":a,"direction":"output","name":"total","typeDefinitionId":"int"}),
    );
    assert_eq!(
        s.document.nodes[again["exteriorId"].as_str().unwrap()].name,
        "total2Out"
    );
    // An Exterior with a second link is rejected by validation.
    let b = entity(&mut s, "b", "");
    run(
        &mut s,
        "threadwork.create_link",
        json!({"name":"extra","sourceNodeId":exterior,"targetNodeId":b}),
    );
    assert!(
        errors(&s.document)
            .iter()
            .any(|e| e.contains("exactly one link"))
    );
    // A component cannot be generated on its own.
    let root = s.document.root_node_id.clone();
    let Err(error) = C17::generate(&s.document, &root) else {
        panic!("a component must not generate on its own");
    };
    assert!(
        error.message.contains("is a component"),
        "{}",
        error.message
    );
}

#[test]
fn references_pin_ports_check_links_and_expand_with_joined_channels() {
    let dir = tempfile::tempdir().unwrap();
    let component_path = component(dir.path());
    let path = dir.path().join("app.orch");
    let mut s = Service::new(Document::new("App"), Some(path.clone()));
    let root = s.document.root_node_id.clone();
    run(
        &mut s,
        "threadwork.update_entity",
        json!({"nodeId":root,"patch":{"technology":{"languageId":"c","technologyId":"c-native"}}}),
    );
    run(&mut s, "file.save", json!({}));
    let reference = run(
        &mut s,
        "components.reference",
        json!({"path":component_path,"name":"twice"}),
    );
    let r = reference["nodeId"].as_str().unwrap().to_string();
    assert_eq!(reference["reference"]["path"], "doubler.orch");
    assert_eq!(
        reference["reference"]["ports"],
        json!([{"name":"doubled","direction":"Output","typeName":"int"},{"name":"numbers","direction":"Input","typeName":"int"}])
    );
    assert!(
        components::diagnostics(&s.document)
            .iter()
            .any(|e| e.severity == "Warning" && e.message.contains("'doubled'"))
    );
    // Two producers fan in to `numbers`; the port is chosen by type and direction.
    let first = entity(
        &mut s,
        "first",
        "static int sent = 0;\nint value = 20;\nif (!sent) {\n    sent = 1;\n    if (push(one, &value) != THREADWORK_OK) return THREADWORK_ERROR;\n}",
    );
    let second = entity(
        &mut s,
        "second",
        "static int sent = 0;\nint value = 1;\nif (!sent) {\n    sent = 1;\n    if (push(two, &value) != THREADWORK_OK) return THREADWORK_ERROR;\n}",
    );
    let total = entity(
        &mut s,
        "total",
        "static int sum = 0, seen = 0;\nint value;\nwhile (threadwork_buffer_count(results) > 0U) {\n    if (pop(results, &value) != THREADWORK_OK) return THREADWORK_ERROR;\n    sum += value;\n    if (++seen == 2) {\n        printf(\"sum %d\\n\", sum);\n        (void)threadwork_runner__shutdown_request(&threadwork_runner);\n    }\n}",
    );
    for (name, source) in [("one", &first), ("two", &second)] {
        run(
            &mut s,
            "threadwork.create_link",
            json!({"name":name,"sourceNodeId":source,"targetNodeId":r,"typeDefinitionId":"int"}),
        );
    }
    run(
        &mut s,
        "threadwork.create_link",
        json!({"name":"results","sourceNodeId":r,"targetNodeId":total,"typeDefinitionId":"int"}),
    );
    let d = &s.document;
    assert_eq!(
        link_named(d, "one").link.as_ref().unwrap().target_port_name,
        "numbers"
    );
    assert_eq!(
        link_named(d, "results")
            .link
            .as_ref()
            .unwrap()
            .source_port_name,
        "doubled"
    );
    assert!(errors(d).is_empty(), "{:?}", errors(d));
    run(&mut s, "file.save", json!({}));

    // Reloading resolves the component relative to the file and generates one channel per port.
    let d = store::load(&path).unwrap();
    let unit = C17::generate(&d, &d.root_node_id).unwrap();
    let source = &unit.files[0].content;
    let shared = source
        .lines()
        .filter(|l| l.contains("transport_one") || l.contains("transport_two"))
        .filter(|l| l.contains("_a_port,&one1_b_port") || l.contains("_a_port,&two1_b_port"))
        .count();
    assert_eq!(shared, 2, "fan-in links must share one buffer:\n{source}");
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("app.c"), source).unwrap();
    let exe = temp.path().join("app");
    let built = std::process::Command::new("clang")
        .args(["-std=c17", "-Wall", "-Wextra", "-pedantic", "-Werror"])
        .arg(temp.path().join("app.c"))
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let output = std::process::Command::new(&exe).output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "sum 42\n");

    // Port rules: a second consumer of an output and a wrong type are errors.
    let extra = entity(&mut s, "extra", "");
    let fan_out = s
        .execute(Request::new(
            "threadwork.create_link",
            json!({"name":"copy","sourceNodeId":r,"targetNodeId":extra,"typeDefinitionId":"int"}),
        ))
        .unwrap();
    assert!(
        errors(&s.document)
            .iter()
            .any(|e| e.contains("feeds 2 links"))
    );
    let copy = fan_out["entity"]["id"].as_str().unwrap().to_string();
    run(&mut s, "threadwork.delete_entity", json!({"nodeId":copy}));
    let one = link_named(&s.document, "one").id.clone();
    run(
        &mut s,
        "threadwork.update_link",
        json!({"nodeId":one,"patch":{"typeDefinitionId":"double"}}),
    );
    assert!(
        errors(&s.document)
            .iter()
            .any(|e| e.contains("carries double")),
        "{:?} {:?}",
        link_named(&s.document, "one").link,
        components::diagnostics(&s.document)
    );
    run(
        &mut s,
        "threadwork.update_link",
        json!({"nodeId":one,"patch":{"typeDefinitionId":"int"}}),
    );
    run(&mut s, "file.save", json!({}));

    // Editing the component invalidates the pin until the reference is updated.
    let mut c = store::load(&component_path).unwrap();
    let double = c
        .nodes
        .values()
        .find(|n| n.name == "double")
        .unwrap()
        .id
        .clone();
    c.nodes.get_mut(&double).unwrap().text.specification = "Doubles numbers.".into();
    store::save(&c, &component_path).unwrap();
    let mut s = Service::new(store::load(&path).unwrap(), Some(path.clone()));
    assert!(
        errors(&s.document)
            .iter()
            .any(|e| e.contains("changed since it was pinned"))
    );
    let root = s.document.root_node_id.clone();
    assert!(C17::generate(&s.document, &root).is_err());
    let updated = run(&mut s, "components.update_reference", json!({"nodeId":r}));
    assert_eq!(updated["removedPorts"], json!([]));
    assert!(errors(&s.document).is_empty());
}

#[test]
fn reference_paths_stay_inside_the_design_directory() {
    let dir = tempfile::tempdir().unwrap();
    let inner = dir.path().join("inner");
    std::fs::create_dir(&inner).unwrap();
    component(dir.path());
    let path = inner.join("app.orch");
    let mut s = Service::new(Document::new("App"), Some(path.clone()));
    run(&mut s, "file.save", json!({}));
    let outside = s
        .execute(Request::new(
            "components.reference",
            json!({"path":dir.path().join("doubler.orch")}),
        ))
        .unwrap_err();
    assert!(outside.message.contains("inside the design's directory"));
    // A hand-written escaping path is reported instead of being read.
    let id = run(
        &mut s,
        "threadwork.create_entity",
        json!({"name":"escape","kind":"Node","attributes":{"stereotype":"Reference","reference":{"path":"../doubler.orch"}}}),
    )["entity"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    run(&mut s, "file.save", json!({}));
    let d = store::load(&path).unwrap();
    assert!(
        components::diagnostics(&d)
            .iter()
            .any(|e| e.node_id.as_deref() == Some(id.as_str())
                && e.message.contains("inside the design's directory"))
    );
}

#[test]
fn ports_inside_composites_publish_expand_and_stay_visible_when_collapsed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested.orch");
    let mut s = Service::new(Document::new("Nested"), Some(path.clone()));
    let root = s.document.root_node_id.clone();
    run(
        &mut s,
        "threadwork.update_entity",
        json!({"nodeId":root,"patch":{"technology":{"languageId":"c","technologyId":"c-native"}}}),
    );
    let stage = entity(&mut s, "stage", "");
    let double = run(
        &mut s,
        "threadwork.create_entity",
        json!({"name":"double","parentId":stage,"attributes":{"text":{"source":"int value;\nwhile (threadwork_buffer_count(numbers) > 0U) {\n    if (pop(numbers, &value) != THREADWORK_OK) return THREADWORK_ERROR;\n    value *= 2;\n    if (push(doubled, &value) != THREADWORK_OK) return THREADWORK_ERROR;\n}"}}}),
    )["entity"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    // A port placed on its own is only a warning until it is linked.
    let placed = run(
        &mut s,
        "threadwork.create_entity",
        json!({"name":"port","kind":"Node","parentId":stage,"attributes":{"stereotype":"Exterior"}}),
    )["entity"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(errors(&s.document).is_empty(), "{:?}", errors(&s.document));
    assert!(
        components::diagnostics(&s.document)
            .iter()
            .any(|e| e.severity == "Warning" && e.message.contains("not linked yet"))
    );
    // Linking it from an entity makes it an output named after the link.
    run(
        &mut s,
        "threadwork.create_link",
        json!({"name":"doubled","sourceNodeId":double,"targetNodeId":placed,"link":{"typeDefinitionId":"int"}}),
    );
    assert_eq!(s.document.nodes[&placed].name, "doubledOut");
    let input = run(
        &mut s,
        "components.publish",
        json!({"nodeId":double,"direction":"input","name":"numbers","typeDefinitionId":"int","parentId":stage}),
    );
    let input = input["exteriorId"].as_str().unwrap().to_string();
    assert_eq!(
        s.document.nodes[&input].parent_id.as_deref(),
        Some(stage.as_str())
    );
    assert!(errors(&s.document).is_empty(), "{:?}", errors(&s.document));
    assert_eq!(
        components::published_ports(&s.document)
            .into_iter()
            .map(|(p, _)| p.name)
            .collect::<Vec<_>>(),
        ["doubled", "numbers"]
    );

    // Collapsed, the composite keeps both ports beside it: input left, output right.
    run(
        &mut s,
        "threadwork.update_entity",
        json!({"nodeId":stage,"patch":{"layout":{"isExpanded":false}}}),
    );
    let drawing = wirecraft_core::drawing::Drawing::new(&s.document, &root);
    let box_ = drawing.shapes[&stage].rect;
    for (id, right) in [(&placed, true), (&input, false)] {
        let port = &drawing.shapes[id];
        assert_eq!(port.pinned_to.as_deref(), Some(stage.as_str()));
        if right {
            assert!(port.rect.x > box_.x + box_.width);
        } else {
            assert!(port.rect.x + port.rect.width < box_.x);
        }
        assert!(
            drawing
                .edges
                .values()
                .any(|e| (e.source == stage && &e.target == id)
                    || (&e.source == id && e.target == stage)),
            "the port link is drawn from the collapsed composite"
        );
    }
    assert!(!drawing.shapes.contains_key(&double));
    run(&mut s, "file.save", json!({}));

    // A consumer feeds 21 and prints what comes back.
    let app = dir.path().join("app.orch");
    let mut s = Service::new(Document::new("App"), Some(app.clone()));
    let root = s.document.root_node_id.clone();
    run(
        &mut s,
        "threadwork.update_entity",
        json!({"nodeId":root,"patch":{"technology":{"languageId":"c","technologyId":"c-native"}}}),
    );
    run(&mut s, "file.save", json!({}));
    let r = run(
        &mut s,
        "components.reference",
        json!({"path":"nested.orch"}),
    )["nodeId"]
        .as_str()
        .unwrap()
        .to_string();
    let feed = entity(
        &mut s,
        "feed",
        "static int sent = 0;\nint value = 21;\nif (!sent) {\n    sent = 1;\n    if (push(numbers, &value) != THREADWORK_OK) return THREADWORK_ERROR;\n}",
    );
    let show = entity(
        &mut s,
        "show",
        "int value;\nwhile (threadwork_buffer_count(doubled) > 0U) {\n    if (pop(doubled, &value) != THREADWORK_OK) return THREADWORK_ERROR;\n    printf(\"got %d\\n\", value);\n    (void)threadwork_runner__shutdown_request(&threadwork_runner);\n}",
    );
    run(
        &mut s,
        "threadwork.create_link",
        json!({"name":"numbers","sourceNodeId":feed,"targetNodeId":r,"typeDefinitionId":"int"}),
    );
    run(
        &mut s,
        "threadwork.create_link",
        json!({"name":"doubled","sourceNodeId":r,"targetNodeId":show,"typeDefinitionId":"int"}),
    );
    assert!(errors(&s.document).is_empty(), "{:?}", errors(&s.document));
    run(&mut s, "file.save", json!({}));
    let d = store::load(&app).unwrap();
    let unit = C17::generate(&d, &d.root_node_id).unwrap();
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("app.c"), &unit.files[0].content).unwrap();
    let exe = temp.path().join("app");
    let built = std::process::Command::new("clang")
        .args(["-std=c17", "-Wall", "-Wextra", "-pedantic", "-Werror"])
        .arg(temp.path().join("app.c"))
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let output = std::process::Command::new(&exe).output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "got 42\n");
}

#[test]
fn io_marking_is_an_extension_field_shown_as_a_badge() {
    let mut s = Service::new(Document::new("io"), None);
    let a = entity(&mut s, "reader", "");
    run(
        &mut s,
        "threadwork.update_entity",
        json!({"nodeId":a,"patch":{"io":"in"}}),
    );
    let root = s.document.root_node_id.clone();
    let drawing = wirecraft_core::drawing::Drawing::new(&s.document, &root);
    assert_eq!(drawing.shapes[&a].io, Some("IO-in"));
    let svg = wirecraft_core::export::svg(&s.document, &root).unwrap();
    assert!(svg.contains("io-badge") && svg.contains(">IO-in<"));
    // Clearing removes the field instead of storing null.
    run(
        &mut s,
        "threadwork.update_entity",
        json!({"nodeId":a,"patch":{"io":null}}),
    );
    assert!(!s.document.nodes[&a].extra.contains_key("io"));
    let drawing = wirecraft_core::drawing::Drawing::new(&s.document, &root);
    assert_eq!(drawing.shapes[&a].io, None);
}

#[test]
fn link_ends_reconnect_to_entities_or_to_a_new_port() {
    let mut s = Service::new(Document::new("rewire"), None);
    let a = entity(&mut s, "a", "");
    let b = entity(&mut s, "b", "");
    let stage = entity(&mut s, "stage", "");
    let c = run(
        &mut s,
        "threadwork.create_entity",
        json!({"name":"c","parentId":stage}),
    )["entity"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let link = run(
        &mut s,
        "threadwork.create_link",
        json!({"name":"data","sourceNodeId":a,"targetNodeId":b,"link":{"targetPortName":"values"}}),
    )["entity"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let root = s.document.root_node_id.clone();
    // The target end moves into the composite: default port name, old port pruned.
    run(
        &mut s,
        "links.reconnect",
        json!({"nodeId":link,"end":"target","entityId":c}),
    );
    let l = s.document.nodes[&link].link.clone().unwrap();
    assert_eq!(
        (l.target_node_id.as_str(), l.target_port_name.as_str()),
        (c.as_str(), "in")
    );
    assert_eq!(
        s.document.nodes[&link].parent_id.as_deref(),
        Some(root.as_str())
    );
    assert!(
        !s.document.nodes[&b]
            .ports
            .iter()
            .any(|p| p.name == "values")
    );
    assert!(s.document.nodes[&b].incoming_links.is_empty());
    assert_eq!(s.document.nodes[&c].incoming_links, [link.clone()]);
    // Dropped on empty canvas inside the composite, the source end becomes an input port.
    let moved = run(
        &mut s,
        "links.reconnect",
        json!({"nodeId":link,"end":"source","parentId":stage,"x":10.,"y":20.}),
    );
    let port = moved["entityId"].as_str().unwrap().to_string();
    assert!(components::is_exterior(&s.document.nodes[&port]));
    assert_eq!(s.document.nodes[&port].name, "dataIn");
    assert_eq!(
        s.document.nodes[&port].parent_id.as_deref(),
        Some(stage.as_str())
    );
    assert_eq!(
        s.document.nodes[&link].parent_id.as_deref(),
        Some(stage.as_str())
    );
    // One undo step restores the previous end.
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(
        s.document.nodes[&link]
            .link
            .as_ref()
            .unwrap()
            .source_node_id,
        a
    );
    assert!(!s.document.nodes.contains_key(&port));
    // A link cannot end on another link.
    let other = run(
        &mut s,
        "threadwork.create_link",
        json!({"name":"other","sourceNodeId":b,"targetNodeId":a}),
    )["entity"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        s.execute(Request::new(
            "links.reconnect",
            json!({"nodeId":link,"end":"source","entityId":other}),
        ))
        .is_err()
    );
}
