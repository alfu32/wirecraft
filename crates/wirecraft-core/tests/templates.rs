use serde_json::json;
use wirecraft_core::{
    Document, NodeKind,
    templates::{self, ASSETS, TemplateSet},
};
#[test]
fn every_template_parses_and_all_seven_sets_resolve_roles() {
    let context = json!({"node":{"kind":{"name":"Processor"}},"link":{},"isCompilationRoot":true,"layoutStrategy":{"id":"single-file"}});
    for (path, source) in ASSETS.iter().filter(|(p, _)| p.ends_with(".peb")) {
        templates::render(source, &context).unwrap_or_else(|e| panic!("{path}: {e:?}"));
    }
    assert_eq!(
        TemplateSet::catalog()["templateSets"]
            .as_array()
            .unwrap()
            .len(),
        7
    );
    // The Wirecraft-authored Rust set has no Threadworks baseline revision.
    assert!(TemplateSet::catalog()["templateSets"][6]["sourceRevision"].is_null());
}
#[test]
fn pebble_branches_legacy_placeholders_and_graph_overrides_are_preserved() {
    let text = "{% for n in nodes %}{% if n.enabled %}${n.name}{% elseif n.name is not empty %}off={{n.name}}{% endif %}{% else %}none{% endfor %}";
    assert_eq!(
        templates::render(
            text,
            &json!({"nodes":[{"name":"A","enabled":true},{"name":"B"}]})
        )
        .unwrap(),
        "Aoff=B"
    );
    let mut d = Document::new("overrides");
    let id = d
        .create("root", "@ProcessorDeclaration", NodeKind::Processor)
        .unwrap();
    d.patch(&id, &json!({"text":{"source":"custom {{node.name}}"}}))
        .unwrap();
    let mut set = TemplateSet::builtin("go").unwrap();
    set.overlay(&d).unwrap();
    assert_eq!(set.roles["processor.declaration"], "custom {{node.name}}");
    assert!(templates::render("{% if x %}", &json!({})).is_err());
}

#[test]
fn rendered_output_is_bounded_and_overrides_are_case_insensitive() {
    assert!(templates::render("{{ 'x' * 17000000 }}", &json!({})).is_err());
    let mut d = Document::new("case");
    let id = d
        .create("root", "@processor_declaration", NodeKind::Processor)
        .unwrap();
    d.patch(&id, &json!({"text":{"source":"case-insensitive"}}))
        .unwrap();
    let mut set = TemplateSet::builtin("go").unwrap();
    set.overlay(&d).unwrap();
    assert_eq!(set.roles["processor.declaration"], "case-insensitive");
    // Projects stored with the former QuickJS technology still select the systemJS set.
    assert_eq!(
        wirecraft_core::compiler::template_set_id("quickjs"),
        "systemjs"
    );
    assert_eq!(TemplateSet::builtin("quickjs").unwrap().id, "systemjs");
    for name in ["go", "nodejs", "kotlin", "php", "systemjs", "rust"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../examples/{name}.orch"));
        let d = wirecraft_core::store::load(&path).unwrap();
        let generated =
            wirecraft_core::compiler::generate(&d, &d.root_node_id, Default::default()).unwrap();
        assert!(!generated.files.is_empty());
    }
}
