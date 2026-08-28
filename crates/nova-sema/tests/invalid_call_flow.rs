use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{ExpressionKind, StatementKind, Type};
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "invalid-call-flow.nv", text);
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
fn wrong_arity_call_does_not_initialize_outer_binding() {
    let output = analyze_text(
        "fn target() -> Int { 0 } fn f() -> Int { var x: Int; target({ x = 1; 0 }); x }",
    );
    assert_has_codes(&output, &["N3006", "N3009"]);
    assert_eq!(expression_statement_type(&output, "f", 1), &Type::Error);
}

#[test]
fn argument_type_mismatch_does_not_initialize_outer_binding() {
    let output = analyze_text(
        "fn target(value: Bool) -> Int { 0 } fn f() -> Int { var x: Int; target({ x = 1; 0 }); x }",
    );
    assert_has_codes(&output, &["N3004", "N3009"]);
    assert_eq!(expression_statement_type(&output, "f", 1), &Type::Error);
}

#[test]
fn noncallable_callee_does_not_export_argument_initialization() {
    let output =
        analyze_text("fn f() -> Int { var x: Int; let callee = 0; callee({ x = 1; 0 }); x }");
    assert_has_codes(&output, &["N3005", "N3009"]);
    assert_eq!(expression_statement_type(&output, "f", 2), &Type::Error);
}

#[test]
fn erroneous_argument_does_not_export_earlier_argument_flow() {
    let output = analyze_text(
        "fn target(value: Bool) -> Int { 0 } fn f() -> Int { var x: Int; target({ x = 1; missing }); x }",
    );
    assert_has_codes(&output, &["N3003", "N3009"]);
    assert_eq!(expression_statement_type(&output, "f", 1), &Type::Error);
}

#[test]
fn rejected_call_discards_conditional_break_exits() {
    let output = analyze_text(
        "fn target() -> Int { 0 } fn f(flag: Bool) -> Int { while true { target(if flag { break; } else { 0 }); } }",
    );
    let actual = codes(&output);
    assert!(actual.contains(&"N3006"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
}

#[test]
fn noncontinuing_wrong_arity_argument_keeps_never_flow() {
    let output =
        analyze_text("fn target() -> Int { 0 } fn f() -> Int { target({ return 1; 0 }); }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3006"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
    assert!(function(&output, "f").body.ty.is_never());
    assert_eq!(expression_statement_type(&output, "f", 0), &Type::Never);
}

#[test]
fn noncontinuing_argument_dominates_noncallable_callee_recovery() {
    let output = analyze_text("fn f() -> Int { let callee = 0; callee({ return 1; 0 }); }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3005"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
    assert!(function(&output, "f").body.ty.is_never());
    assert_eq!(expression_statement_type(&output, "f", 1), &Type::Never);
}

#[test]
fn noncontinuing_argument_dominates_error_callee_recovery() {
    let output = analyze_text("fn f() -> Int { missing({ return 1; 0 }); }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3003"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
    assert!(!actual.contains(&"N3005"), "{actual:?}");
    assert!(function(&output, "f").body.ty.is_never());
    assert_eq!(expression_statement_type(&output, "f", 0), &Type::Never);
}

#[test]
fn call_hir_still_preserves_lowered_children_for_diagnostics() {
    let output =
        analyze_text("fn target() -> Int { 0 } fn f() -> Int { target({ missing; 0 }); 1 }");
    assert_has_codes(&output, &["N3003", "N3006"]);
    let StatementKind::Expression(expression) = &function(&output, "f").body.statements[0].kind
    else {
        panic!("expected call expression statement");
    };
    let ExpressionKind::Call { arguments, .. } = &expression.kind else {
        panic!("expected call HIR to retain lowered children");
    };
    assert_eq!(arguments.len(), 1);
}
