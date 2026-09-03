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

#[test]
fn nested_match_returning_initialization_does_not_leak_into_continuing_flow() {
    let output = analyze_text(
        r#"
        enum Choice { Left, Right }
        enum Detail { First, Second }

        fn main(choice: Choice, detail: Detail) -> Int {
            var value: Int;
            var probe: Int;
            value = match choice {
                Choice::Left => match detail {
                    Detail::First => {
                        probe = 7;
                        probe;
                        return 0;
                    },
                    Detail::Second => 1,
                },
                Choice::Right => 2,
            };
            value;
            probe;
            0
        }
        "#,
    );

    // The returning arm may initialize and read `probe` locally, but because that
    // path cannot continue, its initialization must not flow into the inner or
    // outer match intersections. The post-match `probe` read remains invalid.
    assert_eq!(code_count(&output, "N3009"), 1, "{:?}", output.diagnostics);
}

#[test]
fn if_match_returning_initialization_does_not_leak_into_continuing_flow() {
    let output = analyze_text(
        r#"
        enum Detail { First, Second }

        fn main(flag: Bool, detail: Detail) -> Int {
            var value: Int;
            var probe: Int;
            value = if flag {
                match detail {
                    Detail::First => {
                        probe = 7;
                        probe;
                        return 0;
                    },
                    Detail::Second => 1,
                }
            } else {
                2
            };
            value;
            probe;
            0
        }
        "#,
    );

    // Initialization that exists only on the returning inner match arm must not
    // leak through the inner match continuation or the enclosing dynamic `if`.
    // The outer assignment still initializes `value`, while `probe` remains invalid.
    assert_eq!(code_count(&output, "N3009"), 1, "{:?}", output.diagnostics);
}

#[test]
fn match_if_returning_initialization_does_not_leak_into_continuing_flow() {
    let output = analyze_text(
        r#"
        enum Choice { Left, Right }

        fn main(choice: Choice, flag: Bool) -> Int {
            var value: Int;
            var probe: Int;
            value = match choice {
                Choice::Left => if flag {
                    probe = 7;
                    probe;
                    return 0;
                } else {
                    1
                },
                Choice::Right => 2,
            };
            value;
            probe;
            0
        }
        "#,
    );

    // Initialization that exists only on the returning inner `if` branch must not
    // leak through the `if` continuation or the enclosing dynamic match. The outer
    // assignment still initializes `value`, while `probe` remains invalid.
    assert_eq!(code_count(&output, "N3009"), 1, "{:?}", output.diagnostics);
}
