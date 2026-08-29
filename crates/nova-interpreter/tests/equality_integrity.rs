use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{EnumType, ExpressionKind, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "equality-integrity.nv", text);
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
fn malformed_payload_bearing_enum_equality_fails_closed() {
    let mut analyzed = analyze_text(
        "enum Maybe { None, Some(Int) }\n\
         fn main() -> Bool { true == true }",
    );
    let enumeration = analyzed.program.enums[0].id;
    let enum_type = Type::Enum(EnumType {
        id: enumeration,
        name: analyzed.program.enums[0].name.clone(),
    });
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let expression = main.body.tail.as_deref_mut().expect("main tail expression");
    let ExpressionKind::Binary { left, right, .. } = &mut expression.kind else {
        panic!("expected equality expression");
    };
    for operand in [left, right] {
        operand.kind = ExpressionKind::EnumConstructor {
            enumeration,
            variant_index: 0,
            payload: None,
        };
        operand.ty = enum_type.clone();
    }

    let error = execute(&analyzed.program)
        .expect_err("payload-bearing enum equality must fail closed at runtime");
    assert_eq!(error.code, "N4005");
}

#[test]
fn payload_free_enum_equality_remains_executable() {
    let analyzed = analyze_text(
        "enum Color { Red, Blue }\n\
         fn main() -> Bool { Color::Red != Color::Blue }",
    );
    let value = execute(&analyzed.program).expect("payload-free enum equality should execute");
    assert_eq!(value, Value::Bool(true));
}

#[test]
fn noncontinuing_equality_operand_still_propagates_structured_return() {
    let analyzed = analyze_text(
        "fn main() -> Bool {\n\
             (if true { return true; } else { false }) == false\n\
         }",
    );
    let value = execute(&analyzed.program).expect("return inside equality operand should propagate");
    assert_eq!(value, Value::Bool(true));
}
