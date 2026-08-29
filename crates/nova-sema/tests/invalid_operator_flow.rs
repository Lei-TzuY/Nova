use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{StatementKind, Type};
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "invalid-operator-flow.nv", text);
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

fn expression_statement_type<'a>(
    output: &'a AnalysisOutput,
    function_name: &str,
    index: usize,
) -> &'a Type {
    let StatementKind::Expression(expression) =
        &function(output, function_name).body.statements[index].kind
    else {
        panic!("expected expression statement");
    };
    &expression.ty
}

#[test]
fn unary_mismatch_is_error_and_does_not_initialize_outer_binding() {
    let output = analyze_text("fn f() -> Int { var x: Int; !{ x = 1; 0 }; x }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(actual.contains(&"N3009"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 1), &Type::Error);
}

#[test]
fn arithmetic_mismatch_is_error_and_does_not_initialize_outer_binding() {
    let output = analyze_text("fn f() -> Int { var x: Int; ({ x = 1; true }) + 1; x }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(actual.contains(&"N3009"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 1), &Type::Error);
}

#[test]
fn comparison_mismatch_is_error_and_does_not_initialize_outer_binding() {
    let output = analyze_text("fn f() -> Int { var x: Int; ({ x = 1; true }) < 1; x }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(actual.contains(&"N3009"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 1), &Type::Error);
}

#[test]
fn boolean_mismatch_is_error_and_does_not_initialize_outer_binding() {
    let output = analyze_text("fn f() -> Int { var x: Int; ({ x = 1; 1 }) && true; x }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(actual.contains(&"N3009"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 1), &Type::Error);
}

#[test]
fn equality_error_does_not_export_operand_initialization() {
    let output = analyze_text("fn f() -> Int { var x: Int; ({ x = 1; 1 }) == true; x }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(actual.contains(&"N3009"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 1), &Type::Error);
}

#[test]
fn rejected_binary_discards_conditional_break_exits() {
    let output = analyze_text(
        "fn f(flag: Bool) -> Int { while true { (if flag { break; } else { 1 }) && true; } }",
    );
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
}

#[test]
fn strict_binary_noncontinuation_keeps_never_precedence() {
    let output = analyze_text("fn f() -> Int { true + { return 1; 0 }; }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 0), &Type::Never);
    assert!(function(&output, "f").body.ty.is_never());
}

#[test]
fn unary_noncontinuation_keeps_never_precedence() {
    let output = analyze_text("fn f() -> Int { !{ return 1; false }; }");
    let actual = codes(&output);
    assert!(!actual.contains(&"N3004"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 0), &Type::Never);
    assert!(function(&output, "f").body.ty.is_never());
}

#[test]
fn forced_short_circuit_rhs_noncontinuation_keeps_never() {
    let output = analyze_text("fn f() -> Int { true && { return 1; false }; }");
    let actual = codes(&output);
    assert!(!actual.contains(&"N3004"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 0), &Type::Never);
}

#[test]
fn optional_short_circuit_rhs_noncontinuation_does_not_force_divergence() {
    let output = analyze_text("fn f(flag: Bool) -> Bool { flag && { return true; false } }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(codes(&output), vec!["N3033"]);
    assert_eq!(function(&output, "f").body.ty, Type::Bool);
}
