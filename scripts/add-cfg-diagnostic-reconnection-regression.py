from pathlib import Path

path = Path("crates/nova-sema/src/control_flow.rs")
text = path.read_text()
anchor = '''    #[test]
    fn builder_fails_closed_on_an_invalid_backedge_endpoint() {
'''
test = '''    #[test]
    fn verifier_rejects_diagnostic_only_reconnection() {
        let mut builder = FunctionFlowBuilder::new(FunctionId::new(0), span(0, 20));
        let entry = builder.cursor();
        let header = builder.join([entry], None, FlowEdgeKind::Execution);
        let recovery = builder.fork_from(
            header,
            Some(span(1, 2)),
            FlowEdgeKind::Diagnostic,
        );
        builder.add_backedge(recovery, header);

        let error = builder
            .finish(None)
            .expect_err("diagnostic-only recovery must not reconnect to executable flow");
        assert!(error.message().contains("diagnostic-only"));
    }

'''
if text.count(anchor) != 1:
    raise SystemExit("expected verifier-test insertion anchor once")
path.write_text(text.replace(anchor, test + anchor))
