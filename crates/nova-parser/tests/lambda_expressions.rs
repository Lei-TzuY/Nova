use nova_lexer::lex;
use nova_parser::ast::{ExpressionKind, TypeRefKind};
use nova_parser::parse;
use nova_source::{SourceFile, SourceId};

fn parse_text(text: &str) -> nova_parser::ParseOutput {
    let source = SourceFile::new(SourceId::new(0), "lambdas.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    parse(&source, &lexed.tokens)
}

#[test]
fn parses_explicitly_typed_lambda_and_immediate_call() {
    let parsed = parse_text("fn main() -> Int { (fn(value: Int,) -> Int { value + 1 })(41) }");
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let tail = parsed.program.functions[0]
        .body
        .tail
        .as_deref()
        .expect("main tail");
    let ExpressionKind::Call { callee, arguments } = &tail.kind else {
        panic!("expected immediate lambda call");
    };
    assert_eq!(arguments.len(), 1);
    let ExpressionKind::Lambda {
        parameters,
        return_type,
        body,
    } = &callee.kind
    else {
        panic!("callee should retain lambda syntax");
    };
    assert_eq!(parameters.len(), 1);
    assert!(matches!(return_type.kind, TypeRefKind::Named(_)));
    assert!(body.tail.is_some());
}

#[test]
fn rejects_a_lambda_without_the_required_parameter_parentheses() {
    let parsed = parse_text("fn main() -> Int { let f = fn value: Int -> Int { value }; 0 }");
    assert!(!parsed.is_success());
    assert!(
        parsed.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "N2001"
                && diagnostic
                    .message
                    .contains("after `fn` in an anonymous function")
        }),
        "{:?}",
        parsed.diagnostics
    );
}
