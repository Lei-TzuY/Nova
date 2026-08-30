from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected one anchor in {path}, found {count}: {old[:100]!r}")
    p.write_text(text.replace(old, new, 1))


replace_once(
    "crates/nova-sema/src/analyzer.rs",
    "struct Analyzer {\n    diagnostics: Vec<Diagnostic>,\n    record_definitions: Vec<RecordDefinition>,",
    "struct Analyzer {\n    diagnostics: Vec<Diagnostic>,\n    deferred_warnings: Vec<Diagnostic>,\n    record_definitions: Vec<RecordDefinition>,",
)
replace_once(
    "crates/nova-sema/src/analyzer.rs",
    "        Self {\n            diagnostics: Vec::new(),\n            record_definitions: Vec::new(),",
    "        Self {\n            diagnostics: Vec::new(),\n            deferred_warnings: Vec::new(),\n            record_definitions: Vec::new(),",
)
replace_once(
    "crates/nova-sema/src/analyzer.rs",
    "    if !diagnostics_have_errors(&analyzer.diagnostics) {\n        let warnings = analyzer",
    "    if !diagnostics_have_errors(&analyzer.diagnostics) {\n        analyzer.diagnostics.append(&mut analyzer.deferred_warnings);\n        let warnings = analyzer",
)
replace_once(
    "crates/nova-sema/src/analyzer.rs",
    "            let selected_arm = selected_variant_index\n                .is_some_and(|selected| valid_pattern && resolved_index == Some(selected));\n            let value =",
    '''            let selected_arm = selected_variant_index\n                .is_some_and(|selected| valid_pattern && resolved_index == Some(selected));\n            if self.diagnostic_only_depth == 0 && valid_pattern {\n                if let (Some(selected), Some(actual)) =\n                    (selected_variant_index, resolved_index)\n                {\n                    if actual != selected {\n                        if let Some(enumeration) = &scrutinee_enum {\n                            let selected_name = &self.enum_definitions[enumeration.id.index()]\n                                .variants[selected]\n                                .name;\n                            self.deferred_warnings.push(\n                                Diagnostic::warning("N3034", "statically unreachable match arm")\n                                    .with_primary(\n                                        arm.pattern.span,\n                                        format!(\n                                            "this arm matches `{}::{}`, but this scrutinee can only select `{}::{selected_name}`",\n                                            enumeration.name,\n                                            arm.pattern.variant.text,\n                                            enumeration.name\n                                        ),\n                                    )\n                                    .with_secondary(\n                                        scrutinee.span,\n                                        format!(\n                                            "this direct constructor selects variant `{selected_name}`"\n                                        ),\n                                    )\n                                    .with_note(\n                                        "the arm remains name/type checked for deterministic diagnostics but contributes no reachable flow facts",\n                                    ),\n                            );\n                        }\n                    }\n                }\n            }\n            let value =''',
)
replace_once(
    "crates/nova-cli/tests/cli.rs",
    '("valid/literal-match-flow.nv", "42\\n", None),',
    '("valid/literal-match-flow.nv", "42\\n", Some("warning[N3034]")),',
)

Path("crates/nova-sema/tests/static_match_arm_usefulness.rs").write_text(r'''use nova_diagnostics::Severity;
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
    assert!(warnings
        .iter()
        .all(|diagnostic| diagnostic.severity == Severity::Warning));
    assert!(warnings[0].message.contains("unreachable match arm"));
    assert!(warnings[0].notes.iter().any(|note| note.contains("name/type checked")));
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
    assert!(analyzed.diagnostics.iter().all(|diagnostic| diagnostic.code != "N3034"));
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
    assert!(analyzed.diagnostics.iter().any(|diagnostic| diagnostic.code == "N3003"));
    assert!(analyzed.diagnostics.iter().all(|diagnostic| diagnostic.code != "N3034"));
}
''')
