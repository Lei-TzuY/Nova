use nova_inspect::{build_document, build_document_v2, build_document_v3, v1};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::analyze;
use nova_source::{SourceFile, SourceId};

#[test]
fn all_existing_schemas_preserve_bare_return_as_a_return_without_child_expression() {
    let source = SourceFile::new(
        SourceId::new(0),
        "bare-return.nv",
        "fn main() -> Unit { return; }",
    );
    let lexed = lex(&source);
    let parsed = parse(&source, &lexed.tokens);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);

    let v1_document = build_document(&analyzed.program, &source).expect("v1 document");
    let v2_document = build_document_v2(&analyzed, &source).expect("v2 document");
    let v3_document = build_document_v3(&analyzed, &source).expect("v3 document");
    for program in [
        &v1_document.program,
        &v2_document.program,
        &v3_document.program,
    ] {
        let statement = program
            .statements
            .iter()
            .find(|statement| statement.kind == v1::StatementKind::Return)
            .expect("return statement");
        assert!(statement.expressions.is_empty());
    }
}
