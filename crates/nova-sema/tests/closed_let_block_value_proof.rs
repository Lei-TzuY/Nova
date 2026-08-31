use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "closed_let_block_value.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

fn assert_has_code(output: &AnalysisOutput, code: &str) {
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code),
        "{:?}",
        output.diagnostics
    );
}

#[test]
fn closed_bool_let_block_can_prove_a_guaranteed_loop() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while { let flag = true; flag } {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn mixed_int_and_bool_bindings_compose_in_source_order() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while {\n\
                 let base = 40;\n\
                 let ready = base + 2 == 42;\n\
                 ready\n\
             } {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn closed_bool_binding_can_select_an_int_if_tail() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while ({ let flag = true; if flag { 42 } else { 0 } } == 42) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn closed_unit_binding_block_composes_with_unit_equality() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while (({ let unit = (); unit }) == ()) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn closed_function_binding_block_preserves_declaration_identity() {
    let analyzed = analyze_text(
        "fn target() -> Int { 1 }\n\
         fn other() -> Int { 2 }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (({ let selected = target; selected }) == target) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn closed_enum_binding_block_preserves_variant_identity() {
    let analyzed = analyze_text(
        "enum Signal { Red, Green }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (({ let signal = Signal::Green; signal }) == Signal::Green) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn closed_record_binding_block_can_feed_projection() {
    let analyzed = analyze_text(
        "record Box { value: Int }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (({ let boxed = new Box { value: 42 }; boxed }).value == 42) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn closed_enum_binding_block_can_drive_match_selection() {
    let analyzed = analyze_text(
        "enum Signal { Red, Green }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (match { let signal = Signal::Green; signal } {\n\
                 Signal::Red => false,\n\
                 Signal::Green => true,\n\
             }) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn closed_discarded_non_int_expression_preserves_a_later_int_proof() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while ({ true; 42 } == 42) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn mutable_bool_binding_block_remains_runtime_only() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while { var flag = true; flag } {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(!analyzed.is_success());
    assert_has_code(&analyzed, "N3009");
}

#[test]
fn dynamic_initializer_in_a_bool_block_remains_runtime_only() {
    let analyzed = analyze_text(
        "fn produce() -> Bool { true }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while { let flag = produce(); flag } {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(!analyzed.is_success());
    assert_has_code(&analyzed, "N3009");
}
