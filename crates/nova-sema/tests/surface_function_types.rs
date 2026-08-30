use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::analyze;
use nova_sema::hir::{FunctionType, Type};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "surface-function-types.nv", text);
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
fn resolves_nested_surface_function_types_to_existing_hir_signatures() {
    let output = analyze_text(
        "fn higher(f: fn(Int) -> Int) -> fn(Int) -> Int { f } fn inc(x: Int) -> Int { x + 1 }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let higher = &output.program.functions[0];
    let unary = Type::Function(FunctionType {
        parameters: vec![Type::Int],
        return_type: Box::new(Type::Int),
    });
    assert_eq!(higher.parameters[0].ty, unary);
    assert_eq!(higher.return_type, unary);
}

#[test]
fn higher_order_calls_are_checked_through_surface_function_annotations() {
    let output = analyze_text("fn bad(f: fn(Bool) -> Int) -> Int { f(1) }");
    assert!(output.has_errors());
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3004")
    );
}
