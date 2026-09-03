use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "nested-loop-flow.nv", text);
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
fn inner_continue_backedge_does_not_leak_returning_initialization_to_outer_break() {
    let output = analyze_text(
        "fn f(inner: Bool, stop: Bool) -> Int {\n\
             var value: Int;\n\
             while true {\n\
                 while inner {\n\
                     if stop {\n\
                         value = 7;\n\
                         return 0;\n\
                     } else {\n\
                         value;\n\
                         continue;\n\
                     };\n\
                 }\n\
                 break;\n\
             }\n\
             value;\n\
             0\n\
         }",
    );

    let n3009_count = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "N3009")
        .count();

    // The returning branch may initialize `value` only on a path that never reaches
    // either the inner backedge or the outer break. The inner continue-path read and
    // the post-outer-loop read therefore both remain maybe-uninitialized.
    assert_eq!(n3009_count, 2, "{:?}", output.diagnostics);
}
