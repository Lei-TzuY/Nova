use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "closed-identity-arithmetic-failures.nv",
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
fn closed_unit_identity_operand_propagates_zero_division() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            if ({
                let zero = 0;
                let bad = 1 / zero;
                ()
            } == ()) {
                1
            } else {
                0
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn closed_enum_identity_operand_propagates_overflow() {
    let output = analyze_text(
        r#"
        enum Signal { Red, Green }
        fn main() -> Int {
            if ({
                let max = 9223372036854775807;
                let bad = max + 1;
                Signal::Green
            } == Signal::Green) {
                1
            } else {
                0
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3031"), 1, "{:?}", output.diagnostics);
}

#[test]
fn closed_function_identity_operand_propagates_zero_division() {
    let output = analyze_text(
        r#"
        fn target() -> Int { 1 }
        fn main() -> Int {
            if ({
                let zero = 0;
                let bad = 1 / zero;
                target
            } == target) {
                1
            } else {
                0
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn dynamic_identity_operand_remains_runtime_only() {
    let output = analyze_text(
        r#"
        fn identity(value: Int) -> Int { value }
        fn main() -> Int {
            if ({
                let zero = identity(0);
                let bad = 1 / zero;
                ()
            } == ()) {
                1
            } else {
                0
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn short_circuit_control_path_propagates_closed_arithmetic_failure() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            if ({
                let zero = 0;
                let bad = 1 / zero;
                true
            } && true) {
                1
            } else {
                0
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}
