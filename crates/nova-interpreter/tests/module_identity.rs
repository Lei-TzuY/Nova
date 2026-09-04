use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{ExpressionKind, FunctionId, ModuleId, RecordId, StatementKind};
use nova_sema::{AnalysisOutput, analyze, analyze_in_module};
use nova_source::{SourceFile, SourceId};

fn parsed(text: &str) -> nova_parser::ast::Program {
    let source = SourceFile::new(SourceId::new(0), "module-runtime.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    parsed.program
}

fn accepted(text: &str) -> AnalysisOutput {
    let analyzed = analyze(&parsed(text));
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn executes_a_consistently_module_qualified_program() {
    let program = parsed(
        "record Box { value: Int }\n\
         enum Maybe { None, Some(Box) }\n\
         fn main() -> Int {\n\
             let base = 40;\n\
             let add = fn(delta: Int) -> Int { base + delta };\n\
             let wrapped = Maybe::Some(new Box { value: add(2) });\n\
             match wrapped { Maybe::None => 0, Maybe::Some(value) => value.value }\n\
         }",
    );
    let analyzed = analyze_in_module(&program, ModuleId::new(17));
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    assert_eq!(execute(&analyzed.program), Ok(Value::Int(42)));
}

#[test]
fn forged_function_module_cannot_retarget_the_same_local_index() {
    let mut analyzed = accepted(
        "fn first() -> Int { 1 } fn second() -> Int { 2 } fn main() -> Int { first() }",
    );
    let tail = analyzed.program.functions[2]
        .body
        .tail
        .as_deref_mut()
        .expect("main tail");
    let ExpressionKind::Call { callee, .. } = &mut tail.kind else {
        panic!("expected call");
    };
    let ExpressionKind::Function { function, .. } = &mut callee.kind else {
        panic!("expected function reference");
    };
    *function = FunctionId::in_module(ModuleId::new(9), 1);

    let error = execute(&analyzed.program).expect_err("cross-module target must fail closed");
    assert_eq!(error.code, "N4005");
    assert!(error.labels.iter().any(|label| label.message.contains("module")));
}

#[test]
fn forged_record_and_binding_modules_fail_before_index_lookup() {
    let mut record = accepted(
        "record First { value: Int } record Second { value: Int }\n\
         fn main() -> Int { let item = new First { value: 42 }; item.value }",
    );
    let StatementKind::Binding { initializer, .. } =
        &mut record.program.functions[0].body.statements[0].kind
    else {
        panic!("expected record binding");
    };
    let ExpressionKind::RecordLiteral { record, .. } = &mut initializer.kind else {
        panic!("expected record construction");
    };
    *record = RecordId::in_module(ModuleId::new(9), 1);
    let error = execute(&record.program).expect_err("cross-module record must fail closed");
    assert_eq!(error.code, "N4005");

    let mut binding = accepted("fn main() -> Int { let first = 42; let second = 0; first }");
    let second = match &binding.program.functions[0].body.statements[1].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("expected second binding"),
    };
    let tail = binding.program.functions[0]
        .body
        .tail
        .as_deref_mut()
        .expect("main tail");
    let ExpressionKind::Binding(reference) = &mut tail.kind else {
        panic!("expected binding reference");
    };
    reference.binding = nova_sema::hir::BindingId::in_module(ModuleId::new(9), second.index());
    let error = execute(&binding.program).expect_err("cross-module binding must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn forged_closure_module_is_not_materialized() {
    let mut analyzed = accepted(
        "fn main() -> Int { let base = 40; let add = fn(value: Int) -> Int { base + value }; add(2) }",
    );
    let StatementKind::Binding { initializer, .. } =
        &mut analyzed.program.functions[0].body.statements[1].kind
    else {
        panic!("expected closure binding");
    };
    let ExpressionKind::Closure(closure) = &mut initializer.kind else {
        panic!("expected closure initializer");
    };
    closure.id = nova_sema::hir::ClosureId::in_module(ModuleId::new(9), closure.id.index());

    let error = execute(&analyzed.program).expect_err("cross-module closure must fail closed");
    assert_eq!(error.code, "N4005");
}
