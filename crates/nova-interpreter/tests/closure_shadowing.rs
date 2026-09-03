use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::analyze;
use nova_source::{SourceFile, SourceId};

fn accepted(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "closure_shadowing.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn closure_keeps_the_binding_visible_at_creation_across_later_shadowing() {
    let analyzed = accepted(
        "fn main() -> Int {\n\
             let value = 1;\n\
             let read = fn() -> Int { value };\n\
             { let value = 2; read() }\n\
         }",
    );
    assert_eq!(execute(&analyzed.program), Ok(Value::Int(1)));
}

#[test]
fn closure_created_inside_shadowing_scope_captures_the_inner_binding() {
    let analyzed = accepted(
        "fn main() -> Int {\n\
             let value = 1;\n\
             {\n\
                 let value = 2;\n\
                 let read = fn() -> Int { value };\n\
                 read()\n\
             }\n\
         }",
    );
    assert_eq!(execute(&analyzed.program), Ok(Value::Int(2)));
}

#[test]
fn nested_closure_preserves_the_shadowed_binding_captured_by_its_creator() {
    let analyzed = accepted(
        "fn main() -> Int {\n\
             let value = 1;\n\
             let make = fn() -> fn() -> Int {\n\
                 let value = 2;\n\
                 fn() -> Int { value }\n\
             };\n\
             make()()\n\
         }",
    );
    assert_eq!(execute(&analyzed.program), Ok(Value::Int(2)));
}
