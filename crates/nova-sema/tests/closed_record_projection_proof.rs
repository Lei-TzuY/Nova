use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "closed_record_projection.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn closed_int_projection_can_prove_a_guaranteed_loop() {
    let analyzed = analyze_text(
        "record Pair { value: Int }\n\
         fn main() -> Int {\n\
             var result: Int;\n\
             while (new Pair { value: 42 }.value == 42) {\n\
                 result = 42;\n\
                 break;\n\
             }\n\
             result\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn closed_bool_projection_can_be_a_condition_directly() {
    let analyzed = analyze_text(
        "record Flags { ready: Bool }\n\
         fn main() -> Int {\n\
             var result: Int;\n\
             while new Flags { ready: true }.ready {\n\
                 result = 42;\n\
                 break;\n\
             }\n\
             result\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn closed_unit_projection_participates_in_equality_proof() {
    let analyzed = analyze_text(
        "record Holder { unit: Unit }\n\
         fn main() -> Int {\n\
             var result: Int;\n\
             while (new Holder { unit: () }.unit == ()) {\n\
                 result = 42;\n\
                 break;\n\
             }\n\
             result\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn closed_enum_projection_participates_in_identity_equality() {
    let analyzed = analyze_text(
        "enum Signal { Red, Green }\n\
         record Holder { signal: Signal }\n\
         fn main() -> Int {\n\
             var result: Int;\n\
             while (new Holder { signal: Signal::Green }.signal == Signal::Green) {\n\
                 result = 42;\n\
                 break;\n\
             }\n\
             result\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn closed_function_projection_participates_in_identity_equality() {
    let analyzed = analyze_text(
        "record Holder { action: fn() -> Int }\n\
         fn chosen() -> Int { 42 }\n\
         fn other() -> Int { 0 }\n\
         fn main() -> Int {\n\
             var result: Int;\n\
             while (new Holder { action: chosen }.action == chosen) {\n\
                 result = 42;\n\
                 break;\n\
             }\n\
             result\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn closed_enum_projection_can_select_a_payload_match_arm() {
    let analyzed = analyze_text(
        "enum Choice { A(Int), B(Int) }\n\
         record Holder { choice: Choice }\n\
         fn main() -> Int {\n\
             var result: Int;\n\
             while (match new Holder { choice: Choice::B(7) }.choice {\n\
                 Choice::A(_) => false,\n\
                 Choice::B(_) => true,\n\
             }) {\n\
                 result = 42;\n\
                 break;\n\
             }\n\
             result\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn dynamic_sibling_initializer_blocks_projection_proof() {
    let analyzed = analyze_text(
        "record Pair { wanted: Int, other: Int }\n\
         fn produce() -> Int { 7 }\n\
         fn main() -> Int {\n\
             var result: Int;\n\
             while (new Pair { wanted: 42, other: produce() }.wanted == 42) {\n\
                 result = 42;\n\
                 break;\n\
             }\n\
             result\n\
         }",
    );

    assert!(!analyzed.is_success());
    assert!(
        analyzed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3009"),
        "{:?}",
        analyzed.diagnostics
    );
}

#[test]
fn dynamic_unselected_initializer_branch_does_not_block_projection_proof() {
    let analyzed = analyze_text(
        "record Pair { wanted: Int, other: Int }\n\
         fn produce() -> Int { 7 }\n\
         fn main() -> Int {\n\
             var result: Int;\n\
             while (new Pair {\n\
                 wanted: 42,\n\
                 other: if true { 7 } else { produce() },\n\
             }.wanted == 42) {\n\
                 result = 42;\n\
                 break;\n\
             }\n\
             result\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn nested_closed_record_projections_compose() {
    let analyzed = analyze_text(
        "record Inner { value: Int }\n\
         record Outer { inner: Inner }\n\
         fn main() -> Int {\n\
             var result: Int;\n\
             while (new Outer { inner: new Inner { value: 42 } }.inner.value == 42) {\n\
                 result = 42;\n\
                 break;\n\
             }\n\
             result\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}
