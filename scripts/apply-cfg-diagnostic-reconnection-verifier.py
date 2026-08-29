from pathlib import Path

path = Path("crates/nova-sema/src/control_flow.rs")
text = path.read_text()
anchor = '''    let known_bindings = graph
        .bindings
        .iter()
        .map(|binding| binding.id)
        .collect::<BTreeSet<_>>();
'''
check = '''    let mut execution_reached = BTreeSet::new();
    let mut execution_queue = VecDeque::from([graph.entry]);
    while let Some(node) = execution_queue.pop_front() {
        if !execution_reached.insert(node) {
            continue;
        }
        if let Some(next) = successors.get(node.index()) {
            execution_queue.extend(
                next.iter()
                    .filter(|(_, edge)| *edge != FlowEdgeKind::Diagnostic)
                    .map(|(successor, _)| *successor),
            );
        }
    }
    for node in &graph.nodes {
        if !execution_reached.contains(&node.id) {
            continue;
        }
        if let Some(edge) = node.predecessors.iter().find(|edge| {
            edge.kind == FlowEdgeKind::Diagnostic || !execution_reached.contains(&edge.from)
        }) {
            return Err(FlowError::invalid(
                node.span.unwrap_or(fallback_span),
                format!(
                    "diagnostic-only control flow from node {} reconnects to executable node {}",
                    edge.from.index(),
                    node.id.index()
                ),
            ));
        }
    }

'''
if text.count(anchor) != 1:
    raise SystemExit("expected verifier binding anchor once")
path.write_text(text.replace(anchor, check + anchor))
