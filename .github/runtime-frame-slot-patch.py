from pathlib import Path

lib = Path("crates/nova-interpreter/src/lib.rs")
text = lib.read_text()

old = """type Frame = BTreeMap<BindingId, Option<Value>>;\n\nenum Flow {"""
new = """struct RuntimeSlot {\n    ty: Type,\n    mutable: bool,\n    value: Option<Value>,\n}\n\ntype Frame = BTreeMap<BindingId, RuntimeSlot>;\n\nenum Flow {"""
if old not in text:
    raise SystemExit("Frame anchor not found")
text = text.replace(old, new, 1)

old = """        let mut frame = Frame::new();\n        for (parameter, argument) in function.parameters.iter().zip(arguments) {\n            frame.insert(parameter.id, Some(argument));\n        }\n\n        self.call_depth += 1;"""
new = """        let mut frame = Frame::new();\n        for (parameter, argument) in function.parameters.iter().zip(arguments) {\n            self.bind_runtime_slot(&mut frame, parameter, Some(argument), function.span)?;\n        }\n\n        self.call_depth += 1;"""
if old not in text:
    raise SystemExit("parameter frame anchor not found")
text = text.replace(old, new, 1)

old = """            StatementKind::Binding {\n                binding,\n                initializer,\n            } => match self.eval_expression(initializer, frame)? {\n                Flow::Value(value) => {\n                    frame.insert(binding.id, Some(value));\n                    Ok(None)\n                }\n                flow => Ok(Some(flow)),\n            },\n            StatementKind::UninitializedBinding(binding) => {\n                frame.insert(binding.id, None);\n                Ok(None)\n            }\n            StatementKind::Assignment { target, value } => {"""
new = """            StatementKind::Binding {\n                binding,\n                initializer,\n            } => match self.eval_expression(initializer, frame)? {\n                Flow::Value(value) => {\n                    self.bind_runtime_slot(frame, binding, Some(value), binding.span)?;\n                    Ok(None)\n                }\n                flow => Ok(Some(flow)),\n            },\n            StatementKind::UninitializedBinding(binding) => {\n                if !binding.mutable {\n                    return Err(self.invariant(\n                        binding.span,\n                        "semantically accepted uninitialized binding is not mutable",\n                    ));\n                }\n                self.bind_runtime_slot(frame, binding, None, binding.span)?;\n                Ok(None)\n            }\n            StatementKind::Assignment { target, value } => {"""
if old not in text:
    raise SystemExit("binding frame anchor not found")
text = text.replace(old, new, 1)

old = """                        let Some(slot) = frame.get_mut(target) else {\n                            return Err(self.invariant(\n                                statement.span,\n                                format!(\n                                    "assignment target {} is absent from the frame",\n                                    target.index()\n                                ),\n                            ));\n                        };\n                        *slot = Some(value);\n                        Ok(None)"""
new = """                        let Some(slot) = frame.get_mut(target) else {\n                            return Err(self.invariant(\n                                statement.span,\n                                format!(\n                                    "assignment target {} is absent from the frame",\n                                    target.index()\n                                ),\n                            ));\n                        };\n                        if !slot.mutable {\n                            return Err(self.invariant(\n                                statement.span,\n                                format!(\n                                    "assignment target {} resolved to an immutable runtime slot",\n                                    target.index()\n                                ),\n                            ));\n                        }\n                        if !self.value_conforms_to_type(&value, &slot.ty) {\n                            return Err(self.invariant(\n                                value_span(value, statement.span),\n                                format!(\n                                    "assignment target {} received a runtime value that does not conform to slot type {}",\n                                    target.index(), slot.ty\n                                ),\n                            ));\n                        }\n                        slot.value = Some(value);\n                        Ok(None)"""
if old not in text:
    raise SystemExit("assignment slot anchor not found")
# Use statement span directly; helper placeholder is replaced below to avoid moving the HIR expression name.
new = new.replace("value_span(value, statement.span)", "statement.span")
text = text.replace(old, new, 1)

old = """                let Some(value) = slot else {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            "binding {} reached runtime before initialization",\n                            binding.index()\n                        ),\n                    ));\n                };\n                Ok(Flow::Value(value.clone()))"""
new = """                if &expression.ty != &slot.ty {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            "binding {} expression type {} does not match runtime slot type {}",\n                            binding.index(), expression.ty, slot.ty\n                        ),\n                    ));\n                }\n                let Some(value) = slot.value.as_ref() else {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            "binding {} reached runtime before initialization",\n                            binding.index()\n                        ),\n                    ));\n                };\n                if !self.value_conforms_to_type(value, &slot.ty) {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            "binding {} stored a runtime value that does not conform to slot type {}",\n                            binding.index(), slot.ty\n                        ),\n                    ));\n                }\n                Ok(Flow::Value(value.clone()))"""
if old not in text:
    raise SystemExit("binding read anchor not found")
text = text.replace(old, new, 1)

old = """                match (&arm.binding, payload) {\n                    (Some(binding), Some(payload)) => {\n                        frame.insert(binding.id, Some(*payload));\n                    }\n                    (None, None) => {}"""
new = """                match (&arm.binding, payload) {\n                    (Some(binding), Some(payload)) => {\n                        self.bind_runtime_slot(frame, binding, Some(*payload), arm.span)?;\n                    }\n                    (None, None) => {}"""
if old not in text:
    raise SystemExit("match payload binding anchor not found")
text = text.replace(old, new, 1)

anchor = """    fn value_conforms_to_type(&self, value: &Value, ty: &Type) -> bool {"""
helper = """    fn bind_runtime_slot(\n        &self,\n        frame: &mut Frame,\n        binding: &nova_sema::hir::Binding,\n        value: Option<Value>,\n        span: nova_source::Span,\n    ) -> Result<(), Diagnostic> {\n        if let Some(value) = value.as_ref() {\n            if !self.value_conforms_to_type(value, &binding.ty) {\n                return Err(self.invariant(\n                    span,\n                    format!(\n                        "binding `{}` received a runtime value that does not conform to declared type {}",\n                        binding.name, binding.ty\n                    ),\n                ));\n            }\n        }\n        if frame.contains_key(&binding.id) {\n            return Err(self.invariant(\n                span,\n                format!(\n                    "binding id {} is already present in the runtime frame",\n                    binding.id.index()\n                ),\n            ));\n        }\n        frame.insert(\n            binding.id,\n            RuntimeSlot {\n                ty: binding.ty.clone(),\n                mutable: binding.mutable,\n                value,\n            },\n        );\n        Ok(())\n    }\n\n    fn value_conforms_to_type(&self, value: &Value, ty: &Type) -> bool {"""
if anchor not in text:
    raise SystemExit("runtime conformance helper anchor not found")
text = text.replace(anchor, helper, 1)
lib.write_text(text)

tests = Path("crates/nova-interpreter/tests/frame_slot_integrity.rs")
tests.write_text(r'''use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "frame-slot.nv", text);
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
fn rejects_malformed_local_initializer_before_frame_storage() {
    let mut analyzed = analyze_text("fn main() -> Unit { let value: Int = 42; }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Binding { initializer, .. } = &mut main.body.statements[0].kind else {
        panic!("expected binding statement");
    };
    initializer.kind = ExpressionKind::Boolean(true);
    initializer.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("initializer drift must fail at storage");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_malformed_delayed_assignment_before_frame_storage() {
    let mut analyzed = analyze_text("fn main() -> Unit { var value: Int; value = 42; }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Assignment { value, .. } = &mut main.body.statements[1].kind else {
        panic!("expected assignment statement");
    };
    value.kind = ExpressionKind::Boolean(true);
    value.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("assignment drift must fail at storage");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_assignment_retargeted_to_immutable_runtime_slot() {
    let mut analyzed = analyze_text(
        "fn main() -> Unit { let fixed: Int = 1; var mutable: Int = 2; mutable = 3; }",
    );
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let fixed = match &main.body.statements[0].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("expected immutable binding"),
    };
    let StatementKind::Assignment { target, .. } = &mut main.body.statements[2].kind else {
        panic!("expected assignment statement");
    };
    *target = Some(fixed);

    let error = execute(&analyzed.program).expect_err("immutable slot write must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_duplicate_binding_identity_in_runtime_frame() {
    let mut analyzed = analyze_text("fn main() -> Unit { let left: Int = 1; let right: Int = 2; }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let left = match &main.body.statements[0].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("expected first binding"),
    };
    let StatementKind::Binding { binding, .. } = &mut main.body.statements[1].kind else {
        panic!("expected second binding");
    };
    binding.id = left;

    let error = execute(&analyzed.program).expect_err("duplicate binding id must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_match_payload_binding_type_drift_before_arm_execution() {
    let mut analyzed = analyze_text(
        "enum Maybe { Some(Int), None } fn main() -> Unit { match Maybe::Some(42) { Maybe::Some(value) => (), Maybe::None => (), }; }",
    );
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Expression(expression) = &mut main.body.statements[0].kind else {
        panic!("expected match expression statement");
    };
    let ExpressionKind::Match { arms, .. } = &mut expression.kind else {
        panic!("expected match expression");
    };
    let binding = arms[0].binding.as_mut().expect("payload binding");
    binding.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("payload binding type drift must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn valid_runtime_frame_storage_remains_executable() {
    let analyzed = analyze_text(
        "fn main() -> Int { var value: Int; value = 41; value = value + 1; value }",
    );
    let value = execute(&analyzed.program).expect("valid frame storage should execute");
    assert_eq!(value, Value::Int(42));
}
''')

readme = Path("README.md")
text = readme.read_text()
anchor = """Aggregate construction enforces the same invariant locally: each evaluated record
field must conform to its declaration slot type, and each enum payload must conform
to its selected variant payload type before the aggregate value is created. This
catches malformed HIR even when the aggregate never crosses a function boundary.
"""
addition = anchor + """
Runtime frames preserve the resolved binding contract too. Each slot records its
resolved type, mutability, and initialization state. Parameters, local bindings,
delayed `var` declarations, and match payload bindings reject duplicate binding
identities or non-conforming initial values; assignment additionally requires a
mutable slot and a conforming replacement value. Binding reads verify that the HIR
expression type and stored runtime value still agree with the slot contract. Any
such interpreter/HIR drift fails closed with `N4005`.
"""
if anchor not in text:
    raise SystemExit("README aggregate integrity anchor not found")
readme.write_text(text.replace(anchor, addition, 1))

roadmap = Path("docs/roadmap.md")
text = roadmap.read_text()
status_old = "**Status: twelve vertical slices implemented; execution surface remains small.**"
status_new = "**Status: thirteen vertical slices implemented; execution surface remains small.**"
if status_old not in text:
    raise SystemExit("roadmap Phase 3 status anchor not found")
text = text.replace(status_old, status_new, 1)
anchor = """- adversarial malformed-HIR regressions plus a valid local-aggregate control case
  lock the new construction boundary without changing accepted source behavior.

Next Phase 3 slices should deepen executable semantics without bypassing Phase 2
contracts:
"""
insertion = """- adversarial malformed-HIR regressions plus a valid local-aggregate control case
  lock the new construction boundary without changing accepted source behavior.

Implemented in the thirteenth Phase 3 slice:

- runtime frame entries now retain each resolved binding's type, mutability, and
  initialization state instead of storing only an optional untyped runtime value;
- parameters, ordinary locals, delayed `var` declarations, and match payload
  bindings share one fail-closed slot-construction path that rejects duplicate
  binding identities and non-conforming initial values;
- assignment verifies that its resolved target exists, remains mutable, and accepts
  the replacement runtime value under the slot's declared type before mutation;
- binding reads verify both HIR expression-type agreement and recursive runtime-value
  conformance with the slot contract; and
- malformed-HIR regressions cover initializer drift, delayed assignment drift,
  immutable retargeting, duplicate binding identity, and match-payload binding drift,
  while a valid mutation control case locks accepted execution unchanged.

Next Phase 3 slices should deepen executable semantics without bypassing Phase 2
contracts:
"""
if anchor not in text:
    raise SystemExit("roadmap Phase 3 insertion anchor not found")
roadmap.write_text(text.replace(anchor, insertion, 1))
