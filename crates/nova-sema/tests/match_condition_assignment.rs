use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "match-condition-assignment.nv", text);
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
fn while_match_scrutinee_assignment_initializes_post_loop_state() {
    let output = analyze_text(
        r#"
        enum Switch { Off, On }

        fn main(switch: Switch) -> Int {
            var value: Int;
            while match {
                value = 7;
                switch
            } {
                Switch::Off => false,
                Switch::On => false,
            } {}
            value
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3004"), 0, "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3009"), 0, "{:?}", output.diagnostics);
}
