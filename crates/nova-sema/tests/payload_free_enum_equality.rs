use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze, hir::Type};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "payload-free-enum-equality.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "lex diagnostics: {:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "parse diagnostics: {:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

fn codes(output: &AnalysisOutput) -> Vec<&str> {
    output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

#[test]
fn payload_free_enum_values_are_equality_comparable() {
    let output = analyze_text(
        "enum Color { Red, Green, Blue }\n\
         fn same(left: Color, right: Color) -> Bool { left == right }\n\
         fn different(left: Color, right: Color) -> Bool { left != right }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(output.program.functions[0].body.ty, Type::Bool);
    assert_eq!(output.program.functions[1].body.ty, Type::Bool);
}

#[test]
fn direct_payload_free_enum_comparison_refines_reachability() {
    let output = analyze_text(
        "enum Color { Red, Green }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             if Color::Red == Color::Red && Color::Red != Color::Green {\n\
                 value = 42;\n\
                 ()\n\
             } else { () };\n\
             value\n\
         }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn enum_with_any_payload_remains_non_comparable() {
    let output = analyze_text(
        "enum MaybeInt { None, Some(Int) }\n\
         fn equal(left: MaybeInt, right: MaybeInt) -> Bool { left == right }",
    );
    assert_eq!(codes(&output), vec!["N3004"]);
}

#[test]
fn different_nominal_enums_cannot_be_compared() {
    let output = analyze_text(
        "enum Left { Same }\n\
         enum Right { Same }\n\
         fn equal(left: Left, right: Right) -> Bool { left == right }",
    );
    assert_eq!(codes(&output), vec!["N3004"]);
}

#[test]
fn enum_returning_calls_remain_dynamic_conditions() {
    let output = analyze_text(
        "enum Color { Red, Green }\n\
         fn red() -> Color { Color::Red }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             if red() == red() { value = 1; () } else { () };\n\
             value\n\
         }",
    );
    assert_eq!(codes(&output), vec!["N3009"]);
}
