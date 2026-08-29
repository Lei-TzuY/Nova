use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    AnalysisOutput, analyze,
    control_flow::{FlowEdgeKind, FlowNodeKind, FlowTransfer},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "invalid-while-continue-flow.nv", text);
    let lexed = lex(&source);
    assert!(
        lexed.is_success(),
        "lex diagnostics: {:?}",
        lexed.diagnostics
    );
    let parsed = parse(&source, &lexed.tokens);
    assert!(
        parsed.is_success(),
        "parse diagnostics: {:?}",
        parsed.diagnostics
    );
    analyze(&parsed.program)
}

fn has_any_backedge(output: &AnalysisOutput) -> bool {
    output.control_flow.functions()[0]
        .nodes()
        .iter()
        .flat_map(|node| node.predecessors.iter())
        .any(|edge| edge.kind == FlowEdgeKind::Backedge)
}

fn has_backedge_from_continue(output: &AnalysisOutput) -> bool {
    let graph = &output.control_flow.functions()[0];
    let continue_node = graph
        .nodes()
        .iter()
        .find(|node| matches!(node.kind, FlowNodeKind::Transfer(FlowTransfer::Continue)))
        .expect("expected continue transfer");

    graph.nodes().iter().any(|node| {
        node.predecessors
            .iter()
            .any(|edge| edge.from == continue_node.id && edge.kind == FlowEdgeKind::Backedge)
    })
}

#[test]
fn invalid_while_condition_keeps_continue_on_discarded_recovery_path() {
    let output = analyze_text("fn f() -> Int { while 0 { continue; } 1 }");
    let codes = output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();

    assert!(codes.contains(&"N3004"), "{codes:?}");
    assert!(!codes.contains(&"N3013"), "{codes:?}");
    assert!(!codes.contains(&"N3999"), "{codes:?}");
    assert!(
        !has_backedge_from_continue(&output),
        "continue in an invalid while body must not reconnect a discarded recovery path to the loop header"
    );
}

#[test]
fn invalid_while_fallthrough_does_not_form_a_recovery_backedge() {
    let output = analyze_text("fn f() -> Int { while 0 { 1; } 1 }");
    let codes = output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();

    assert!(codes.contains(&"N3004"), "{codes:?}");
    assert!(!codes.contains(&"N3999"), "{codes:?}");
    assert!(
        !has_any_backedge(&output),
        "fallthrough from an invalid while body must stay on the discarded recovery path"
    );
}

#[test]
fn valid_dynamic_while_continue_still_has_a_backedge() {
    let output = analyze_text("fn f(flag: Bool) -> Int { while flag { continue; } 1 }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(has_backedge_from_continue(&output));
    assert!(has_any_backedge(&output));
}
