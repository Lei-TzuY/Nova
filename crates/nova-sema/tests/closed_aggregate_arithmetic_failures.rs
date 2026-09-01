use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "closed-aggregate-arithmetic-failures.nv",
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
fn direct_enum_record_payload_propagates_projected_overflow() {
    let output = analyze_text(
        r#"
        record Inner { value: Int }
        record Payload { projected: Int }
        enum Choice { A(Payload), B(Payload) }
        fn main() -> Int {
            match Choice::A(new Payload {
                projected: ({
                    let max = 9223372036854775807;
                    let bad = max + 1;
                    new Inner { value: 1 }
                }).value,
            }) {
                Choice::A(_) => 1,
                Choice::B(_) => 2,
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3031"), 1, "{:?}", output.diagnostics);
}

#[test]
fn closed_record_binding_propagates_nested_projection_failure() {
    let output = analyze_text(
        r#"
        record Inner { value: Int }
        record Payload { projected: Int }
        fn main() -> Int {
            if ({
                let payload = new Payload {
                    projected: ({
                        let zero = 0;
                        let bad = 1 / zero;
                        new Inner { value: 1 }
                    }).value,
                };
                payload
            }).projected == 1 {
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
fn nested_record_closure_propagates_deep_overflow() {
    let output = analyze_text(
        r#"
        record Inner { value: Int }
        record Payload { projected: Int }
        record Outer { payload: Payload }
        fn main() -> Int {
            if new Outer {
                payload: new Payload {
                    projected: ({
                        let max = 9223372036854775807;
                        let bad = max + 1;
                        new Inner { value: 1 }
                    }).value,
                },
            }.payload.projected == 1 {
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
fn dynamic_sibling_does_not_hide_reachable_closed_failure() {
    let output = analyze_text(
        r#"
        record Inner { value: Int }
        record Payload { dynamic: Int, projected: Int }
        fn identity(value: Int) -> Int { value }
        fn main() -> Int {
            let payload = new Payload {
                dynamic: identity(7),
                projected: ({
                    let zero = 0;
                    let bad = 1 / zero;
                    new Inner { value: 1 }
                }).value,
            };
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn dynamic_projection_divisor_remains_runtime_only() {
    let output = analyze_text(
        r#"
        record Inner { value: Int }
        record Payload { projected: Int }
        fn identity(value: Int) -> Int { value }
        fn main() -> Int {
            if new Payload {
                projected: ({
                    let zero = identity(0);
                    let bad = 1 / zero;
                    new Inner { value: 1 }
                }).value,
            }.projected == 1 {
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
fn proven_unselected_aggregate_branch_does_not_report_failure() {
    let output = analyze_text(
        r#"
        record Inner { value: Int }
        record Payload { projected: Int }
        fn main() -> Int {
            if true {
                1
            } else {
                let payload = new Payload {
                    projected: ({
                        let zero = 0;
                        let bad = 1 / zero;
                        new Inner { value: 1 }
                    }).value,
                };
                payload.projected
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}
