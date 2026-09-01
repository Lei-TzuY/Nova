use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "static-match-arithmetic-failure-reachability.nv",
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
fn direct_known_variant_with_dynamic_payload_prunes_unselected_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }
        enum Choice { A(Int), B }

        fn runtime_int(value: Int) -> Int { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(zero) => match Choice::A(runtime_int(1)) {
                    Choice::A(_) => 0,
                    Choice::B => 1 / zero,
                },
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn direct_known_variant_still_reports_selected_outer_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }
        enum Choice { A(Int), B }

        fn runtime_int(value: Int) -> Int { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(zero) => match Choice::A(runtime_int(1)) {
                    Choice::A(_) => 1 / zero,
                    Choice::B => 0,
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn selected_dynamic_payload_binding_shadows_outer_closed_binding() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }
        enum Choice { A(Int), B }

        fn runtime_int(value: Int) -> Int { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => match Choice::A(runtime_int(1)) {
                    Choice::A(x) => 1 / x,
                    Choice::B => 2 / x,
                },
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn immutable_block_alias_preserves_static_variant_reachability() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }
        enum Choice { A(Int), B }

        fn runtime_int(value: Int) -> Int { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(zero) => {
                    let choice = Choice::A(runtime_int(1));
                    match choice {
                        Choice::A(_) => 0,
                        Choice::B => 1 / zero,
                    }
                },
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn immutable_record_alias_preserves_static_field_reachability() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }
        enum Choice { A(Int), B }
        record Holder { choice: Choice }

        fn runtime_int(value: Int) -> Int { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(zero) => {
                    let holder = new Holder { choice: Choice::A(runtime_int(1)) };
                    match holder.choice {
                        Choice::A(_) => 0,
                        Choice::B => 1 / zero,
                    }
                },
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn mutable_alias_keeps_both_match_arms_potentially_executable() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }
        enum Choice { A(Int), B }

        fn runtime_int(value: Int) -> Int { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(zero) => {
                    var choice = Choice::A(runtime_int(1));
                    match choice {
                        Choice::A(_) => 1 / zero,
                        Choice::B => 2 / zero,
                    }
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 2, "{:?}", output.diagnostics);
}
