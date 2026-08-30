use nova_interpreter::execute;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "record-field-identity.nv", text);
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
    let analyzed = analyze(&parsed.program);
    assert!(
        analyzed.is_success(),
        "semantic diagnostics: {:?}",
        analyzed.diagnostics
    );
    analyzed
}

const SOURCE: &str = "record Pair { left: Int, right: Int }\n\
                      fn main() -> Int { let pair = new Pair { left: 1, right: 2 }; pair.left }";

#[test]
fn malformed_same_typed_constructor_slot_retargeting_fails_closed() {
    let mut analyzed = analyze_text(SOURCE);
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Binding { initializer, .. } = &mut main.body.statements[0].kind else {
        panic!("expected record binding");
    };
    let ExpressionKind::RecordLiteral { fields, .. } = &mut initializer.kind else {
        panic!("expected record literal");
    };
    fields[0].field_index = 1;
    fields[1].field_index = 0;

    let error = execute(&analyzed.program)
        .expect_err("same-typed constructor retargeting must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn malformed_same_typed_projection_slot_retargeting_fails_closed() {
    let mut analyzed = analyze_text(SOURCE);
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let expression = main.body.tail.as_deref_mut().expect("main tail");
    let ExpressionKind::FieldAccess { field_index, .. } = &mut expression.kind else {
        panic!("expected field access");
    };
    *field_index = 1;

    let error =
        execute(&analyzed.program).expect_err("same-typed projection retargeting must fail closed");
    assert_eq!(error.code, "N4005");
}
