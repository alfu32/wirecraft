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
        json!({"edges":self.out.values().map(BTreeSet::len).sum::<usize>(),"links":self.links,"cycles":self.cycles(),"degrees":self.out.iter().map(|(id,row)|json!({"nodeId":id,"fanOut":row.len(),"fanIn":self.incoming[id].len()})).collect::<Vec<_>>()})
    }
}
pub fn report(d: &Document) -> Value {
    let ids: Vec<_> = d
        .nodes
        .values()
        .filter(|n| !n.is_link())
        .map(|n| n.id.clone())
        .collect();
    let mut principal = Graph::new(&ids);
    let mut error = Graph::new(&ids);
    let mut dependency = Graph::new(&ids);
    let mut structural = Graph::new(&ids);
    let mut findings = vec![];
    for n in d.nodes.values() {
        if let Some(l) = &n.link {
            match relation(d, n) {
                Some(Relation::Principal) => {
                    principal.add(&n.id, &l.source_node_id, &l.target_node_id)
                }
                Some(Relation::Error) => error.add(&n.id, &l.source_node_id, &l.target_node_id),
                Some(_) => dependency.add(&n.id, &l.source_node_id, &l.target_node_id),
                _ => {}
            }
        } else if let Some(parent) = &n.parent_id {
            structural.add(&n.id, parent, &n.id);
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
            *id != &d.root_node_id
                && principal.out[*id].is_empty()
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
    json!({"counts":d.network_counts(),"graphs":{"principal":principal.summary(),"error":error.summary(),"dependency":dependency.summary(),"structural":structural.summary()},"isolatedNodes":isolated,"connectedComponents":components,"classifications":d.nodes.values().map(|n|json!({"nodeId":n.id,"stereotype":stereotype(d,n),"relation":relation(d,n)})).collect::<Vec<_>>(),"findings":findings,"limitations":["Layout optimization and actionable navigation are not yet ported.","Threshold defaults are currently fixed at fan-out 8."]})
}
