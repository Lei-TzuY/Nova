use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::analyze;
use nova_source::{SourceFile, SourceId};

#[test]
fn closure_captures_and_returns_the_mainline_string_scalar() {
    let source = SourceFile::new(
        SourceId::new(0),
        "closure_string_capture.nv",
        "fn main() -> Bool {\n\
             let message = \"nova\";\n\
             let read = fn() -> String { message };\n\
             read() == \"nova\"\n\
         }",
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    assert_eq!(execute(&analyzed.program), Ok(Value::Bool(true)));
}
