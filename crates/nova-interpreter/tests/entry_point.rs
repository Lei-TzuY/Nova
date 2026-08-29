use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::analyze;
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "entry-point.nv", text);
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
    let analyzed = analyze(&parsed.program);
    assert!(
        analyzed.is_success(),
        "semantic diagnostics: {:?}",
        analyzed.diagnostics
    );
    analyzed
}

#[test]
fn unit_main_is_an_executable_entry_point() {
    let analyzed = analyze_text("fn main() -> Unit {}");
    let value = execute(&analyzed.program).expect("Unit main should execute");
    assert_eq!(value, Value::Unit);
}

#[test]
fn aggregate_main_remains_outside_the_bootstrap_entry_point_contract() {
    let analyzed =
        analyze_text("record Box { value: Int } fn main() -> Box { new Box { value: 42 } }");
    let error = execute(&analyzed.program).expect_err("record-valued main must remain rejected");
    assert_eq!(error.code, "N4001");
}
