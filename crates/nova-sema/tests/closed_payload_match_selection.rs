use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "closed_payload_match.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn closed_integer_payload_can_select_a_discarding_match_arm() {
    let analyzed = analyze_text(
        "enum Choice { A(Int), B(Int) }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (match Choice::B(7) {\n\
                 Choice::A(_) => false,\n\
                 Choice::B(_) => true,\n\
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
fn closed_payload_can_select_an_arm_even_when_the_payload_is_bound_but_unused() {
    let analyzed = analyze_text(
        "enum Choice { A(Int), B(Int) }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (match Choice::B(7) {\n\
                 Choice::A(payload) => false,\n\
                 Choice::B(payload) => true,\n\
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
fn dynamic_call_payload_keeps_match_selection_runtime_only() {
    let analyzed = analyze_text(
        "enum Choice { A(Int), B(Int) }\n\
         fn produce() -> Int { 7 }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (match Choice::B(produce()) {\n\
                 Choice::A(_) => false,\n\
                 Choice::B(_) => true,\n\
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
fn statement_bearing_payload_block_stops_closed_match_selection() {
    let analyzed = analyze_text(
        "enum Choice { A(Int), B(Int) }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (match Choice::B({ let payload = 7; payload }) {\n\
                 Choice::A(_) => false,\n\
                 Choice::B(_) => true,\n\
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
fn dynamic_payload_in_an_unselected_if_branch_does_not_block_selection() {
    let analyzed = analyze_text(
        "enum Choice { A(Int), B(Int) }\n\
         fn produce() -> Int { 7 }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (match (if true { Choice::B(7) } else { Choice::A(produce()) }) {\n\
                 Choice::A(_) => false,\n\
                 Choice::B(_) => true,\n\
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
fn closed_record_payload_can_select_a_discarding_match_arm() {
    let analyzed = analyze_text(
        "record Pair { value: Int }\n\
         enum Choice { A(Pair), B(Pair) }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (match Choice::B(new Pair { value: 7 }) {\n\
                 Choice::A(_) => false,\n\
                 Choice::B(_) => true,\n\
             }) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}
