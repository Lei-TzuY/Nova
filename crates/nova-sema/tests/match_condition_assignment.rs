use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "noncontinuing-assignment.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

fn code_count(output: &AnalysisOutput, code: &str) -> usize {
    output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .count()
}

#[test]
fn never_rhs_assignment_does_not_initialize_unreachable_continuation() {
    let output = analyze_text(
        r#"
        fn stop() -> ! { stop() }

        fn main() -> Int {
            var value: Int;
            value = stop();
            value;
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3004"), 0, "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3009"), 1, "{:?}", output.diagnostics);
}
