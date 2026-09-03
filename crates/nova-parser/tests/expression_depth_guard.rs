use nova_lexer::lex;
use nova_parser::parse;
use nova_source::{SourceFile, SourceId};

fn parse_nested_parentheses(depth: usize) -> nova_parser::ParseOutput {
    let text = format!(
        "fn main() -> Int {{ {}1{} }}",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    let source = SourceFile::new(SourceId::new(0), "expression_depth.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    parse(&source, &lexed.tokens)
}

#[test]
fn moderate_expression_nesting_remains_accepted() {
    let parsed = parse_nested_parentheses(64);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
}

#[test]
fn excessive_expression_nesting_fails_closed_before_host_stack_exhaustion() {
    let parsed = parse_nested_parentheses(192);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N2008"),
        "{:?}",
        parsed.diagnostics
    );
    assert!(
        parsed.diagnostics.len() < 20,
        "recovery diagnostic cascade: {:?}",
        parsed.diagnostics
    );
}
