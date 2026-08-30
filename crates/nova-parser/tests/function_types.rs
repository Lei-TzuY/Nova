use nova_lexer::lex;
use nova_parser::ast::TypeRefKind;
use nova_parser::parse;
use nova_source::{SourceFile, SourceId};

fn parse_text(text: &str) -> nova_parser::ParseOutput {
    let source = SourceFile::new(SourceId::new(0), "function-types.nv", text);
    let lexed = lex(&source);
    assert!(
        lexed.is_success(),
        "lex diagnostics: {:?}",
        lexed.diagnostics
    );
    parse(&source, &lexed.tokens)
}

#[test]
fn parses_recursive_surface_function_types() {
    let parsed =
        parse_text("fn apply(f: fn(fn(Int) -> Int, Int) -> Int, x: Int) -> fn(Int) -> Int { f }");
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let function = &parsed.program.functions[0];
    let TypeRefKind::Function {
        parameters,
        return_type,
    } = &function.parameters[0].ty.kind
    else {
        panic!("parameter should be a function type");
    };
    assert_eq!(parameters.len(), 2);
    assert!(matches!(parameters[0].kind, TypeRefKind::Function { .. }));
    assert!(matches!(return_type.kind, TypeRefKind::Named(_)));
    assert!(matches!(
        function.return_type.kind,
        TypeRefKind::Function { .. }
    ));
}

#[test]
fn rejects_pathological_function_type_nesting_without_unbounded_recursion() {
    let mut ty = "Int".to_owned();
    for _ in 0..140 {
        ty = format!("fn() -> {ty}");
    }
    let parsed = parse_text(&format!("fn main() -> {ty} {{ main }}"));
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N2009"),
        "{:?}",
        parsed.diagnostics
    );
    assert!(parsed.diagnostics.len() < 20, "{:?}", parsed.diagnostics);
}
