from pathlib import Path

path = Path('crates/nova-sema/src/control_flow.rs')
text = path.read_text()

anchor = '''    for node in &graph.nodes {
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
'''
replacement = '''    for node in &graph.nodes {
        let mut has_backedge = false;
        for edge in &node.predecessors {
            if edge.kind != FlowEdgeKind::Backedge {
                continue;
            }
            has_backedge = true;
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
        if has_backedge
            && !node
                .predecessors
                .iter()
                .any(|edge| edge.kind == FlowEdgeKind::Execution && edge.from < node.id)
        {
            return Err(FlowError::invalid(
                node.span.unwrap_or(fallback_span),
                format!(
                    "loop-header Join node {} has no forward Execution predecessor",
                    node.id.index()
                ),
            ));
        }
    }
'''
if anchor not in text:
    raise SystemExit('backedge verifier anchor not found')
text = text.replace(anchor, replacement, 1)

test_anchor = '''    #[test]\n    fn verifier_rejects_execution_after_return() {\n'''
test = '''    #[test]
    fn verifier_rejects_loop_header_without_first_entry_predecessor() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 30));
        let value = binding(0, "value", 1);
        builder.register_binding(&value);
        let entry = builder.cursor();
        let header = builder.join([entry], Some(span(2, 3)), FlowEdgeKind::Execution);
        builder.advance(
            FlowNodeKind::Read(value.id),
            Some(span(4, 9)),
            FlowEdgeKind::Execution,
        );
        builder.advance(
            FlowNodeKind::Initialize(value.id),
            Some(span(10, 15)),
            FlowEdgeKind::Execution,
        );
        let loop_path = builder.cursor();

        let alternate = builder.fork_from(entry, Some(span(16, 17)), FlowEdgeKind::Execution);
        builder.advance(
            FlowNodeKind::Initialize(value.id),
            Some(span(18, 23)),
            FlowEdgeKind::Execution,
        );
        let alternate_initialized = builder.cursor();
        let tail = builder.join(
            [loop_path, alternate_initialized],
            Some(span(24, 25)),
            FlowEdgeKind::Execution,
        );
        builder.add_backedge(tail, header);

        let mut graph = builder.finish(None).expect("valid seed cyclic graph");
        let diagnostics = definite_initialization_diagnostics(&graph, span(0, 30))
            .expect("seed graph must verify");
        assert_eq!(diagnostics.len(), 1, "first loop entry must remain visible");

        graph.nodes[header.index()]
            .predecessors
            .retain(|edge| edge.kind == FlowEdgeKind::Backedge);

        let error = super::verify(&graph, span(0, 30))
            .expect_err("a loop header must retain its first-entry execution path");
        assert!(error.message().contains("loop-header"));
        assert!(error.message().contains("Execution predecessor"));
        assert_ne!(alternate, header);
    }

'''
if test_anchor not in text:
    raise SystemExit('test anchor not found')
text = text.replace(test_anchor, test + test_anchor, 1)
path.write_text(text)
