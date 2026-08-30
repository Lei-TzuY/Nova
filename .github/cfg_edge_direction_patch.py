from pathlib import Path

path = Path('crates/nova-sema/src/control_flow.rs')
text = path.read_text()

old = '''        for edge in &node.predecessors {
            let Some(outgoing) = successors.get_mut(edge.from.index()) else {
                return Err(FlowError::invalid(
                    node.span.unwrap_or(fallback_span),
                    format!("flow node {index} has an out-of-range predecessor"),
                ));
            };
            outgoing.push((node.id, edge.kind));
        }
'''
new = '''        for edge in &node.predecessors {
            let Some(outgoing) = successors.get_mut(edge.from.index()) else {
                return Err(FlowError::invalid(
                    node.span.unwrap_or(fallback_span),
                    format!("flow node {index} has an out-of-range predecessor"),
                ));
            };
            let source = edge.from.index();
            match edge.kind {
                FlowEdgeKind::Backedge if source <= index => {
                    return Err(FlowError::invalid(
                        node.span.unwrap_or(fallback_span),
                        format!(
                            "backedge from node {source} to node {index} is not strictly backward"
                        ),
                    ));
                }
                FlowEdgeKind::Execution | FlowEdgeKind::Diagnostic if source >= index => {
                    return Err(FlowError::invalid(
                        node.span.unwrap_or(fallback_span),
                        format!(
                            "forward {:?} edge from node {source} to node {index} is not strictly forward",
                            edge.kind
                        ),
                    ));
                }
                _ => {}
            }
            outgoing.push((node.id, edge.kind));
        }
'''
if old not in text:
    raise SystemExit('verify predecessor anchor not found')
text = text.replace(old, new, 1)

anchor = '''    #[test]
    fn builder_fails_closed_on_an_invalid_backedge_endpoint() {
'''
tests = '''    #[test]
    fn verifier_rejects_forward_edge_marked_as_backedge() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let entry = builder.cursor();
        let left = builder.fork_from(entry, Some(span(1, 2)), FlowEdgeKind::Execution);
        let right = builder.fork_from(entry, Some(span(3, 4)), FlowEdgeKind::Execution);
        let join = builder.join([left, right], Some(span(5, 6)), FlowEdgeKind::Execution);
        let graph_exit = builder.cursor();
        let mut graph = builder.finish(Some(graph_exit)).expect("valid seed graph");
        graph.nodes[join.index()].predecessors[0].kind = FlowEdgeKind::Backedge;

        let error = super::verify(&graph, span(0, 20))
            .expect_err("a backedge must point to an earlier loop header");
        assert!(error.message().contains("strictly backward"));
    }

    #[test]
    fn verifier_rejects_backward_execution_edge() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let entry = builder.cursor();
        let header = builder.join([entry], Some(span(1, 2)), FlowEdgeKind::Execution);
        builder.advance(
            FlowNodeKind::Branch,
            Some(span(3, 4)),
            FlowEdgeKind::Execution,
        );
        let body = builder.cursor();
        builder.add_backedge(body, header);
        let mut graph = builder.finish(None).expect("valid seed graph");
        let backedge = graph.nodes[header.index()]
            .predecessors
            .iter_mut()
            .find(|edge| edge.kind == FlowEdgeKind::Backedge)
            .expect("loop backedge");
        backedge.kind = FlowEdgeKind::Execution;

        let error = super::verify(&graph, span(0, 20))
            .expect_err("an ordinary execution edge cannot encode a backward cycle");
        assert!(error.message().contains("strictly forward"));
    }

'''
if anchor not in text:
    raise SystemExit('test insertion anchor not found')
text = text.replace(anchor, tests + anchor, 1)
path.write_text(text)
