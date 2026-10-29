use super::*;
use std::collections::BTreeMap;
impl Wirecraft {
    pub(super) fn compiler_issues(&self) -> &[Value] {
        &self.compiler_issue_cache
    }
    pub(super) fn refresh_compiler_issues(&mut self) {
        let mut issues = BTreeMap::new();
        let d = &self.snapshot.document;
        let live_c17 = c17_checked(d) && d.effective(&d.root_node_id, "technologyId") == "c-native";
        for node in d.nodes.values() {
            for diagnostic in &node.diagnostics {
                // Results persisted by the baseline C compiler are superseded by the live C17 check.
                if live_c17 && diagnostic["sourcePluginId"] == "c-compiler" {
                    continue;
                }
                let severity = diagnostic["severity"].as_u64().or_else(|| {
                    match diagnostic["severity"]
                        .as_str()?
                        .to_ascii_lowercase()
                        .as_str()
                    {
                        "error" => Some(1),
                        "warning" => Some(2),
                        _ => None,
                    }
                });
                let Some(severity @ (1 | 2)) = severity else {
                    continue;
                };
                let mut d = diagnostic.clone();
                d["severity"] = json!(severity);
                if !d["source"].is_object() {
                    d["source"] = json!({"nodeId":node.id,"section":diagnostic["textSection"].as_str().unwrap_or("source").to_lowercase(),"line":diagnostic["line"].as_u64().unwrap_or(1),"column":diagnostic["column"].as_u64().unwrap_or(1)});
                }
                issues.insert(
                    format!("{}:{}:{}", node.id, d["message"], d["source"]["line"]),
                    d,
                );
            }
        }
        for diagnostic in &self.generated_diagnostics {
            if diagnostic["severity"] != 1 && diagnostic["severity"] != 2 {
                continue;
            }
            issues.insert(
                format!(
                    "{}:{}:{}",
                    diagnostic["source"]["nodeId"].as_str().unwrap_or(""),
                    diagnostic["message"],
                    diagnostic["source"]["line"]
                ),
                diagnostic.clone(),
            );
        }
        if let Some(message) = &self.compiler_failure {
            issues.insert(
                "compiler-service".into(),
                json!({"severity":2,"message":message}),
            );
        }
        self.compiler_issue_cache = issues.into_values().collect();
    }
    pub(super) fn queue_compiler_check(&mut self, cx: &mut Context<Self>) {
        self.compiler_again = true;
        self.compiler_timer = Some(cx.spawn(async move |this, cx| {
            Timer::after(std::time::Duration::from_millis(500)).await;
            let _ = this.update(cx, |this, _| {
                if this.compiler_inflight {
                    return;
                }
                this.compiler_again = false;
                let d = &this.snapshot.document;
                if !c17_checked(d) {
                    return;
                }
                let mut request =
                    Request::new("editor.analyze_generated", json!({"nodeId":d.root_node_id}));
                request.expected_revision = Some(this.snapshot.revision);
                this.compiler_inflight = true;
                let _ = this.client.sender.send(Message::Execute(request));
            });
        }));
    }
    pub(super) fn compiler_finished(&mut self, cx: &mut Context<Self>) {
        self.compiler_inflight = false;
        if self.compiler_again {
            self.queue_compiler_check(cx);
        }
    }
}
/// Whether the live C17 or Rust (`cargo check`) analysis owns this design's compiler
/// diagnostics. A component (a design with Exterior ports) generates only through a
/// design that references it; the Rust generator supports no components yet.
fn c17_checked(d: &wirecraft_core::Document) -> bool {
    let root = &d.root_node_id;
    let exterior = d
        .nodes
        .values()
        .any(wirecraft_core::components::is_exterior);
    match d.effective(root, "technologyId").as_str() {
        "c-native" => d.effective(root, "languageId") == "c" && !exterior,
        "rust" => {
            d.effective(root, "languageId") == "rust"
                && !exterior
                && !d
                    .nodes
                    .values()
                    .any(wirecraft_core::components::is_reference)
        }
        _ => false,
    }
}
