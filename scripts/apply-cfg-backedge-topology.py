from pathlib import Path

path = Path("crates/nova-sema/src/control_flow.rs")
text = path.read_text()
needle = '''    for bindings in graph.bindings.windows(2) {
'''
insert = '''    for node in &graph.nodes {
        for edge in &node.predecessors {
            if edge.kind != FlowEdgeKind::Backedge {
                continue;
            }
            if !matches!(node.kind, FlowNodeKind::Join) {
                return Err(FlowError::invalid(
                    node.span.unwrap_or(fallback_span),
                    format!(
                        "backedge from node {} targets non-Join node {}",
                        edge.from.index(),
                        node.id.index()
                    ),
                ));
            }
            if !execution_reached.contains(&node.id) || !execution_reached.contains(&edge.from) {
                return Err(FlowError::invalid(
                    node.span.unwrap_or(fallback_span),
                    format!(
                        "backedge from node {} to node {} is not confined to executable control flow",
                        edge.from.index(),
                        node.id.index()
                    ),
                ));
            }
        }
    }

    for bindings in graph.bindings.windows(2) {
'''
if text.count(needle) != 1:
    raise SystemExit("expected one binding verifier anchor")
path.write_text(text.replace(needle, insert))
