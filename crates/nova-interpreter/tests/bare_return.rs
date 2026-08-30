use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::Type};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "bare-return.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn bare_unit_return_executes_as_unit_and_calls_continue_normally() {
    let analyzed = analyze_text("fn stop() -> Unit { return; } fn main() -> Int { stop(); 42 }");
    assert_eq!(
        execute(&analyzed.program).expect("program executes"),
        Value::Int(42)
    );
}

#[test]
fn runtime_boundary_rejects_bare_return_retyped_to_int() {
    let mut analyzed = analyze_text("fn main() -> Unit { return; }");
    analyzed.program.functions[0].return_type = Type::Int;
    let error = execute(&analyzed.program).expect_err("Unit return must not inhabit Int");
    assert_eq!(error.code, "N4005");
}
