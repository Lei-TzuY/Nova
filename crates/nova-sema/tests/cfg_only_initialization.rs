use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::control_flow::FlowNodeKind;
use nova_sema::hir::{ExpressionKind, StatementKind, Type};
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "test.nv", text);
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

fn codes(output: &AnalysisOutput) -> Vec<&str> {
    output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

#[test]
fn uninitialized_read_keeps_its_declared_hir_type() {
    let output = analyze_text("fn main() -> Int { var value: Int; value }");

    assert_eq!(codes(&output), vec!["N3009"]);
    let tail = output.program.functions[0]
        .body
        .tail
        .as_deref()
        .expect("tail expression");
    assert!(matches!(tail.kind, ExpressionKind::Binding(_)));
    assert_eq!(tail.ty, Type::Int);
}

#[test]
fn type_mismatch_and_definite_initialization_are_orthogonal() {
    let output = analyze_text("fn main() -> Bool { var value: Int; value }");

    assert_eq!(codes(&output), vec!["N3004", "N3009"]);
    let tail = output.program.functions[0]
        .body
        .tail
        .as_deref()
        .expect("tail expression");
    assert_eq!(tail.ty, Type::Int);
}

#[test]
fn noncontinuing_initializer_does_not_create_an_initialization_event() {
    let output = analyze_text("fn main() -> Int { let value: Int = { return 7; }; value }");

    assert_eq!(codes(&output), vec!["N3009"]);
    assert_eq!(output.control_flow.functions().len(), 1);

    let StatementKind::Binding {
        binding,
        initializer,
    } = &output.program.functions[0].body.statements[0].kind
    else {
        panic!("expected binding statement");
    };
    assert_eq!(binding.ty, Type::Int);
    assert!(initializer.ty.is_never());

    let graph = &output.control_flow.functions()[0];
    assert!(
        graph
            .nodes()
            .iter()
            .all(|node| { !matches!(node.kind, FlowNodeKind::Initialize(id) if id == binding.id) }),
        "a binding whose initializer cannot complete must not become initialized"
    );
}
