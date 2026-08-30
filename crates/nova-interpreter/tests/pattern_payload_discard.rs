use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn accepted(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "pattern-payload-discard.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn discarded_payload_is_not_bound_and_selected_arm_executes() {
    let analyzed = accepted(
        "enum Packet { Empty, Data(Int) } fn score(packet: Packet) -> Int { match packet { Packet::Empty => 0, Packet::Data(_) => 42 } } fn main() -> Int { score(Packet::Data(99)) }",
    );
    assert_eq!(execute(&analyzed.program), Ok(Value::Int(42)));
}

#[test]
fn deleting_a_real_payload_binding_is_not_reinterpreted_as_discard() {
    let mut analyzed = accepted(
        "enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(7) { Maybe::None => 0, Maybe::Some(value) => value } }",
    );
    let tail = analyzed.program.functions[0]
        .body
        .tail
        .as_deref_mut()
        .expect("tail");
    let ExpressionKind::Match { arms, .. } = &mut tail.kind else {
        panic!("expected match");
    };
    arms[1].binding = None;
    assert!(!arms[1].payload_discarded);
    let diagnostic = execute(&analyzed.program).expect_err("malformed HIR must fail");
    assert_eq!(diagnostic.code, "N4005");
}
