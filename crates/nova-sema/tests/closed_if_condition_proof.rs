use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "closed_if_condition.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn selected_statement_free_if_can_prove_a_guaranteed_loop() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while (if true { true } else { false }) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn false_condition_selects_and_recurses_into_the_else_expression() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while (if false { false } else { if 1 < 2 { true } else { false } }) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn statement_bearing_selected_branch_remains_outside_the_closed_proof() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while (if true { let flag = true; flag } else { false }) {\n\
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
fn unselected_statement_bearing_branch_does_not_block_the_proof() {
    let analyzed = analyze_text(
        "fn main() -> Int {\n\
             var value: Int;\n\
             while (if true { true } else { let flag = false; flag }) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}
