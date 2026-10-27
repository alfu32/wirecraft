use serde_json::json;
use wirecraft_core::{
    Document, NodeKind,
    templates::{self, ASSETS, TemplateSet},
};
#[test]
fn every_original_template_parses_and_all_six_sets_resolve_roles() {
    let context = json!({"node":{"kind":{"name":"Processor"}},"link":{},"isCompilationRoot":true,"layoutStrategy":{"id":"single-file"}});
    for (path, source) in ASSETS.iter().filter(|(p, _)| p.ends_with(".peb")) {
        templates::render(source, &context).unwrap_or_else(|e| panic!("{path}: {e:?}"));
    }
    assert_eq!(
        TemplateSet::catalog()["templateSets"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
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
