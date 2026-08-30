use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    AnalysisOutput, analyze,
    hir::{StatementKind, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "bare-return.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn bare_return_is_unit_typed_noncontinuation_without_a_synthetic_expression() {
    let output = analyze_text("fn stop() -> Unit { return; }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let function = &output.program.functions[0];
    assert_eq!(function.return_type, Type::Unit);
    assert_eq!(function.body.ty, Type::Never);
    assert!(matches!(
        function.body.statements[0].kind,
        StatementKind::Return(None)
    ));
}

#[test]
fn bare_return_reuses_ordinary_return_type_mismatch_for_non_unit_functions() {
    for source in [
        "fn main() -> Int { return; }",
        "fn main() -> Bool { return; }",
        "fn main() -> ! { return; }",
    ] {
        let output = analyze_text(source);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "N3004"),
            "{source}: {:?}",
            output.diagnostics
        );
    }
}

#[test]
fn explicit_unit_return_remains_value_bearing_hir() {
    let output = analyze_text("fn stop() -> Unit { return (); }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(matches!(
        output.program.functions[0].body.statements[0].kind,
        StatementKind::Return(Some(_))
    ));
}
