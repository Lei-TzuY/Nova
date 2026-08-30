use nova_inspect::build_document;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn accepted(text: &str) -> (SourceFile, nova_sema::AnalysisOutput) {
    let source = SourceFile::new(SourceId::new(0), "pattern-payload-discard.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed)
}

#[test]
fn schema_v1_projects_discard_as_payload_variant_without_a_binding() {
    let (source, analyzed) = accepted(
        "enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(7) { Maybe::None => 0, Maybe::Some(_) => 1 } }",
    );
    let document = build_document(&analyzed.program, &source).expect("inspection must succeed");
    assert!(document.program.matches[0].arms[1].binding.is_none());
    assert!(
        document.program.matches[0].arms[1]
            .variant
            .ends_with("variant:1")
    );
}

#[test]
fn inspector_rejects_discard_metadata_removed_from_payload_variant() {
    let (source, mut analyzed) = accepted(
        "enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(7) { Maybe::None => 0, Maybe::Some(_) => 1 } }",
    );
    let tail = analyzed.program.functions[0]
        .body
        .tail
        .as_deref_mut()
        .expect("tail");
    let ExpressionKind::Match { arms, .. } = &mut tail.kind else {
        panic!("expected match");
    };
    arms[1].payload_discarded = false;
    let error = build_document(&analyzed.program, &source).expect_err("malformed HIR must fail");
    assert!(
        error
            .message()
            .contains("match payload mode does not match"),
        "{}",
        error.message()
    );
}
