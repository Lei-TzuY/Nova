use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "generic-closed-value-arithmetic-preflight.nv",
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
fn closed_unit_call_argument_reports_zero_division() {
    let output = analyze_text(
        r#"
        fn consume(value: Unit) -> Int { 0 }
        fn main() -> Int {
            consume({
                let zero = 0;
                let bad = 1 / zero;
                ()
            })
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn closed_unit_binding_initializer_reports_overflow() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            let value = {
                let max = 9223372036854775807;
                let bad = max + 1;
                ()
            };
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3031"), 1, "{:?}", output.diagnostics);
}

#[test]
fn closed_int_binding_initializer_reports_zero_division() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            let value = {
                let zero = 0;
                let bad = 1 / zero;
                7
            };
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn closed_bool_binding_initializer_reports_overflow() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            let value = {
                let max = 9223372036854775807;
                let bad = max + 1;
                true
            };
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3031"), 1, "{:?}", output.diagnostics);
}

#[test]
fn closed_standalone_unit_expression_reports_zero_division() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            {
                let zero = 0;
                let bad = 1 / zero;
                ()
            };
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn closed_function_tail_value_reports_zero_division() {
    let output = analyze_text(
        r#"
        fn helper() -> Unit {
            {
                let zero = 0;
                let bad = 1 / zero;
                ()
            }
        }
        fn main() -> Int { 0 }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn dynamic_closed_shape_remains_runtime_only() {
    let output = analyze_text(
        r#"
        fn identity(value: Int) -> Int { value }
        fn consume(value: Unit) -> Int { 0 }
        fn main() -> Int {
            consume({
                let zero = identity(0);
                let bad = 1 / zero;
                ()
            })
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn proven_unselected_closed_value_does_not_report_failure() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            if true {
                1
            } else {
                {
                    let zero = 0;
                    let bad = 1 / zero;
                    ()
                };
                2
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}
