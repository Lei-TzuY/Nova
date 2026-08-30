use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "closed_match_condition.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn direct_payload_free_match_can_prove_a_guaranteed_loop() {
    let analyzed = analyze_text(
        "enum Switch { Off, On }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (match Switch::On {\n\
                 Switch::Off => false,\n\
                 Switch::On => true,\n\
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
fn local_scrutinee_remains_outside_the_closed_match_proof() {
    let analyzed = analyze_text(
        "enum Switch { Off, On }\n\
         fn main() -> Int {\n\
             let selected = Switch::On;\n\
             var value: Int;\n\
             while (match selected {\n\
                 Switch::Off => false,\n\
                 Switch::On => true,\n\
             }) {\n\
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
fn statement_bearing_selected_arm_remains_outside_the_closed_proof() {
    let analyzed = analyze_text(
        "enum Switch { Off, On }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (match Switch::On {\n\
                 Switch::Off => false,\n\
                 Switch::On => { let flag = true; flag },\n\
             }) {\n\
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
fn unselected_statement_bearing_arm_does_not_block_the_proof() {
    let analyzed = analyze_text(
        "enum Switch { Off, On }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (match Switch::On {\n\
                 Switch::Off => { let flag = false; flag },\n\
                 Switch::On => true,\n\
             }) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}
