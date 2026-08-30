use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "closed_record_arithmetic.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn closed_projected_zero_divisor_is_rejected_before_execution() {
    let analyzed = analyze_text(
        "record Box { value: Int }\n\
         fn main() -> Int { 10 / new Box { value: 0 }.value }",
    );

    assert!(!analyzed.is_success());
    assert!(
        analyzed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3032"),
        "{:?}",
        analyzed.diagnostics
    );
}

#[test]
fn closed_projected_operand_can_prove_overflow() {
    let analyzed = analyze_text(
        "record Box { value: Int }\n\
         fn main() -> Int { 9223372036854775807 + new Box { value: 1 }.value }",
    );

    assert!(!analyzed.is_success());
    assert!(
        analyzed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3031"),
        "{:?}",
        analyzed.diagnostics
    );
}

#[test]
fn dynamic_sibling_initializer_keeps_projected_arithmetic_runtime_checked() {
    let analyzed = analyze_text(
        "record Pair { divisor: Int, side: Int }\n\
         fn produce() -> Int { 1 }\n\
         fn main() -> Int {\n\
             10 / new Pair { divisor: 0, side: produce() }.divisor\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    assert!(
        analyzed
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "N3032")
    );
}

#[test]
fn dynamic_unselected_sibling_branch_does_not_hide_projected_zero_divisor() {
    let analyzed = analyze_text(
        "record Pair { divisor: Int, side: Int }\n\
         fn produce() -> Int { 1 }\n\
         fn main() -> Int {\n\
             10 / new Pair {\n\
                 divisor: 0,\n\
                 side: if true { 1 } else { produce() },\n\
             }.divisor\n\
         }",
    );

    assert!(!analyzed.is_success());
    assert!(
        analyzed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3032"),
        "{:?}",
        analyzed.diagnostics
    );
}
