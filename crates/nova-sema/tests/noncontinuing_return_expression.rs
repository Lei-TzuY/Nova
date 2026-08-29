use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::control_flow::{FlowNodeKind, FlowTransfer};
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "test.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "lex diagnostics: {:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(
        parsed.is_success(),
        "parse diagnostics: {:?}",
        parsed.diagnostics
    );
    analyze(&parsed.program)
}

fn codes(output: &AnalysisOutput) -> Vec<&str> {
    output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

fn transfer_count(output: &AnalysisOutput, transfer: FlowTransfer) -> usize {
    output.control_flow.functions()[0]
        .nodes()
        .iter()
        .filter(|node| matches!(node.kind, FlowNodeKind::Transfer(actual) if actual == transfer))
        .count()
}

#[test]
fn nested_return_expression_does_not_append_a_second_return_transfer() {
    let output = analyze_text("fn main() -> Int { return { return 7; }; }");

    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(transfer_count(&output, FlowTransfer::Return), 1);
}

#[test]
fn break_from_return_expression_consumes_the_parent_return() {
    let output = analyze_text(
        "fn main() -> Int { while true { return { break; 0 }; } 42 }",
    );

    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(transfer_count(&output, FlowTransfer::Break), 1);
    assert_eq!(transfer_count(&output, FlowTransfer::Return), 0);
}

#[test]
fn ordinary_return_still_emits_exactly_one_return_transfer() {
    let output = analyze_text("fn main() -> Int { return 42; }");

    assert_eq!(codes(&output), Vec::<&str>::new());
    assert_eq!(transfer_count(&output, FlowTransfer::Return), 1);
}
