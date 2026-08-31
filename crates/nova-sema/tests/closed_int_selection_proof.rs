use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "closed_int_selection.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn selected_if_int_value_composes_with_arithmetic_condition_proof() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while ((if true { 20 } else { 0 }) + 22 == 42) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn selected_match_int_value_composes_with_comparison_proof() {
    let analyzed = analyze_text(
        "enum Choice { Low, High }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while (match Choice::High {\n\
                 Choice::Low => 1,\n\
                 Choice::High => 42,\n\
             } == 42) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn selected_immutable_binding_int_branch_can_be_proven() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while (if true { let selected = 42; selected } else { 0 } == 42) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn selected_mutable_binding_int_branch_remains_outside_the_proof() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while (if true { var selected = 42; selected } else { 0 } == 42) {\n\
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
fn dynamic_match_scrutinee_remains_outside_the_int_proof() {
    let analyzed = analyze_text(
        "enum Choice { Low, High }\n\
         fn main() -> Int {\n\
             let choice = Choice::High;\n\
             var value: Int;\n\
             while (match choice {\n\
                 Choice::Low => 1,\n\
                 Choice::High => 42,\n\
             } == 42) {\n\
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
