use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "reachable-state.nv", text);
    let lexed = lex(&source);
    assert!(
        lexed.is_success(),
        "lex diagnostics: {:?}",
        lexed.diagnostics
    );
    let parsed = parse(&source, &lexed.tokens);
    assert!(
        parsed.is_success(),
        "parse diagnostics: {:?}",
        parsed.diagnostics
    );
    analyze(&parsed.program)
}

#[test]
fn unreachable_break_after_continue_does_not_create_a_loop_exit() {
    let output = analyze_text("fn f() -> Int { while true { continue; break; } }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.program.functions[0].body.ty.is_never());
}

#[test]
fn unreachable_tail_break_does_not_create_a_loop_exit() {
    let output = analyze_text("fn f() -> Int { while true { return 1; { break; 0 } } }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.program.functions[0].body.ty.is_never());
}

#[test]
fn diagnostic_only_expression_paths_restore_enclosing_loop_state() {
    let output = analyze_text(
        "enum Choice { A, B } fn f() -> Int { while true { match Choice::A { Choice::A => { continue; }, Choice::B => { break; }, }; } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.program.functions[0].body.ty.is_never());
}
