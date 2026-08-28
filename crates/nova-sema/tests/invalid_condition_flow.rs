use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{StatementKind, Type};
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "invalid-condition-flow.nv", text);
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
fn invalid_if_condition_does_not_export_condition_initialization() {
    let output =
        analyze_text("fn f() -> Int { var x: Int; if ({ x = 1; 0 }) { 1 } else { 2 }; x }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(actual.contains(&"N3009"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 1), &Type::Error);
}

#[test]
fn invalid_if_condition_does_not_export_branch_initialization() {
    let output =
        analyze_text("fn f() -> Int { var x: Int; if 0 { x = 1; 1 } else { x = 2; 2 }; x }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(actual.contains(&"N3009"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 1), &Type::Error);
}

#[test]
fn erroneous_if_condition_is_error_typed_and_fail_closed() {
    let output =
        analyze_text("fn f() -> Int { var x: Int; if missing { x = 1; 1 } else { x = 2; 2 }; x }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3003"), "{actual:?}");
    assert!(actual.contains(&"N3009"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 1), &Type::Error);
}

#[test]
fn invalid_if_condition_discards_branch_break_exits() {
    let output =
        analyze_text("fn f() -> Int { while true { if 0 { break; 1 } else { break; 2 }; } }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
}

#[test]
fn noncontinuing_if_condition_keeps_never_precedence() {
    let output = analyze_text("fn f() -> Int { if ({ return 1; true }) { 2 } else { 3 }; }");
    let actual = codes(&output);
    assert!(!actual.contains(&"N3004"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
    assert_eq!(expression_statement_type(&output, "f", 0), &Type::Never);
    assert!(function(&output, "f").body.ty.is_never());
}

#[test]
fn invalid_while_condition_does_not_export_pretest_initialization() {
    let output = analyze_text("fn f() -> Int { var x: Int; while ({ x = 1; 0 }) {} x }");
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(actual.contains(&"N3009"), "{actual:?}");
}

#[test]
fn invalid_nested_while_condition_discards_outer_break_exit() {
    let output = analyze_text(
        "fn f(flag: Bool) -> Int { while true { while (if flag { break; } else { 0 }) {} } }",
    );
    let actual = codes(&output);
    assert!(actual.contains(&"N3004"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
}

#[test]
fn valid_while_pretest_initialization_still_survives() {
    let output = analyze_text("fn f() -> Int { var x: Int; while ({ x = 1; false }) {} x }");
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(function(&output, "f").body.ty, Type::Int);
}

#[test]
fn noncontinuing_while_condition_still_diverges() {
    let output = analyze_text("fn f() -> Int { while ({ return 1; true }) {} }");
    let actual = codes(&output);
    assert!(!actual.contains(&"N3004"), "{actual:?}");
    assert!(!actual.contains(&"N3007"), "{actual:?}");
    assert!(function(&output, "f").body.ty.is_never());
}
