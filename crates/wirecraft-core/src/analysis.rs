//! Sparse graph analysis. Principal, error, and capability relations stay distinct.
use crate::{Document, Node, NodeKind};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relation {
    Principal,
    Error,
    Dependency,
    TypeUsage,
    Source,
    Runnable,
}
pub fn relation(d: &Document, n: &Node) -> Option<Relation> {
    let l = n.link.as_ref()?;
    let explicit = match l.interaction_kind.as_str() {
        "lib" => Some(Relation::Dependency),
        "src" => Some(Relation::Source),
        "run" => Some(Relation::Runnable),
        "type-usage" => Some(Relation::TypeUsage),
        _ => None,
    };
    if explicit.is_some() {
        return explicit;
    }
    let src = d.nodes.get(&l.source_node_id);
    let dst = d.nodes.get(&l.target_node_id);
    if l.interaction_kind == "auto" {
        if src.is_some_and(|n| n.kind == NodeKind::Type) {
            return Some(Relation::TypeUsage);
        }
        if [
            "usage",
            "use",
            "import",
            "library",
            "lib",
            "dependency",
            "di",
            "inject",
            "injection",
        ]
        .contains(&l.transport_kind.as_str())
            || src.is_some_and(|n| named(&n.name, &["service", "client", "library", "lib"]))
            || named(&n.name, &["usage", "use", "import", "inject"])
        {
            return Some(Relation::Dependency);
        }
    }
    if ["error", "error-pipe", "exception", "failure"].contains(&l.transport_kind.as_str())
        || dst.is_some_and(|n| named(&n.name, &["error", "err"]))
        || l.source_port_name.to_lowercase().contains("error")
        || l.target_port_name.to_lowercase().contains("error")
        || named(&n.name, &["error"])
    {
        Some(Relation::Error)
    } else {
        Some(Relation::Principal)
    }
}
fn named(name: &str, tokens: &[&str]) -> bool {
    let name = name.to_lowercase();
    tokens.iter().any(|t| {
        name == *t
            || ['_', '-', ' ']
                .iter()
                .any(|c| name.starts_with(&format!("{t}{c}")) || name.ends_with(&format!("{c}{t}")))
    })
}
pub fn stereotype(d: &Document, n: &Node) -> &'static str {
    if n.is_link() {
        return "Link";
    }
    if n.kind == NodeKind::Type {
        return "Type";
    }
    if n.kind == NodeKind::Node {
        return "Node";
    }
    if n.is_composite() {
        return if named(&n.name, &["error", "err"]) {
            "CompositeErrorHandler"
        } else if named(&n.name, &["test"]) {
            "TestSuite"
        } else {
            "CompositeWorker"
        };
    }
    if named(&n.name, &["service", "client", "library", "lib"]) {
        return "ServiceLibrary";
    }
    if named(&n.name, &["error", "err"]) {
        return "ErrorHandler";
    }
    if named(&n.name, &["test"]) {
        return "Test";
    }
    let data = |ids: &[String]| {
        ids.iter()
            .filter_map(|id| d.nodes.get(id))
            .filter(|n| matches!(relation(d, n), Some(Relation::Principal | Relation::Error)))
            .count()
    };
    match (data(&n.incoming_links), data(&n.outgoing_links)) {
        (0, 0) => {
            if n.kind == NodeKind::Processor {
                "ProcessingUnit"
            } else {
                "Script"
            }
        }
        (0, _) => "Generator",
        (_, 0) => "Sink",
        _ => "Transformer",
    }
}
#[derive(Default)]
struct Graph {
    out: BTreeMap<String, BTreeSet<String>>,
    incoming: BTreeMap<String, BTreeSet<String>>,
    links: Vec<String>,
}
impl Graph {
    fn new(ids: &[String]) -> Self {
        Self {
            out: ids.iter().map(|id| (id.clone(), BTreeSet::new())).collect(),
            incoming: ids.iter().map(|id| (id.clone(), BTreeSet::new())).collect(),
            links: vec![],
        }
    }
    fn add(&mut self, id: &str, a: &str, b: &str) {
        if let Some(row) = self.out.get_mut(a) {
            row.insert(b.into());
        }
        if let Some(row) = self.incoming.get_mut(b) {
            row.insert(a.into());
        }
        self.links.push(id.into());
    }
    fn cycles(&self) -> Vec<Vec<String>> {
        // Iterative Kosaraju avoids a stack overflow on deeply connected designs.
        let mut seen = BTreeSet::new();
        let mut order = vec![];
        for root in self.out.keys() {
            if seen.contains(root) {
                continue;
            }
            let mut stack = vec![(root.clone(), false)];
            while let Some((id, done)) = stack.pop() {
                if done {
                    order.push(id);
                    continue;
                }
                if !seen.insert(id.clone()) {
                    continue;
                }
                stack.push((id.clone(), true));
                for next in &self.out[&id] {
                    if !seen.contains(next) {
                        stack.push((next.clone(), false));
                    }
                }
            }
        }
        seen.clear();
        let mut cycles = vec![];
        for root in order.into_iter().rev() {
            if seen.contains(&root) {
                continue;
            }
            let mut component = vec![];
            let mut stack = vec![root];
            while let Some(id) = stack.pop() {
                if !seen.insert(id.clone()) {
                    continue;
                }
                stack.extend(self.incoming[&id].iter().cloned());
                component.push(id);
            }
            if component.len() > 1
                || component
                    .first()
                    .is_some_and(|id| self.out[id].contains(id))
            {
                component.sort();
                cycles.push(component);
            }
        }
        cycles
    }
    fn summary(&self) -> Value {
        json!({"edges":self.out.values().map(BTreeSet::len).sum::<usize>(),"links":self.links,"adjacency":self.out,"cycles":self.cycles(),"degrees":self.out.iter().map(|(id,row)|json!({"nodeId":id,"fanOut":row.len(),"fanIn":self.incoming[id].len()})).collect::<Vec<_>>()})
    }
}
pub fn report(d: &Document) -> Value {
    let mut ids: Vec<_> = d
        .nodes
        .values()
        .filter(|n| !n.is_link())
        .map(|n| n.id.clone())
        .collect();
    ids.sort_by_key(|id| (d.qualified_name(id), id.clone()));
    let mut principal = Graph::new(&ids);
    let mut error = Graph::new(&ids);
    let mut dependency = Graph::new(&ids);
    let mut structural = Graph::new(&ids);
    let mut containment = Graph::new(&ids);
    let mut findings = vec![];
    for n in d.nodes.values() {
        if let Some(l) = &n.link {
            match relation(d, n) {
                Some(Relation::Principal) => {
                    principal.add(&n.id, &l.source_node_id, &l.target_node_id);
                    structural.add(&n.id, &l.source_node_id, &l.target_node_id);
                }
                Some(Relation::Error) => {
                    error.add(&n.id, &l.source_node_id, &l.target_node_id);
                    structural.add(&n.id, &l.source_node_id, &l.target_node_id);
                }
                Some(Relation::Dependency | Relation::TypeUsage) => {
                    dependency.add(&n.id, &l.target_node_id, &l.source_node_id);
                    structural.add(&n.id, &l.target_node_id, &l.source_node_id);
                }
                _ => {}
            }
        } else if let Some(parent) = &n.parent_id {
            containment.add(&n.id, parent, &n.id);
        }
    }
    for (kind, graph) in [("principal", &principal), ("dependency", &dependency)] {
        for cycle in graph.cycles() {
            findings.push(json!({"severity":if kind=="dependency"{"WARNING"}else{"NOTICE"},"category":kind,"title":format!("{kind} cycle"),"affectedNodes":cycle,"recommendation":if kind=="dependency"{"Check whether a capability boundary can remove the cyclic dependency."}else{"Check that feedback has an intentional termination or buffering contract."}}));
        }
        for (id, targets) in &graph.out {
            if targets.len() >= 8 {
                findings.push(json!({"severity":"WARNING","category":kind,"title":"High fan-out","affectedNodes":[id],"value":targets.len(),"recommendation":"Inspect the node's responsibilities and coupling."}));
            }
        }
    }
    let isolated: Vec<_> = ids
        .iter()
        .filter(|id| {
            principal.out[*id].is_empty()
                && principal.incoming[*id].is_empty()
                && dependency.out[*id].is_empty()
                && dependency.incoming[*id].is_empty()
                && error.out[*id].is_empty()
                && error.incoming[*id].is_empty()
        })
        .cloned()
        .collect();
    let mut components = vec![];
    let mut seen = BTreeSet::new();
    for root in &ids {
        if root == &d.root_node_id || seen.contains(root) {
            continue;
        }
        let mut queue = VecDeque::from([root.clone()]);
        let mut component = vec![];
        while let Some(id) = queue.pop_front() {
            if !seen.insert(id.clone()) {
                continue;
            }
            component.push(id.clone());
            for graph in [&principal, &error, &dependency] {
                queue.extend(
                    graph.out[&id]
                        .iter()
                        .filter(|id| *id != &d.root_node_id)
                        .cloned(),
                );
                queue.extend(
                    graph.incoming[&id]
                        .iter()
                        .filter(|id| *id != &d.root_node_id)
                        .cloned(),
                );
            }
        }
        components.push(component);
    }
    let processing = ids
        .iter()
        .filter(|id| {
            let n = &d.nodes[*id];
            *id != &d.root_node_id
                && !matches!(n.kind, NodeKind::Type | NodeKind::Note)
                && stereotype(d, n) != "ServiceLibrary"
                && (matches!(n.kind, NodeKind::Processor | NodeKind::Group)
                    || !principal.out[*id].is_empty()
                    || !principal.incoming[*id].is_empty())
        })
        .cloned()
        .collect::<Vec<_>>();
    let libraries = ids
        .iter()
        .filter(|id| stereotype(d, &d.nodes[*id]) == "ServiceLibrary")
        .cloned()
        .collect::<Vec<_>>();
    let select = |graph: &Graph, kind: &str| {
        ids.iter()
            .filter(|id| {
                let input = graph.incoming[*id].len();
                let output = graph.out[*id].len();
                match kind {
                    "generator" => input == 0 && output > 0,
                    "sink" => input > 0 && output == 0,
                    "transformer" => input == 1 && output == 1,
                    "router" => input > 0 && output > 1,
                    "merge" => input > 1,
                    "fanOut" => output > 1,
                    _ => false,
                }
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    let uncovered = processing
        .iter()
        .filter(|id| error.out[*id].is_empty())
        .cloned()
        .collect::<Vec<_>>();
    let mut indicators = serde_json::Map::new();
    let metric = |label: &str, ids: &Vec<String>| json!({"label":label,"value":ids.len(),"affectedNodes":ids});
    indicators.insert(
        "overview".into(),
        json!([
            metric("Nodes", &ids),
            metric("Processing candidates", &processing),
            metric("Libraries", &libraries),
            metric("Structurally isolated", &isolated)
        ]),
    );
    indicators.insert(
        "principal".into(),
        json!([
            metric("Generator candidates", &select(&principal, "generator")),
            metric("Sink candidates", &select(&principal, "sink")),
            metric("Transformer candidates", &select(&principal, "transformer")),
            metric("Fan-out candidates", &select(&principal, "fanOut")),
            metric("Merge candidates", &select(&principal, "merge")),
            metric("Router candidates", &select(&principal, "router"))
        ]),
    );
    indicators.insert("error".into(),json!([metric("Error sources",&select(&error,"generator")),metric("Error sinks",&select(&error,"sink")),metric("Error collectors",&select(&error,"merge")),metric("Processing without an error route",&uncovered),{"label":"Processing with an error route (%)","value":if processing.is_empty(){0.}else{100.*(processing.len()-uncovered.len())as f64/processing.len()as f64},"affectedNodes":processing.iter().filter(|id|!uncovered.contains(id)).collect::<Vec<_>>()}]));
    let unused = libraries
        .iter()
        .filter(|id| dependency.incoming[*id].is_empty())
        .cloned()
        .collect::<Vec<_>>();
    indicators.insert(
        "dependency".into(),
        json!([
            metric("Library candidates", &libraries),
            metric("Unused library candidates", &unused),
            metric(
                "Shared library candidates",
                &libraries
                    .iter()
                    .filter(|id| dependency.incoming[*id].len() > 1)
                    .cloned()
                    .collect()
            ),
            metric(
                "Foundational library candidates",
                &libraries
                    .iter()
                    .filter(|id| dependency.out[*id].is_empty())
                    .cloned()
                    .collect()
            )
        ]),
    );
    if !isolated.is_empty() {
        findings.push(json!({"severity":"NOTICE","category":"overview","title":"Some entities are structurally isolated","affectedNodes":isolated,"recommendation":"Confirm whether these are intentionally standalone documentation, types or unfinished work."}));
    }
    if !uncovered.is_empty() {
        findings.push(json!({"severity":"WARNING","category":"error","title":"Processing candidates have no outgoing error route","affectedNodes":uncovered,"recommendation":"Add or document an error path where the runtime contract requires one. This is a review observation, not a validation failure."}));
    }
    if !unused.is_empty() {
        findings.push(json!({"severity":"NOTICE","category":"dependency","title":"Some library candidates have no dependants","affectedNodes":unused,"recommendation":"Remove stale libraries or add the explicit capability link that records their use."}));
    }
    for cycle in error.cycles() {
        findings.push(json!({"severity":"NOTICE","category":"error","title":"Error flow contains a cycle","affectedNodes":cycle,"recommendation":"Verify retry, backoff and termination behavior for this region."}));
    }
    json!({"nodeOrder":ids,"nodeLabels":ids.iter().map(|id|(id,d.qualified_name(id))).collect::<BTreeMap<_,_>>(),"indicators":indicators,"counts":d.network_counts(),"graphs":{"principal":principal.summary(),"error":error.summary(),"dependency":dependency.summary(),"structural":structural.summary(),"containment":containment.summary()},"isolatedNodes":isolated,"connectedComponents":components,"classifications":d.nodes.values().map(|n|json!({"nodeId":n.id,"stereotype":stereotype(d,n),"relation":relation(d,n)})).collect::<Vec<_>>(),"findings":findings,"limitations":["Layout optimization is not yet ported.","Threshold defaults are currently fixed at fan-out 8."]})
}
