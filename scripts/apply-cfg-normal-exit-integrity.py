from pathlib import Path

path = Path("crates/nova-sema/src/control_flow.rs")
text = path.read_text()
old = '''                    FlowNodeKind::Transfer(FlowTransfer::Return) | FlowNodeKind::Exit => {
                        *edge != FlowEdgeKind::Diagnostic
                    }
                    FlowNodeKind::Transfer(FlowTransfer::Continue) => {
'''
new = '''                    FlowNodeKind::Transfer(FlowTransfer::Return) => {
                        *edge != FlowEdgeKind::Diagnostic
                    }
                    FlowNodeKind::Exit => true,
                    FlowNodeKind::Transfer(FlowTransfer::Continue) => {
'''
if text.count(old) != 1:
    raise SystemExit("expected Exit successor rule once")
text = text.replace(old, new)

old = '''    if graph.normal_exits.iter().any(|exit| {
        !matches!(
            graph.nodes.get(exit.index()).map(|node| &node.kind),
            Some(FlowNodeKind::Exit)
        )
    }) {
        return Err(FlowError::invalid(
            fallback_span,
            "normal exit does not reference an exit node",
        ));
    }
'''
new = '''    let actual_exits = graph
        .nodes
        .iter()
        .filter_map(|node| matches!(node.kind, FlowNodeKind::Exit).then_some(node.id))
        .collect::<BTreeSet<_>>();
    let declared_exits = graph.normal_exits.iter().copied().collect::<BTreeSet<_>>();
    if declared_exits.len() != graph.normal_exits.len() {
        return Err(FlowError::invalid(
            fallback_span,
            "normal exit table contains duplicate entries",
        ));
    }
    if declared_exits != actual_exits {
        return Err(FlowError::invalid(
            fallback_span,
            "normal exit table does not exactly match exit nodes",
        ));
    }
    if let Some(exit) = declared_exits
        .iter()
        .find(|exit| !execution_reached.contains(exit))
    {
        return Err(FlowError::invalid(
            graph.nodes[exit.index()].span.unwrap_or(fallback_span),
            format!("normal exit node {} is not executable-reachable", exit.index()),
        ));
    }
'''
if text.count(old) != 1:
    raise SystemExit("expected normal exit verification block once")
path.write_text(text.replace(old, new))
