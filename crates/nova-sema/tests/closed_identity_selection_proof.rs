use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "closed_identity_selection.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn selected_empty_unit_if_can_prove_a_guaranteed_loop() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while ((if true {} else { () }) == ()) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn selected_enum_if_can_prove_identity_equality() {
    let analyzed = analyze_text(
        "enum Signal { Red, Green }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while ((if true { Signal::Green } else { Signal::Red }) == Signal::Green) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn selected_function_if_can_prove_declaration_identity() {
    let analyzed = analyze_text(
        "fn left() -> Int { 1 }\n\
         fn right() -> Int { 2 }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while ((if true { left } else { right }) == left) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn selected_match_can_produce_a_closed_enum_identity() {
    let analyzed = analyze_text(
        "enum Choice { A, B }\n\
         enum Signal { Red, Green }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while ((match Choice::B {\n\
                 Choice::A => Signal::Red,\n\
                 Choice::B => Signal::Green,\n\
             }) == Signal::Green) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn dynamic_match_scrutinees_remain_outside_identity_proofs() {
    let analyzed = analyze_text(
        "enum Choice { A, B }\n\
         enum Signal { Red, Green }\n\
         fn main() -> Int {\n\
             let choice = Choice::B;\n\
             var value: Int;\n\
             while ((match choice {\n\
                 Choice::A => Signal::Red,\n\
                 Choice::B => Signal::Green,\n\
             }) == Signal::Green) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
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
fn statement_bearing_selected_identity_branch_stops_the_proof() {
    let analyzed = analyze_text(
        "enum Signal { Red, Green }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while ((if true {\n\
                 let signal = Signal::Green;\n\
                 signal\n\
             } else { Signal::Red }) == Signal::Green) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
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
fn statement_bearing_unselected_identity_branch_does_not_block_the_proof() {
    let analyzed = analyze_text(
        "enum Signal { Red, Green }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while ((if true { Signal::Green } else {\n\
                 let signal = Signal::Red;\n\
                 signal\n\
             }) == Signal::Green) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}
