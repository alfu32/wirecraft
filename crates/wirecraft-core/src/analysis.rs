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
/// Extra key holding a user's IO designation: `in`, `out` or `inout`.
pub const IO: &str = "io";
/// Badge for an entity the user marked as doing meaningful IO.
pub fn io_badge(n: &Node) -> Option<&'static str> {
    match n.extra.get(IO)?.as_str()? {
        "in" => Some("IO-in"),
        "out" => Some("IO-out"),
        "inout" => Some("IO-in/out"),
        _ => None,
    }
}
pub fn stereotype(d: &Document, n: &Node) -> &'static str {
    if n.is_link() {
        return "Link";
    }
    if crate::components::is_exterior(n) {
        return crate::components::EXTERIOR;
    }
    if crate::components::is_reference(n) {
        return crate::components::REFERENCE;
    }
    if n.kind == NodeKind::Type {
        return "Type";
    }
    if n.kind == NodeKind::Node {
        return "Node";
    }
    if n.name.trim().eq_ignore_ascii_case("@StaticFile") {
        return "StaticFile";
    }
    if n.name.trim().starts_with('@') && n.name.trim().len() > 1 {
        return "CompilerTemplate";
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
    fn reachable(&self, start: &str) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut pending = vec![start.to_string()];
        while let Some(id) = pending.pop() {
            if seen.insert(id.clone())
                && let Some(next) = self.out.get(&id)
            {
                pending.extend(next.iter().cloned());
            }
        }
        seen
    }
    fn weak_components(&self) -> Vec<Vec<String>> {
        let mut seen = BTreeSet::new();
        let mut result = Vec::new();
        for root in self.out.keys() {
            if seen.contains(root) {
                continue;
            }
            let mut pending = vec![root.clone()];
            let mut component = Vec::new();
            while let Some(id) = pending.pop() {
                if seen.insert(id.clone()) {
                    component.push(id.clone());
                    pending.extend(
                        self.out[&id]
                            .iter()
                            .chain(self.incoming[&id].iter())
                            .cloned(),
                    );
                }
            }
            component.sort();
            result.push(component);
        }
        result.sort_by_key(|c| (std::cmp::Reverse(c.len()), c.first().cloned()));
        result
    }
    fn pairs(&self) -> BTreeSet<(String, String)> {
        self.out
            .iter()
            .flat_map(|(a, bs)| bs.iter().map(move |b| (a.clone(), b.clone())))
            .collect()
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
    ids.sort_by_cached_key(|id| (d.qualified_name(id), id.clone()));
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
    let workflows = principal.weak_components();
    let error_sinks = select(&error, "sink");
    let overview = indicators["overview"].as_array_mut().unwrap();
    overview.extend([json!({"label":"Principal workflows","value":workflows.len()}),json!({"label":"Principal cycles","value":principal.cycles().len()}),json!({"label":"Error cycles","value":error.cycles().len()}),json!({"label":"Dependency cycles","value":dependency.cycles().len()}),json!({"label":"Principal self-links","value":principal.out.iter().filter(|(id,targets)|targets.contains(*id)).count()}),json!({"label":"Structural density (%)","value":if ids.is_empty(){0.}else{100.*structural.pairs().len()as f64/(ids.len()*ids.len())as f64}})]);
    // Reachability can be quadratic in graph size. Keep the overview bounded on huge projects;
    // independent degree/SCC/component findings still cover the complete graph.
    let reachability_complete = ids.len() <= 4096;
    let mut max_reach = 0;
    let mut most_reachable = Vec::new();
    let mut deep = Vec::new();
    if reachability_complete {
        for id in &ids {
            let size = principal.reachable(id).len();
            if size > max_reach {
                max_reach = size;
                most_reachable = vec![id.clone()];
            }
            if dependency.reachable(id).len() > 6 {
                deep.push(id.clone());
            }
        }
    }
    indicators["principal"].as_array_mut().unwrap().push(json!({"label":"Largest downstream reachability","value":if reachability_complete{json!(max_reach)}else{json!("not computed above 4096 nodes")},"affectedNodes":most_reachable}));
    indicators["dependency"].as_array_mut().unwrap().push(json!({"label":"Deep dependency reachability (>6)","value":if reachability_complete{json!(deep.len())}else{json!("not computed above 4096 nodes")},"affectedNodes":deep}));
    let mut workflow_metrics = vec![
        json!({"label":"Principal components","value":workflows.len()}),
        json!({"label":"Largest workflow size","value":workflows.first().map(Vec::len).unwrap_or(0)}),
    ];
    let mut membership = BTreeMap::new();
    for (index, workflow) in workflows.iter().enumerate() {
        for id in workflow {
            membership.insert(id.clone(), index);
        }
        workflow_metrics.push(json!({"label":format!("Workflow {}",index+1),"value":workflow.len(),"affectedNodes":workflow}));
        if workflow.len() > 1
            && (!workflow
                .iter()
                .any(|id| principal.incoming[id].is_empty() && !principal.out[id].is_empty())
                || !workflow
                    .iter()
                    .any(|id| !principal.incoming[id].is_empty() && principal.out[id].is_empty()))
        {
            findings.push(json!({"severity":"NOTICE","category":"workflows","title":format!("Workflow {} has an incomplete boundary shape",index+1),"affectedNodes":workflow,"recommendation":"Inspect inputs, outputs and feedback at the workflow boundary."}));
        }
    }
    indicators.insert("workflows".into(), json!(workflow_metrics));
    let shared_pairs = principal
        .pairs()
        .intersection(&error.pairs())
        .cloned()
        .collect::<Vec<_>>();
    let error_only = error.pairs().difference(&principal.pairs()).count();
    let cross_error = error
        .pairs()
        .into_iter()
        .filter(|(a, b)| membership.get(a) != membership.get(b))
        .collect::<Vec<_>>();
    let cross_dependency = dependency
        .pairs()
        .into_iter()
        .filter(|(a, b)| {
            processing.contains(a)
                && processing.contains(b)
                && membership.get(a) != membership.get(b)
        })
        .collect::<Vec<_>>();
    indicators.insert("cross-layer".into(),json!([{"label":"Pairs carrying data and error","value":shared_pairs.len(),"affectedNodes":shared_pairs.iter().flat_map(|(a,b)|[a,b]).collect::<BTreeSet<_>>()},{"label":"Error-only directed pairs","value":error_only},{"label":"Cross-workflow error links","value":cross_error.len(),"affectedNodes":cross_error.iter().flat_map(|(a,b)|[a,b]).collect::<BTreeSet<_>>()},{"label":"Cross-workflow processing dependencies","value":cross_dependency.len(),"affectedNodes":cross_dependency.iter().flat_map(|(a,b)|[a,b]).collect::<BTreeSet<_>>()}]));
    for (title, pairs) in [
        ("Data and error share endpoint pairs", &shared_pairs),
        ("Error flow crosses workflow boundaries", &cross_error),
        ("Processing dependencies cross workflows", &cross_dependency),
    ] {
        if !pairs.is_empty() {
            findings.push(json!({"severity":"NOTICE","category":"cross-layer","title":title,"affectedNodes":pairs.iter().flat_map(|(a,b)|[a,b]).collect::<BTreeSet<_>>(),"recommendation":"Review ownership and the explicit normal/error/capability contract across this boundary."}));
        }
    }
    if reachability_complete {
        let no_sink = select(&error, "generator")
            .into_iter()
            .filter(|id| {
                error
                    .reachable(id)
                    .is_disjoint(&error_sinks.iter().cloned().collect())
            })
            .collect::<Vec<_>>();
        if !no_sink.is_empty() {
            findings.push(json!({"severity":"WARNING","category":"error","title":"Some error sources cannot reach an error sink","affectedNodes":no_sink,"recommendation":"Inspect termination, retry and ownership of the error path."}));
        }
    }
    let high_dependency = libraries
        .iter()
        .filter(|id| dependency.incoming[*id].len() >= 8)
        .cloned()
        .collect::<Vec<_>>();
    if !high_dependency.is_empty() {
        findings.push(json!({"severity":"WARNING","category":"quality","title":"High dependency fan-in","affectedNodes":high_dependency,"recommendation":"Review the ownership and stability of this shared library contract."}));
    }
    indicators.insert("quality".into(),json!([{"label":"High fan-out threshold","value":8},{"label":"High dependency fan-in threshold","value":8},{"label":"Deep dependency reachability threshold","value":6},{"label":"Reachability scope complete","value":reachability_complete}]));
    let mut actions = vec![
        json!({"title":"Select processing candidates","affectedNodes":processing}),
        json!({"title":"Select library candidates","affectedNodes":libraries}),
        json!({"title":"Select isolated entities","affectedNodes":isolated}),
        json!({"title":"Select all principal cyclic regions","affectedNodes":principal.cycles().into_iter().flatten().collect::<BTreeSet<_>>()}),
    ];
    actions.extend(workflows.iter().enumerate().filter(|(_,nodes)|nodes.len()>1).map(|(i,nodes)|json!({"title":format!("Select workflow {}",i+1),"affectedNodes":nodes})));
    json!({"nodeOrder":ids,"nodeLabels":ids.iter().map(|id|(id,d.qualified_name(id))).collect::<BTreeMap<_,_>>(),"indicators":indicators,"actions":actions,"workflows":workflows,"counts":d.network_counts(),"graphs":{"principal":principal.summary(),"error":error.summary(),"dependency":dependency.summary(),"structural":structural.summary(),"containment":containment.summary()},"isolatedNodes":isolated,"connectedComponents":components,"classifications":d.nodes.values().map(|n|json!({"nodeId":n.id,"stereotype":stereotype(d,n),"relation":relation(d,n)})).collect::<Vec<_>>(),"findings":findings,"limitations":["Automatic layout/model optimization was a future action in the source; inspection actions remain read-only.","Threshold defaults: fan-out 8, dependency fan-in 8, deep dependency reachability 6.","All-node reachability summaries are bounded to 4096 nodes; degree/SCC/component analysis covers the full graph."]})
}
