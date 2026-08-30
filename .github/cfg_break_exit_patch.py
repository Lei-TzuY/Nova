from pathlib import Path

path = Path('crates/nova-sema/src/control_flow.rs')
text = path.read_text()

old = '''        let invalid_successor =
            successors[node.id.index()]
                .iter()
                .any(|(_, edge)| match &node.kind {
                    FlowNodeKind::Transfer(FlowTransfer::Return) => {
                        *edge != FlowEdgeKind::Diagnostic
                    }
                    FlowNodeKind::Exit => true,
                    FlowNodeKind::Transfer(FlowTransfer::Continue) => {
                        !matches!(edge, FlowEdgeKind::Diagnostic | FlowEdgeKind::Backedge)
                    }
                    FlowNodeKind::Transfer(FlowTransfer::Break) => *edge == FlowEdgeKind::Backedge,
                    _ => false,
                });
'''
new = '''        let invalid_successor =
            successors[node.id.index()]
                .iter()
                .any(|(successor, edge)| match &node.kind {
                    FlowNodeKind::Transfer(FlowTransfer::Return) => {
                        *edge != FlowEdgeKind::Diagnostic
                    }
                    FlowNodeKind::Exit => true,
                    FlowNodeKind::Transfer(FlowTransfer::Continue) => {
                        !matches!(edge, FlowEdgeKind::Diagnostic | FlowEdgeKind::Backedge)
                    }
                    FlowNodeKind::Transfer(FlowTransfer::Break) => match edge {
                        FlowEdgeKind::Backedge => true,
                        FlowEdgeKind::Diagnostic => false,
                        FlowEdgeKind::Execution => !matches!(
                            graph.nodes[successor.index()].kind,
                            FlowNodeKind::Join
                        ),
                    },
                    _ => false,
                });
'''
if old not in text:
    raise SystemExit('transfer successor verifier anchor not found')
text = text.replace(old, new, 1)

anchor = '''    #[test]\n    fn verifier_rejects_execution_after_return() {\n'''
test = '''    #[test]
    fn verifier_rejects_break_execution_that_bypasses_a_join() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let transfer = builder.advance(
            FlowNodeKind::Transfer(FlowTransfer::Break),
            Some(span(1, 7)),
            FlowEdgeKind::Execution,
        );
        let join = builder.join([transfer], Some(span(8, 9)), FlowEdgeKind::Execution);
        let mut graph = builder.finish(None).expect("valid break-to-join seed graph");

        graph.nodes[join.index()].kind = FlowNodeKind::Branch;

        let error = super::verify(&graph, span(0, 20))
            .expect_err("break execution must re-enter continuation through a Join");
        assert!(error.message().contains("incompatible"));
    }

'''
if anchor not in text:
    raise SystemExit('return successor test anchor not found')
text = text.replace(anchor, test + anchor, 1)
path.write_text(text)
