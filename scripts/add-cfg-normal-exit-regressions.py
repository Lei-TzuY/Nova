from pathlib import Path

path = Path("crates/nova-sema/src/control_flow.rs")
text = path.read_text()
anchor = '''    #[test]
    fn builder_fails_closed_on_an_invalid_backedge_endpoint() {
'''
tests = '''    #[test]
    fn verifier_rejects_diagnostic_only_normal_exit() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let entry = builder.cursor();
        let recovery = builder.fork_from(entry, Some(span(1, 2)), FlowEdgeKind::Diagnostic);

        let error = builder
            .finish(Some(recovery))
            .expect_err("a normal exit must be reachable without crossing diagnostic flow");
        assert!(error.message().contains("normal exit"));
    }

    #[test]
    fn verifier_rejects_unlisted_exit_node() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        builder.advance(
            FlowNodeKind::Branch,
            Some(span(1, 2)),
            FlowEdgeKind::Execution,
        );
        let graph_exit = builder.cursor();
        let mut graph = builder.finish(Some(graph_exit)).expect("valid seed graph");
        graph.normal_exits.clear();

        let error = super::verify(&graph, span(0, 20))
            .expect_err("every Exit node must appear in the normal-exit table");
        assert!(error.message().contains("normal exit"));
    }

    #[test]
    fn verifier_rejects_successor_after_exit() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        builder.advance(
            FlowNodeKind::Branch,
            Some(span(1, 2)),
            FlowEdgeKind::Execution,
        );
        let graph_exit = builder.cursor();
        let mut graph = builder.finish(Some(graph_exit)).expect("valid seed graph");
        let exit = graph.normal_exits[0];
        let successor = super::FlowNodeId(graph.nodes.len());
        graph.nodes.push(super::FlowNode {
            id: successor,
            kind: FlowNodeKind::Branch,
            predecessors: vec![super::FlowEdge {
                from: exit,
                kind: FlowEdgeKind::Diagnostic,
            }],
            span: Some(span(3, 4)),
        });

        let error = super::verify(&graph, span(0, 20))
            .expect_err("a function Exit must be terminal even for diagnostic source");
        assert!(error.message().contains("successor"));
    }

'''
if text.count(anchor) != 1:
    raise SystemExit("expected normal-exit test insertion anchor once")
path.write_text(text.replace(anchor, tests + anchor))
