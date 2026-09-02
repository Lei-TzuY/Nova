use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "invalid-assignment-flow.nv", text);
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
fn invalid_self_assignment_does_not_initialize_delayed_binding() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            var value: Int;
            value = value;
            value;
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3009"), 2, "{:?}", output.diagnostics);
}

#[test]
fn invalid_compound_self_assignment_does_not_initialize_delayed_binding() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            var value: Int;
            var other: Int = 1;
            value = value + other;
            value;
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3009"), 2, "{:?}", output.diagnostics);
}

#[test]
fn unreachable_rhs_self_read_does_not_block_initialization() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            var value: Int;
            var other: Int = 1;
            value = if true { other } else { value };
            value;
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3009"), 1, "{:?}", output.diagnostics);
}

#[test]
fn reachable_rhs_self_read_still_blocks_initialization() {
    let output = analyze_text(
        r#"
        fn main(flag: Bool) -> Int {
            var value: Int;
            var other: Int = 1;
            value = if flag { value } else { other };
            value;
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3009"), 2, "{:?}", output.diagnostics);
}

#[test]
fn initialized_rhs_self_read_does_not_block_outer_assignment() {
    let output = analyze_text(
        r#"
        fn main(flag: Bool) -> Int {
            var value: Int;
            value = if flag {
                value = 1;
                value
            } else {
                2
            };
            value;
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3009"), 0, "{:?}", output.diagnostics);
}

#[test]
fn earlier_invalid_read_does_not_block_later_valid_assignment() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            var value: Int;
            value;
            value = 7;
            value;
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3009"), 1, "{:?}", output.diagnostics);
}
