from pathlib import Path

path = Path('crates/nova-sema/src/control_flow.rs')
text = path.read_text()

old = '''        } else if node.predecessors.is_empty() {
            return Err(FlowError::invalid(
                node.span.unwrap_or(fallback_span),
                format!("flow node {index} has no predecessor"),
            ));
        }
        for edge in &node.predecessors {
            let Some(outgoing) = successors.get_mut(edge.from.index()) else {
'''
new = '''        } else if node.predecessors.is_empty() {
            return Err(FlowError::invalid(
                node.span.unwrap_or(fallback_span),
                format!("flow node {index} has no predecessor"),
            ));
        }
        for (edge_index, edge) in node.predecessors.iter().enumerate() {
            if node.predecessors[..edge_index].contains(edge) {
                return Err(FlowError::invalid(
                    node.span.unwrap_or(fallback_span),
                    format!(
                        "flow node {index} contains a duplicate {:?} predecessor from node {}",
                        edge.kind,
                        edge.from.index()
                    ),
                ));
            }
            let Some(outgoing) = successors.get_mut(edge.from.index()) else {
'''
if old not in text:
    raise SystemExit('verifier anchor not found')
text = text.replace(old, new, 1)

cardinality_anchor = '''    for bindings in graph.bindings.windows(2) {\n'''
cardinality = '''    for node in &graph.nodes {
        if node.id != graph.entry
            && !matches!(node.kind, FlowNodeKind::Join)
            && node.predecessors.len() != 1
        {
            return Err(FlowError::invalid(
                node.span.unwrap_or(fallback_span),
                format!(
                    "non-Join flow node {} has {} predecessors; expected exactly one",
                    node.id.index(),
                    node.predecessors.len()
                ),
            ));
        }
    }

'''
if cardinality_anchor not in text:
    raise SystemExit('cardinality anchor not found')
text = text.replace(cardinality_anchor, cardinality + cardinality_anchor, 1)

anchor = '''    #[test]\n    fn builder_fails_closed_on_an_invalid_backedge_endpoint() {\n'''
tests = '''    #[test]
    fn verifier_rejects_multiple_predecessors_on_non_join_nodes() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let value = binding(0, "value", 1);
        builder.register_binding(&value);
        builder.advance(
            FlowNodeKind::Initialize(value.id),
            Some(value.span),
            FlowEdgeKind::Execution,
        );
        builder.advance(
            FlowNodeKind::Read(value.id),
            Some(span(8, 13)),
            FlowEdgeKind::Execution,
        );
        let read = builder.cursor();
        let exit = builder.cursor();
        let mut graph = builder.finish(Some(exit)).expect("valid seed graph");
        let diagnostics = definite_initialization_diagnostics(&graph, span(0, 20))
            .expect("seed graph must verify");
        assert!(diagnostics.is_empty());

        graph.nodes[read.index()].predecessors.push(super::FlowEdge {
            from: graph.entry,
            kind: FlowEdgeKind::Execution,
        });

        let error = super::verify(&graph, span(0, 20))
            .expect_err("only Join nodes may merge multiple predecessor paths");
        assert!(error.message().contains("non-Join"));
    }

    #[test]
    fn verifier_rejects_duplicate_join_predecessor_edges() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let entry = builder.cursor();
        let left = builder.fork_from(entry, Some(span(1, 2)), FlowEdgeKind::Execution);
        let right = builder.fork_from(entry, Some(span(3, 4)), FlowEdgeKind::Execution);
        let join = builder.join([left, right], Some(span(5, 6)), FlowEdgeKind::Execution);
        let exit = builder.cursor();
        let mut graph = builder.finish(Some(exit)).expect("valid seed graph");
        let duplicate = graph.nodes[join.index()].predecessors[0];
        graph.nodes[join.index()].predecessors.push(duplicate);

        let error = super::verify(&graph, span(0, 20))
            .expect_err("verified predecessor lists must be duplicate-free");
        assert!(error.message().contains("duplicate"));
    }

'''
if anchor not in text:
    raise SystemExit('test anchor not found')
text = text.replace(anchor, tests + anchor, 1)
path.write_text(text)
