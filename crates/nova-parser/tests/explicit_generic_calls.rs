use nova_lexer::lex;
use nova_parser::ast::{BinaryOperator, ExpressionKind, TypeRefKind};
use nova_parser::parse;
use nova_source::{SourceFile, SourceId};

fn parse_text(text: &str) -> nova_parser::ParseOutput {
    let source = SourceFile::new(SourceId::new(0), "explicit-generics.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    parse(&source, &lexed.tokens)
}

#[test]
fn parses_explicit_generic_call_without_stealing_less_than() {
    let parsed = parse_text(
        "fn id<T>(value: T) -> T { value } fn main() -> Bool { id<Int>(1) < 2 }",
    );
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let tail = parsed.program.functions[1]
        .body
        .tail
        .as_deref()
        .expect("main tail");
    let ExpressionKind::Binary {
        operator: BinaryOperator::Less,
        left,
        ..
    } = &tail.kind
    else {
        panic!("expected outer less-than comparison: {tail:?}");
    };
    let ExpressionKind::Call { type_arguments, .. } = &left.kind else {
        panic!("expected explicit generic call: {left:?}");
    };
    assert_eq!(type_arguments.len(), 1);
    let TypeRefKind::Named(name) = &type_arguments[0].kind else {
        panic!("expected named type argument");
    };
    assert_eq!(name.text, "Int");
}

#[test]
fn ordinary_less_than_remains_a_comparison() {
    let parsed = parse_text("fn main() -> Bool { let left = 1; let right = 2; left < right }");
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let tail = parsed.program.functions[0]
        .body
        .tail
        .as_deref()
        .expect("main tail");
    assert!(matches!(
        tail.kind,
        ExpressionKind::Binary {
            operator: BinaryOperator::Less,
            ..
        }
    ));
}
