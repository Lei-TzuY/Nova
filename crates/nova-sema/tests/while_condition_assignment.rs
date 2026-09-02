use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "while-condition-assignment.nv", text);
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
fn valid_while_condition_exports_mandatory_assignment_initialization() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            var value: Int;
            while {
                value = 7;
                false
            } {}
            value
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3004"), 0, "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3009"), 0, "{:?}", output.diagnostics);
}

#[test]
fn while_condition_partial_assignment_does_not_initialize_post_loop_state() {
    let output = analyze_text(
        r#"
        fn main(flag: Bool) -> Int {
            var value: Int;
            while {
                if flag {
                    value = 7;
                    false
                } else {
                    false
                }
            } {}
            value
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3004"), 0, "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3009"), 1, "{:?}", output.diagnostics);
}

#[test]
fn while_condition_terminated_branch_does_not_erase_reachable_initialization() {
    let output = analyze_text(
        r#"
        fn main(flag: Bool) -> Int {
            var value: Int;
            while {
                if flag {
                    return 0;
                } else {
                    value = 7;
                    false
                }
            } {}
            value
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3004"), 0, "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3009"), 0, "{:?}", output.diagnostics);
}
