use nova_inspect::{build_document, build_document_v2, build_document_v3};
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
fn v1_and_v2_refuse_to_reinterpret_payload_binding_null_as_discard() {
    let (source, analyzed) = accepted(
        "enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(7) { Maybe::None => 0, Maybe::Some(_) => 1 } }",
    );
    let v1 = build_document(&analyzed.program, &source).expect_err("v1 cannot encode discard");
    assert!(
        v1.message().contains("select schema v3"),
        "{}",
        v1.message()
    );
    let v2 = build_document_v2(&analyzed, &source).expect_err("v2 cannot encode discard");
    assert!(
        v2.message().contains("select schema v3"),
        "{}",
        v2.message()
    );
}

#[test]
fn schema_v3_projects_explicit_payload_modes_without_adding_a_binding() {
    let (source, analyzed) = accepted(
        "enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(7) { Maybe::None => 0, Maybe::Some(_) => 1 } }",
    );
    let document = build_document_v3(&analyzed, &source).expect("v3 must represent discard");
    assert_eq!(document.schema_version, 3);
    assert!(document.program.matches[0].arms[1].binding.is_none());
    assert_eq!(document.match_patterns.len(), 2);
    assert_eq!(document.match_patterns[0].arm, "match:0.arm:0");
    assert_eq!(
        document.match_patterns[0].payload_mode,
        nova_inspect::v3::MatchPayloadMode::None
    );
    assert_eq!(
        document.match_patterns[1].payload_mode,
        nova_inspect::v3::MatchPayloadMode::Discard
    );
    assert_eq!(
        document.control_flow.len(),
        document.program.functions.len()
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
    let error = build_document_v3(&analyzed, &source).expect_err("malformed HIR must fail");
    assert!(
        error
            .message()
            .contains("match payload mode does not match"),
        "{}",
        error.message()
    );
}
