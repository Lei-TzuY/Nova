use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "assignment-reachability-integration.nv",
        text,
    );
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
fn dynamic_if_assignment_reports_each_potential_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime_bool(value: Bool) -> Bool { value }

        fn main() -> Int {
            var sink: Int = 0;
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    sink = if runtime_bool(true) {
                        1 / x
                    } else {
                        2 / x
                    };
                    sink
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 2, "{:?}", output.diagnostics);
}

#[test]
fn static_tag_assignment_collects_only_the_selected_match_arm() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }
        enum Choice { A(Int), B }

        fn runtime_int(value: Int) -> Int { value }

        fn main() -> Int {
            var sink: Int = 0;
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(zero) => {
                    sink = match Choice::A(runtime_int(1)) {
                        Choice::A(_) => 1 / zero,
                        Choice::B => 2 / zero,
                    };
                    sink
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn noncontinuing_assignment_rhs_suppresses_following_execution_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn stop() -> ! { stop() }

        fn main() -> Int {
            var sink: Int = 0;
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    sink = stop();
                    let bad = 1 / x;
                    bad
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}
