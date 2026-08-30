use nova_lexer::lex;
use nova_parser::{ast::StatementKind, parse};
use nova_source::{SourceFile, SourceId};

#[test]
fn preserves_bare_and_value_return_forms() {
    let source = SourceFile::new(
        SourceId::new(0),
        "bare-return.nv",
        "fn bare() -> Unit { return; } fn explicit() -> Unit { return (); }",
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);

    assert!(matches!(
        parsed.program.functions[0].body.statements[0].kind,
        StatementKind::Return(None)
    ));
    assert!(matches!(
        parsed.program.functions[1].body.statements[0].kind,
        StatementKind::Return(Some(_))
    ));
}
