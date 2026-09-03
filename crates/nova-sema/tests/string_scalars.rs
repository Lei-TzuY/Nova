use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{ExpressionKind, StatementKind, Type};
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "strings.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "lex diagnostics: {:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(
        parsed.is_success(),
        "parse diagnostics: {:?}",
        parsed.diagnostics
    );
    analyze(&parsed.program)
}

#[test]
fn lowers_strings_through_functions_records_enums_and_match() {
    let output = analyze_text(
        r#"
record Message { text: String }
enum MaybeText { None, Some(String) }
fn identity(value: String) -> String { value }
fn unwrap(value: MaybeText) -> String {
    match value {
        MaybeText::None => "empty",
        MaybeText::Some(text) => identity(text),
    }
}
fn main() -> String {
    let message = new Message { text: "Nova 🦀" };
    unwrap(MaybeText::Some(message.text))
}
"#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    let main = output
        .program
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main function");
    assert_eq!(main.return_type, Type::String);
    assert_eq!(main.body.ty, Type::String);
    let StatementKind::Binding { initializer, .. } = &main.body.statements[0].kind else {
        panic!("message binding");
    };
    let ExpressionKind::RecordLiteral { fields, .. } = &initializer.kind else {
        panic!("record literal");
    };
    assert_eq!(fields[0].value.ty, Type::String);
    assert_eq!(
        fields[0].value.kind,
        ExpressionKind::String("Nova 🦀".to_owned())
    );
}

#[test]
fn closed_string_equality_drives_definite_initialization_without_folding_hir() {
    let output = analyze_text(
        r#"fn main() -> Int {
            var answer: Int;
            if "Nova" == "Nova" { answer = 42; () } else { () };
            answer
        }"#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    let condition = output.program.functions[0]
        .body
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::Expression(expression) => match &expression.kind {
                ExpressionKind::If { condition, .. } => Some(condition.as_ref()),
                _ => None,
            },
            _ => None,
        })
        .expect("if condition");
    assert_eq!(condition.ty, Type::Bool);
    assert!(matches!(condition.kind, ExpressionKind::Binary { .. }));
}

#[test]
fn rejects_mixed_string_equality_and_redefinition_of_the_builtin_type() {
    let mismatch = analyze_text(r#"fn main() -> Bool { "1" == 1 }"#);
    assert!(mismatch
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "N3004"
            && diagnostic.message == "type mismatch"));
    assert!(!mismatch.is_success());

    let duplicate = analyze_text("record String { value: Int } fn main() -> Unit {}");
    assert!(duplicate
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "N3002"
            && diagnostic.message == "duplicate type definition"));
    assert!(!duplicate.is_success());
}
