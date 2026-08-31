use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::analyze;
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "test.nv", text);
    let lexed = lex(&source);
    assert!(
        lexed.is_success(),
        "lex diagnostics: {:?}",
        lexed.diagnostics
    );
    let parsed = parse(&source, &lexed.tokens);
    assert!(
        parsed.is_success(),
        "parse diagnostics: {:?}",
        parsed.diagnostics
    );
    analyze(&parsed.program)
}

fn codes(text: &str) -> Vec<String> {
    analyze_text(text)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn selected_int_payload_binding_can_prove_a_guaranteed_loop() {
    let output = analyze_text(
        r#"
        enum Choice { Value(Int), Other(Int) }
        fn main() -> Int {
            var answer: Int;
            while match Choice::Value(42) {
                Choice::Value(payload) => payload == 42,
                Choice::Other(_) => false,
            } {
                answer = 42;
                break;
            }
            answer
        }
        "#,
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn selected_bool_payload_binding_can_be_the_closed_condition() {
    let output = analyze_text(
        r#"
        enum Switch { Value(Bool), Other(Bool) }
        fn main() -> Int {
            var answer: Int;
            while match Switch::Value(true) {
                Switch::Value(flag) => flag,
                Switch::Other(_) => false,
            } {
                answer = 42;
                break;
            }
            answer
        }
        "#,
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn selected_int_payload_binding_composes_with_arithmetic() {
    let output = analyze_text(
        r#"
        enum Choice { Value(Int), Other(Int) }
        fn main() -> Int {
            var answer: Int;
            while match Choice::Value(40) {
                Choice::Value(payload) => payload + 2 == 42,
                Choice::Other(_) => false,
            } {
                answer = 42;
                break;
            }
            answer
        }
        "#,
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn dynamic_payload_binding_remains_runtime_only() {
    let actual = codes(
        r#"
        enum Choice { Value(Int), Other(Int) }
        fn produce() -> Int { 42 }
        fn main() -> Int {
            var answer: Int;
            while match Choice::Value(produce()) {
                Choice::Value(payload) => payload == 42,
                Choice::Other(_) => false,
            } {
                answer = 42;
                break;
            }
            answer
        }
        "#,
    );
    assert!(actual.contains(&"N3009".to_owned()), "codes: {actual:?}");
}

#[test]
fn statement_bearing_payload_binding_remains_runtime_only() {
    let actual = codes(
        r#"
        enum Choice { Value(Int), Other(Int) }
        fn main() -> Int {
            var answer: Int;
            while match Choice::Value({ let inner = 42; inner }) {
                Choice::Value(payload) => payload == 42,
                Choice::Other(_) => false,
            } {
                answer = 42;
                break;
            }
            answer
        }
        "#,
    );
    assert!(actual.contains(&"N3009".to_owned()), "codes: {actual:?}");
}

#[test]
fn unselected_dynamic_payload_branch_does_not_block_the_proof() {
    let output = analyze_text(
        r#"
        enum Choice { Value(Int), Other(Int) }
        fn produce() -> Int { 0 }
        fn main() -> Int {
            var answer: Int;
            while match Choice::Value(if true { 42 } else { produce() }) {
                Choice::Value(payload) => payload == 42,
                Choice::Other(_) => false,
            } {
                answer = 42;
                break;
            }
            answer
        }
        "#,
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn selected_int_payload_binding_feeds_static_arithmetic_preflight() {
    let actual = codes(
        r#"
        enum Choice { Value(Int), Other(Int) }
        fn main() -> Int {
            10 / match Choice::Value(0) {
                Choice::Value(payload) => payload,
                Choice::Other(_) => 1,
            }
        }
        "#,
    );
    assert!(actual.contains(&"N3032".to_owned()), "codes: {actual:?}");
}

#[test]
fn dynamic_payload_binding_does_not_create_static_arithmetic_failure() {
    let output = analyze_text(
        r#"
        enum Choice { Value(Int), Other(Int) }
        fn zero() -> Int { 0 }
        fn main() -> Int {
            10 / match Choice::Value(zero()) {
                Choice::Value(payload) => payload,
                Choice::Other(_) => 1,
            }
        }
        "#,
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}
