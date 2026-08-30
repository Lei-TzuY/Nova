use nova_lexer::lex;
use nova_parser::{ast::ExpressionKind, parse};
use nova_source::{SourceFile, SourceId};

#[test]
fn parses_underscore_as_an_explicit_payload_discard() {
    let source = SourceFile::new(
        SourceId::new(0),
        "pattern-payload-discard.nv",
        "enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(9) { Maybe::None => 0, Maybe::Some(_) => 1 } }",
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let tail = parsed.program.functions[0]
        .body
        .tail
        .as_deref()
        .expect("match tail");
    let ExpressionKind::Match { arms, .. } = &tail.kind else {
        panic!("expected match");
    };
    assert!(!arms[0].pattern.payload_discarded);
    assert!(arms[0].pattern.binding.is_none());
    assert!(arms[1].pattern.payload_discarded);
    assert!(arms[1].pattern.binding.is_none());
    assert_eq!(source.slice(arms[1].pattern.span), Some("Maybe::Some(_)"));
}
