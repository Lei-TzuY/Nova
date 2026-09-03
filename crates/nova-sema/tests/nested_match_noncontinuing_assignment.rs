use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "nested-match-noncontinuing.nv", text);
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
fn nested_match_noncontinuing_self_read_does_not_poison_outer_assignment() {
    let output = analyze_text(
        r#"
        enum Choice { Left, Right }
        enum Detail { First, Second }

        fn main(choice: Choice, detail: Detail) -> Int {
            var value: Int;
            value = match choice {
                Choice::Left => match detail {
                    Detail::First => {
                        value;
                        return 0;
                    },
                    Detail::Second => 1,
                },
                Choice::Right => 2,
            };
            value;
            0
        }
        "#,
    );

    // The returning inner arm must keep its local invalid-read diagnostic, but it
    // must not participate in either the inner match intersection or the outer
    // assignment merge. The post-assignment read therefore remains initialized.
    assert_eq!(code_count(&output, "N3009"), 1, "{:?}", output.diagnostics);
}
