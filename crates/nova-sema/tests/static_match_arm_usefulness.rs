use nova_diagnostics::Severity;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "usefulness.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn warns_for_nonselected_arms_of_a_direct_enum_constructor() {
    let analyzed = analyze_text(
        "enum Signal { Red, Amber, Green }\n\
         fn main() -> Int {\n\
             match Signal::Green {\n\
                 Signal::Red => 1,\n\
                 Signal::Amber => 2,\n\
                 Signal::Green => 42,\n\
             }\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    let warnings = analyzed
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "N3034")
        .collect::<Vec<_>>();
    assert_eq!(warnings.len(), 2, "{:?}", analyzed.diagnostics);
    assert!(
        warnings
            .iter()
            .all(|diagnostic| diagnostic.severity == Severity::Warning)
    );
    assert!(warnings[0].message.contains("unreachable match arm"));
    assert!(
        warnings[0]
            .notes
            .iter()
            .any(|note| note.contains("name/type checked"))
    );
}

#[test]
fn does_not_guess_usefulness_for_a_dynamic_scrutinee() {
    let analyzed = analyze_text(
        "enum Signal { Red, Green }\n\
         fn choose(signal: Signal) -> Int {\n\
             match signal { Signal::Red => 1, Signal::Green => 42 }\n\
         }\n\
         fn main() -> Int { choose(Signal::Green) }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    assert!(
        analyzed
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "N3034")
    );
}

#[test]
fn semantic_errors_suppress_static_match_usefulness_warnings() {
    let analyzed = analyze_text(
        "enum Signal { Red, Green }\n\
         fn main() -> Int {\n\
             match Signal::Green {\n\
                 Signal::Red => missing,\n\
                 Signal::Green => 42,\n\
             }\n\
         }",
    );

    assert!(!analyzed.is_success());
    assert!(
        analyzed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3003")
    );
    assert!(
        analyzed
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "N3034")
    );
}
