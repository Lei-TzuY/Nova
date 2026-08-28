use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{StatementKind, Type};
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "invalid-field-access-flow.nv", text);
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

fn function<'a>(output: &'a AnalysisOutput, name: &str) -> &'a nova_sema::hir::Function {
    output
        .program
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing function {name}"))
}

fn expression_statement_type(output: &AnalysisOutput, function_name: &str, index: usize) -> Type {
    let StatementKind::Expression(expression) =
        &function(output, function_name).body.statements[index].kind
    else {
        panic!("expected expression statement");
    };
    expression.ty.clone()
}

#[test]
fn nonrecord_field_access_does_not_initialize_outer_binding() {
    let output = analyze_text(
        "fn identity(value: Int) -> Int { value } fn f() -> Int { var x: Int; identity({ x = 1; 0 }).value; x }",
    );
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(actual.contains(&"N3009"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 1), Type::Error);
}

#[test]
fn unknown_field_does_not_initialize_outer_binding() {
    let output = analyze_text(
        "record Box { value: Int } fn f() -> Int { var x: Int; new Box { value: { x = 1; 0 } }.missing; x }",
    );
    let actual = codes(&output);
    assert!(actual.contains(&"N3011"), "{actual:?}");
    assert!(actual.contains(&"N3009"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 1), Type::Error);
}

#[test]
fn rejected_field_access_discards_conditional_break_exits() {
    let output = analyze_text(
        "fn identity(value: Int) -> Int { value } fn f(flag: Bool) -> Int { while true { identity(if flag { break; } else { 0 }).value; } }",
    );
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
}

#[test]
fn noncontinuing_base_keeps_never_flow_without_field_type_cascade() {
    let output = analyze_text(
        "fn identity(value: Int) -> Int { value } fn f() -> Int { identity({ return 1; 0 }).value; }",
    );
    let actual = codes(&output);
    assert!(!actual.contains(&"N3004"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 0), Type::Never);
    assert!(function(&output, "f").body.ty.is_never());
}
