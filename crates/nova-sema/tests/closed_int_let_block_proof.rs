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
fn immutable_int_binding_block_can_prove_a_guaranteed_loop() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            var answer: Int;
            while ({ let value = 40; value + 2 }) == 42 {
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
fn chained_immutable_int_bindings_compose_in_source_order() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            var answer: Int;
            while ({
                let base = 20;
                let doubled = base * 2;
                doubled + 2
            }) == 42 {
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
fn mutable_binding_block_remains_runtime_only() {
    let actual = codes(
        r#"
        fn main() -> Int {
            var answer: Int;
            while ({ var value = 42; value }) == 42 {
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
fn dynamic_initializer_block_remains_runtime_only() {
    let actual = codes(
        r#"
        fn produce() -> Int { 42 }
        fn main() -> Int {
            var answer: Int;
            while ({ let value = produce(); value }) == 42 {
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
fn dynamic_discarded_expression_keeps_the_block_runtime_only() {
    let actual = codes(
        r#"
        fn produce() -> Int { 1 }
        fn main() -> Int {
            var answer: Int;
            while ({ produce(); 42 }) == 42 {
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
fn closed_discarded_int_expression_preserves_the_proof() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            var answer: Int;
            while ({ 1 + 2; 42 }) == 42 {
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
fn closed_int_binding_block_feeds_static_arithmetic_preflight() {
    let actual = codes(
        r#"
        fn main() -> Int {
            10 / ({ let zero = 0; zero })
        }
        "#,
    );
    assert!(actual.contains(&"N3032".to_owned()), "codes: {actual:?}");
}

#[test]
fn closed_int_binding_payload_can_select_a_match() {
    let output = analyze_text(
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
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn mutable_int_binding_payload_remains_runtime_only() {
    let actual = codes(
        r#"
        enum Choice { Value(Int), Other(Int) }
        fn main() -> Int {
            var answer: Int;
            while match Choice::Value({ var inner = 42; inner }) {
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
