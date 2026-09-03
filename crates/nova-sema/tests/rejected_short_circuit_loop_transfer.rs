use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "rejected-short-circuit-loop-transfer.nv",
        text,
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

fn codes(output: &AnalysisOutput) -> Vec<&str> {
    output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

#[test]
fn rejected_string_short_circuit_rolls_back_conditional_break_transfer() {
    let output = analyze_text(
        "fn f(flag: Bool) -> Int { while true { \"left\" && { if flag { break; } else { () }; true }; } }",
    );

    assert_eq!(codes(&output), vec!["N3004"], "{:?}", output.diagnostics);
}
