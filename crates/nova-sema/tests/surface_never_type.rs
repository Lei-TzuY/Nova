use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::Type};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "never.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn surface_never_reuses_existing_bottom_type_and_branch_join() {
    let analyzed = analyze_text(
        "fn forever() -> ! { while true {} }\n\
         fn choose(flag: Bool) -> Int { if flag { 42 } else { forever() } }\n\
         fn main() -> Int { choose(true) }",
    );
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    assert_eq!(analyzed.program.functions[0].return_type, Type::Never);
    assert_eq!(analyzed.program.functions[0].body.ty, Type::Never);
    assert_eq!(analyzed.program.functions[1].return_type, Type::Int);
    assert_eq!(analyzed.program.functions[1].body.ty, Type::Int);
}

#[test]
fn never_function_rejects_continuing_fallthrough_and_tail_values() {
    let fallthrough = analyze_text("fn bad() -> ! {}");
    assert!(
        fallthrough
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3007")
    );
    let tail = analyze_text("fn bad() -> ! { () }");
    assert!(
        tail.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3004")
    );
}
