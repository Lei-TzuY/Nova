from pathlib import Path

lib = Path("crates/nova-interpreter/src/lib.rs")
text = lib.read_text()
old = '''    fn eval_expression(
        &mut self,
        expression: &Expression,
        frame: &mut Frame,
    ) -> Result<Flow, Diagnostic> {
        self.step(expression.span)?;
        match &expression.kind {
'''
new = '''    fn eval_expression(
        &mut self,
        expression: &Expression,
        frame: &mut Frame,
    ) -> Result<Flow, Diagnostic> {
        self.step(expression.span)?;
        let flow = self.eval_expression_unchecked(expression, frame)?;
        if let Flow::Value(value) = &flow {
            if !self.value_conforms_to_type(value, &expression.ty) {
                return Err(self.invariant(
                    expression.span,
                    format!(
                        "expression produced a runtime value that does not conform to HIR type {}",
                        expression.ty
                    ),
                ));
            }
        }
        Ok(flow)
    }

    fn eval_expression_unchecked(
        &mut self,
        expression: &Expression,
        frame: &mut Frame,
    ) -> Result<Flow, Diagnostic> {
        match &expression.kind {
'''
if text.count(old) != 1:
    raise SystemExit(f"eval_expression anchor count: {text.count(old)}")
lib.write_text(text.replace(old, new, 1))

tests = Path("crates/nova-interpreter/tests/expression_result_integrity.rs")
if tests.exists():
    raise SystemExit("expression result test file already exists")
tests.write_text(r'''use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "expression-result.nv", text);
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
fn rejects_discarded_primitive_value_with_drifted_hir_type() {
    let mut analyzed = analyze_text("fn main() -> Unit { 42; }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Expression(expression) = &mut main.body.statements[0].kind else {
        panic!("expected expression statement");
    };
    assert!(matches!(expression.kind, ExpressionKind::Integer(42)));
    expression.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("discarded expression drift must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_composed_field_result_with_drifted_hir_type() {
    let mut analyzed = analyze_text(
        "record Box { value: Int } fn main() -> Unit { let boxed = new Box { value: 42 }; boxed.value; }",
    );
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Expression(expression) = &mut main.body.statements[1].kind else {
        panic!("expected field expression statement");
    };
    assert!(matches!(expression.kind, ExpressionKind::FieldAccess { .. }));
    expression.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("field result drift must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn structured_return_bypasses_value_postcondition() {
    let analyzed = analyze_text(
        "fn choose(flag: Bool) -> Int { if flag { return 42; } else { 0 } } fn main() -> Int { choose(true) }",
    );
    let value = execute(&analyzed.program).expect("return flow should remain executable");
    assert_eq!(value, Value::Int(42));
}

#[test]
fn structured_loop_transfers_bypass_value_postcondition() {
    let analyzed = analyze_text(
        "fn main() -> Int { var value: Int = 0; while value < 3 { value = value + 1; if value < 3 { continue; } else { break; }; } value }",
    );
    let value = execute(&analyzed.program).expect("loop transfers should remain executable");
    assert_eq!(value, Value::Int(3));
}
''')

readme = Path("README.md")
text = readme.read_text()
anchor = '''Runtime frames preserve the resolved binding contract too. Each slot records its
resolved type, mutability, and initialization state. Parameters, local bindings,
delayed `var` declarations, and match payload bindings reject non-conforming initial
values or incompatible reuse of one binding identity; repeated execution of the same
lexical binding may refresh its slot only with identical type/mutability metadata.
Assignment additionally requires a mutable slot and a conforming replacement value.
Binding reads verify that the HIR expression type and stored runtime value still agree
with the slot contract. Any such interpreter/HIR drift fails closed with `N4005`.
'''
addition = anchor + '''
Every expression that completes with an ordinary runtime value also has a final
interpreter postcondition: the value must recursively conform to that expression's
typed-HIR result type. This closes gaps for local or discarded literals, projections,
operators, blocks, conditionals, matches, and other values that may never cross a
function, aggregate, or frame-storage boundary. Structured `return`, `break`, and
`continue` propagation is not a runtime value and therefore remains outside this
postcondition. A mismatched value fails closed with `N4005` at the expression span.
'''
if text.count(anchor) != 1:
    raise SystemExit(f"README frame anchor count: {text.count(anchor)}")
readme.write_text(text.replace(anchor, addition, 1))

roadmap = Path("docs/roadmap.md")
text = roadmap.read_text()
old_status = "**Status: thirteen vertical slices implemented; execution surface remains small.**"
new_status = "**Status: fourteen vertical slices implemented; execution surface remains small.**"
if text.count(old_status) != 1:
    raise SystemExit(f"Phase 3 status anchor count: {text.count(old_status)}")
text = text.replace(old_status, new_status, 1)
anchor = '''- malformed-HIR regressions cover initializer drift, delayed assignment drift,
  immutable retargeting, incompatible binding-identity aliasing, and match-payload
  binding drift, while valid mutation and loop-local re-entry controls lock accepted
  execution unchanged.

Next Phase 3 slices should deepen executable semantics without bypassing Phase 2
'''
addition = '''- malformed-HIR regressions cover initializer drift, delayed assignment drift,
  immutable retargeting, incompatible binding-identity aliasing, and match-payload
  binding drift, while valid mutation and loop-local re-entry controls lock accepted
  execution unchanged.

Implemented in the fourteenth Phase 3 slice:

- expression evaluation now has one typed-HIR runtime postcondition around the existing
  per-kind evaluator: every ordinary `Flow::Value` must recursively conform to the
  expression's resolved result type before that value can leave the expression boundary;
- the check applies uniformly to primitive literals, function references, aggregates,
  projections, unary/binary operations, calls, blocks, conditionals, and selected matches,
  including values that are immediately discarded and never reach another runtime boundary;
- structured `Return`, `Break`, and `Continue` flows deliberately bypass the value
  postcondition, preserving the existing propagation semantics for `!` expressions;
- the implementation keeps execution order, step accounting, per-kind invariant checks,
  and valid source behavior unchanged by wrapping rather than rewriting expression logic;
  and
- malformed-HIR regressions cover discarded primitive and composed projection result-type
  drift, while return and loop-transfer controls prove non-value flow remains executable.

Next Phase 3 slices should deepen executable semantics without bypassing Phase 2
'''
if text.count(anchor) != 1:
    raise SystemExit(f"roadmap Phase 3 insertion anchor count: {text.count(anchor)}")
roadmap.write_text(text.replace(anchor, addition, 1))
