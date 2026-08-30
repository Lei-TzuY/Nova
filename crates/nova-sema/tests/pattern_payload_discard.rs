use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "pattern-payload-discard.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn hir_distinguishes_payload_discard_from_absent_payload() {
    let analyzed = analyze_text(
        "enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(9) { Maybe::None => 0, Maybe::Some(_) => 42 } }",
    );
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    let tail = analyzed.program.functions[0]
        .body
        .tail
        .as_deref()
        .expect("tail");
    let ExpressionKind::Match { arms, .. } = &tail.kind else {
        panic!("expected match");
    };
    assert!(!arms[0].payload_discarded);
    assert!(arms[0].binding.is_none());
    assert!(arms[1].payload_discarded);
    assert!(arms[1].binding.is_none());
}

#[test]
fn payload_free_variant_cannot_use_discard_syntax() {
    let analyzed = analyze_text(
        "enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::None { Maybe::None(_) => 0, Maybe::Some(value) => value } }",
    );
    assert!(
        analyzed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3022"
                && diagnostic.message == "unexpected pattern payload discard"),
        "{:?}",
        analyzed.diagnostics
    );
}
