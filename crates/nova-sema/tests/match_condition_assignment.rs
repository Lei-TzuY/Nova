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
fn selected_match_rhs_unreachable_self_read_does_not_poison_initialization() {
    let output = analyze_text(
        r#"
        enum Choice { Selected, Other }

        fn main() -> Int {
            var value: Int;
            value = match Choice::Selected {
                Choice::Selected => {
                    value = 1;
                    value
                },
                Choice::Other => value,
            };
            value;
            0
        }
        "#,
    );

    // The unselected arm is still checked and reports its own N3009. The selected
    // arm initializes `value`, so neither its read nor the post-assignment read
    // may add another definite-initialization diagnostic.
    assert_eq!(code_count(&output, "N3009"), 1, "{:?}", output.diagnostics);
}

#[test]
fn dynamic_match_rhs_self_read_keeps_outer_assignment_uninitialized() {
    let output = analyze_text(
        r#"
        enum Choice { Left, Right }

        fn main(choice: Choice) -> Int {
            var value: Int;
            value = match choice {
                Choice::Left => value,
                Choice::Right => 1,
            };
            value;
            0
        }
        "#,
    );

    // The reachable self-read reports one N3009, and because that continuing arm
    // participates in the dynamic match intersection, the later read reports one too.
    assert_eq!(code_count(&output, "N3009"), 2, "{:?}", output.diagnostics);
}

#[test]
fn noncontinuing_match_rhs_self_read_does_not_poison_outer_assignment() {
    let output = analyze_text(
        r#"
        enum Choice { Left, Right }

        fn main(choice: Choice) -> Int {
            var value: Int;
            value = match choice {
                Choice::Left => {
                    value;
                    return 0;
                },
                Choice::Right => 1,
            };
            value;
            0
        }
        "#,
    );

    // The returning arm still reports its local invalid read, but because it does
    // not continue to the assignment merge, only the valid Right arm determines
    // post-match initialization.
    assert_eq!(code_count(&output, "N3009"), 1, "{:?}", output.diagnostics);
}

#[test]
fn nested_if_noncontinuing_self_read_does_not_poison_match_assignment() {
    let output = analyze_text(
        r#"
        enum Choice { Left, Right }

        fn main(choice: Choice, flag: Bool) -> Int {
            var value: Int;
            value = match choice {
                Choice::Left => if flag {
                    value;
                    return 0;
                } else {
                    1
                },
                Choice::Right => 2,
            };
            value;
            0
        }
        "#,
    );

    // The nested returning path keeps its local invalid read diagnostic, but it
    // must not participate in either the enclosing if merge or the match RHS merge.
    assert_eq!(code_count(&output, "N3009"), 1, "{:?}", output.diagnostics);
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
