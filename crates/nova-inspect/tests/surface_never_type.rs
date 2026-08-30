use nova_inspect::{build_document, build_document_v2, build_document_v3, v1};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::analyze;
use nova_source::{SourceFile, SourceId};

#[test]
fn every_supported_schema_projects_surface_never_without_a_version_bump() {
    let source = SourceFile::new(
        SourceId::new(0),
        "never.nv",
        "fn forever() -> ! { while true {} } fn main() -> Int { 42 }",
    );
    let lexed = lex(&source);
    let parsed = parse(&source, &lexed.tokens);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);

    let v1_document = build_document(&analyzed.program, &source).expect("v1 accepts Never");
    let v2_document = build_document_v2(&analyzed, &source).expect("v2 accepts Never");
    let v3_document = build_document_v3(&analyzed, &source).expect("v3 accepts Never");

    for program in [
        &v1_document.program,
        &v2_document.program,
        &v3_document.program,
    ] {
        assert!(
            program
                .types
                .iter()
                .any(|ty| ty.kind == v1::TypeKind::Never && ty.display == "!")
        );
        assert_eq!(
            program.functions[0].return_type,
            program
                .types
                .iter()
                .find(|ty| ty.kind == v1::TypeKind::Never)
                .unwrap()
                .id
        );
    }
}
