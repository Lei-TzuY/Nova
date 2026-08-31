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
fn selected_unit_payload_binding_can_participate_in_equality_proof() {
    let output = analyze_text(
        r#"
        enum Wrap { Value(Unit), Other(Unit) }
        fn main() -> Int {
            var answer: Int;
            while match Wrap::Value(()) {
                Wrap::Value(payload) => payload == (),
                Wrap::Other(_) => false,
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
fn selected_function_payload_binding_preserves_declaration_identity() {
    let output = analyze_text(
        r#"
        enum Wrap { Value(fn(Int) -> Int), Other(fn(Int) -> Int) }
        fn target(value: Int) -> Int { value }
        fn other(value: Int) -> Int { value + 1 }
        fn main() -> Int {
            var answer: Int;
            while match Wrap::Value(target) {
                Wrap::Value(payload) => payload == target,
                Wrap::Other(_) => false,
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
fn selected_payload_free_enum_binding_preserves_variant_identity() {
    let output = analyze_text(
        r#"
        enum Flag { A, B }
        enum Wrap { Value(Flag), Other(Flag) }
        fn main() -> Int {
            var answer: Int;
            while match Wrap::Value(Flag::A) {
                Wrap::Value(payload) => payload == Flag::A,
                Wrap::Other(_) => false,
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
fn selected_record_payload_binding_can_feed_field_projection_proof() {
    let output = analyze_text(
        r#"
        record Box { value: Int }
        enum Wrap { Value(Box), Other(Box) }
        fn main() -> Int {
            var answer: Int;
            while match Wrap::Value(new Box { value: 42 }) {
                Wrap::Value(payload) => payload.value == 42,
                Wrap::Other(_) => false,
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
fn selected_enum_payload_binding_can_be_a_nested_match_scrutinee() {
    let output = analyze_text(
        r#"
        enum Inner { A(Int), B(Int) }
        enum Outer { Value(Inner), Other(Inner) }
        fn main() -> Int {
            var answer: Int;
            while match Outer::Value(Inner::A(42)) {
                Outer::Value(inner) => match inner {
                    Inner::A(value) => value == 42,
                    Inner::B(_) => false,
                },
                Outer::Other(_) => false,
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
fn dynamic_record_payload_sibling_keeps_projection_runtime_only() {
    let actual = codes(
        r#"
        record Box { value: Int, other: Int }
        enum Wrap { Value(Box), Other(Box) }
        fn produce() -> Int { 0 }
        fn main() -> Int {
            var answer: Int;
            while match Wrap::Value(new Box { value: 42, other: produce() }) {
                Wrap::Value(payload) => payload.value == 42,
                Wrap::Other(_) => false,
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
fn selected_record_payload_projection_feeds_static_arithmetic_preflight() {
    let actual = codes(
        r#"
        record Box { value: Int }
        enum Wrap { Value(Box), Other(Box) }
        fn main() -> Int {
            10 / match Wrap::Value(new Box { value: 0 }) {
                Wrap::Value(payload) => payload.value,
                Wrap::Other(_) => 1,
            }
        }
        "#,
    );
    assert!(actual.contains(&"N3032".to_owned()), "codes: {actual:?}");
}
