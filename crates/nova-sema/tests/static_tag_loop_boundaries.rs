use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "static-tag-loop-boundaries.nv", text);
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
fn nonterminating_loop_blocks_diagnostic_only_tail_tag() {
    let output = analyze_text(
        r#"
        enum Choice { A, B }

        fn main() -> Int {
            match {
                while true {}
                Choice::A
            } {
                Choice::A => 1 / 0,
                Choice::B => 2 / 0,
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn statically_skipped_loop_preserves_tail_tag_selection() {
    let output = analyze_text(
        r#"
        enum Choice { A, B }

        fn main() -> Int {
            match {
                while false {}
                Choice::A
            } {
                Choice::A => 1 / 0,
                Choice::B => 2 / 0,
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn loop_with_reachable_break_preserves_tail_tag_selection() {
    let output = analyze_text(
        r#"
        enum Choice { A, B }

        fn main() -> Int {
            match {
                while true {
                    break;
                }
                Choice::A
            } {
                Choice::A => 1 / 0,
                Choice::B => 2 / 0,
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}
