use nova_lexer::lex;
use nova_parser::{ast::TypeRefKind, parse};
use nova_source::{SourceFile, SourceId};

#[test]
fn parses_never_in_direct_and_nested_type_positions() {
    let source = SourceFile::new(
        SourceId::new(0),
        "never.nv",
        "fn sink(value: !) -> ! { while true {} }\nfn higher(f: fn() -> !) -> fn(!) -> ! { higher }",
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    assert!(matches!(
        parsed.program.functions[0].parameters[0].ty.kind,
        TypeRefKind::Never
    ));
    assert!(matches!(
        parsed.program.functions[0].return_type.kind,
        TypeRefKind::Never
    ));
    let TypeRefKind::Function { return_type, .. } =
        &parsed.program.functions[1].parameters[0].ty.kind
    else {
        panic!("expected function parameter type");
    };
    assert!(matches!(return_type.kind, TypeRefKind::Never));
    let TypeRefKind::Function {
        parameters,
        return_type,
    } = &parsed.program.functions[1].return_type.kind
    else {
        panic!("expected function return type");
    };
    assert!(matches!(parameters[0].kind, TypeRefKind::Never));
    assert!(matches!(return_type.kind, TypeRefKind::Never));
}
