use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{StatementKind, Type};
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "invalid-aggregate-flow.nv", text);
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

fn assert_has_codes(output: &AnalysisOutput, expected: &[&str]) {
    let actual = codes(output);
    for code in expected {
        assert!(actual.contains(code), "missing {code}: {actual:?}");
    }
}

#[test]
fn invalid_record_field_type_does_not_initialize_outer_binding() {
    let output = analyze_text(
        "record Box { value: Bool } fn f() -> Int { var x: Int; new Box { value: { x = 1; 0 } }; x }",
    );
    assert_has_codes(&output, &["N3004", "N3009"]);
    let StatementKind::Expression(expression) =
        &output.program.functions[0].body.statements[1].kind
    else {
        panic!("expected record expression statement");
    };
    assert_eq!(expression.ty, Type::Error);
}

#[test]
fn structurally_invalid_record_does_not_initialize_outer_binding() {
    let output = analyze_text(
        "record Box { value: Int } fn f() -> Int { var x: Int; new Box { missing: { x = 1; 0 } }; x }",
    );
    assert_has_codes(&output, &["N3011", "N3012", "N3009"]);
}

#[test]
fn invalid_enum_payload_type_does_not_initialize_outer_binding() {
    let output = analyze_text(
        "enum Choice { Value(Bool) } fn f() -> Int { var x: Int; Choice::Value({ x = 1; 0 }); x }",
    );
    assert_has_codes(&output, &["N3004", "N3009"]);
    let StatementKind::Expression(expression) =
        &output.program.functions[0].body.statements[1].kind
    else {
        panic!("expected enum constructor expression statement");
    };
    assert_eq!(expression.ty, Type::Error);
}

#[test]
fn invalid_enum_arity_does_not_initialize_outer_binding() {
    let output = analyze_text(
        "enum Choice { Empty } fn f() -> Int { var x: Int; Choice::Empty({ x = 1; 0 }); x }",
    );
    assert_has_codes(&output, &["N3022", "N3009"]);
}

#[test]
fn invalid_aggregate_rollback_discards_conditional_break_exits() {
    for text in [
        "record Box { value: Bool } fn f(flag: Bool) -> Int { while true { new Box { value: if flag { break; } else { 0 } }; } }",
        "enum Choice { Value(Bool) } fn f(flag: Bool) -> Int { while true { Choice::Value(if flag { break; } else { 0 }); } }",
    ] {
        let output = analyze_text(text);
        let actual = codes(&output);
        assert!(actual.contains(&"N3004"), "{text}: {actual:?}");
        assert!(!actual.contains(&"N3007"), "{text}: {actual:?}");
    }
}

#[test]
fn noncontinuing_valid_aggregate_inputs_keep_their_never_flow() {
    for text in [
        "record Box { value: Bool } fn f() -> Int { new Box { value: { return 1; false } }; }",
        "enum Choice { Value(Bool) } fn f() -> Int { Choice::Value({ return 1; false }); }",
    ] {
        let output = analyze_text(text);
        assert!(output.is_success(), "{text}: {:?}", output.diagnostics);
        assert!(output.program.functions[0].body.ty.is_never(), "{text}");
    }
}
