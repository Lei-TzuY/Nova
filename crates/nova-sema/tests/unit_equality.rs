use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze, hir::Type};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "unit-equality.nv", text);
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

#[test]
fn unit_values_are_equality_comparable() {
    let output = analyze_text(
        "fn same(left: Unit, right: Unit) -> Bool { left == right }\n\
         fn different(left: Unit, right: Unit) -> Bool { left != right }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(output.program.functions[0].body.ty, Type::Bool);
    assert_eq!(output.program.functions[1].body.ty, Type::Bool);
}

#[test]
fn literal_unit_equality_drives_closed_condition_reachability() {
    let output = analyze_text(
        "fn main() -> Int { var value: Int; if () == () { value = 42; () } else { () }; value }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn aggregate_equality_remains_rejected() {
    let output = analyze_text(
        "record Pair { value: Int }\n\
         fn id(value: Int) -> Int { value }\n\
         fn record_eq(a: Pair, b: Pair) -> Bool { a == b }\n\
         fn function_eq() -> Bool { id == id }",
    );
    let codes = output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    assert_eq!(codes, vec!["N3004"]);
}

#[test]
fn unit_returning_calls_do_not_become_constant_conditions() {
    let output = analyze_text(
        "fn unit() -> Unit { () }\n\
         fn main() -> Int { var value: Int; if unit() == unit() { value = 1; () } else { () }; value }",
    );
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3009"),
        "{:?}",
        output.diagnostics
    );
}
