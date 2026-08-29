use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, FunctionType, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "function-equality.nv", text);
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
    let analyzed = analyze(&parsed.program);
    assert!(
        analyzed.is_success(),
        "semantic diagnostics: {:?}",
        analyzed.diagnostics
    );
    analyzed
}

#[test]
fn runtime_function_aliases_compare_by_declaration_identity() {
    let analyzed = analyze_text(
        "fn first() -> Int { 1 }\n\
         fn second() -> Int { 2 }\n\
         fn main() -> Bool {\n\
             let left = first;\n\
             let right = second;\n\
             left == first && left != right\n\
         }",
    );
    let value = execute(&analyzed.program).expect("function identity equality should execute");
    assert_eq!(value, Value::Bool(true));
}

#[test]
fn malformed_function_equality_signature_drift_fails_closed() {
    let mut analyzed = analyze_text(
        "fn first() -> Int { 1 }\n\
         fn second() -> Int { 2 }\n\
         fn flag() -> Bool { true }\n\
         fn main() -> Bool { first == second }",
    );
    let flag = analyzed
        .program
        .functions
        .iter()
        .find(|function| function.name == "flag")
        .expect("flag function")
        .id;
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let expression = main.body.tail.as_deref_mut().expect("main tail expression");
    let ExpressionKind::Binary { right, .. } = &mut expression.kind else {
        panic!("expected equality expression");
    };
    right.kind = ExpressionKind::Function(flag);
    right.ty = Type::Function(FunctionType {
        parameters: Vec::new(),
        return_type: Box::new(Type::Bool),
    });

    let error = execute(&analyzed.program).expect_err("signature drift must fail closed");
    assert_eq!(error.code, "N4005");
}
