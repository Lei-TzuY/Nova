use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind},
};
use nova_source::{SourceFile, SourceId};

#[test]
fn hir_preserves_resolved_record_field_name_and_slot() {
    let source = SourceFile::new(
        SourceId::new(0),
        "record-field-identity.nv",
        "record Pair { left: Int, right: Int }\n\
         fn main() -> Int {\n\
             let pair = new Pair { right: 2, left: 1 };\n\
             pair.left\n\
         }",
    );
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

    let main = analyzed
        .program
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Binding { initializer, .. } = &main.body.statements[0].kind else {
        panic!("expected record binding");
    };
    let ExpressionKind::RecordLiteral { fields, .. } = &initializer.kind else {
        panic!("expected record literal");
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].field_name, "right");
    assert_eq!(fields[0].field_index, 1);
    assert_eq!(fields[1].field_name, "left");
    assert_eq!(fields[1].field_index, 0);

    let projection = main.body.tail.as_deref().expect("main tail");
    let ExpressionKind::FieldAccess {
        field_name,
        field_index,
        ..
    } = &projection.kind
    else {
        panic!("expected field access");
    };
    assert_eq!(field_name, "left");
    assert_eq!(*field_index, 0);
}
