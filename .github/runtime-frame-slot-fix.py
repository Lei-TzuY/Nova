from pathlib import Path

lib = Path("crates/nova-interpreter/src/lib.rs")
text = lib.read_text()
old = '''        if frame.contains_key(&binding.id) {
            return Err(self.invariant(
                span,
                format!(
                    "binding id {} is already present in the runtime frame",
                    binding.id.index()
                ),
            ));
        }
        frame.insert(
            binding.id,
            RuntimeSlot {
                ty: binding.ty.clone(),
                mutable: binding.mutable,
                value,
            },
        );
        Ok(())
'''
new = '''        if let Some(slot) = frame.get_mut(&binding.id) {
            if slot.ty != binding.ty || slot.mutable != binding.mutable {
                return Err(self.invariant(
                    span,
                    format!(
                        "binding id {} was reused with runtime slot metadata that does not match its resolved binding",
                        binding.id.index()
                    ),
                ));
            }
            slot.value = value;
            return Ok(());
        }
        frame.insert(
            binding.id,
            RuntimeSlot {
                ty: binding.ty.clone(),
                mutable: binding.mutable,
                value,
            },
        );
        Ok(())
'''
if old not in text:
    raise SystemExit("slot duplicate anchor not found")
lib.write_text(text.replace(old, new, 1))

tests = Path("crates/nova-interpreter/tests/frame_slot_integrity.rs")
text = tests.read_text()
old = '''#[test]
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
'''
new = '''#[test]
fn rejects_binding_identity_reused_with_incompatible_slot_metadata() {
    let mut analyzed =
        analyze_text("fn main() -> Unit { let left: Int = 1; let right: Bool = true; }");
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

    let error = execute(&analyzed.program).expect_err("binding metadata alias must fail");
    assert_eq!(error.code, "N4005");
}
'''
if old not in text:
    raise SystemExit("duplicate binding test anchor not found")
text = text.replace(old, new, 1)
anchor = '''#[test]
fn valid_runtime_frame_storage_remains_executable() {
    let analyzed = analyze_text(
        "fn main() -> Int { var value: Int; value = 41; value = value + 1; value }",
    );
    let value = execute(&analyzed.program).expect("valid frame storage should execute");
    assert_eq!(value, Value::Int(42));
}
'''
replacement = '''#[test]
fn repeated_lexical_binding_execution_refreshes_the_same_runtime_slot() {
    let analyzed = analyze_text(
        "fn main() -> Int { var total: Int = 0; while total < 3 { let step: Int = 1; total = total + step; } total }",
    );
    let value = execute(&analyzed.program).expect("loop-local binding should re-enter cleanly");
    assert_eq!(value, Value::Int(3));
}

#[test]
fn valid_runtime_frame_storage_remains_executable() {
    let analyzed = analyze_text(
        "fn main() -> Int { var value: Int; value = 41; value = value + 1; value }",
    );
    let value = execute(&analyzed.program).expect("valid frame storage should execute");
    assert_eq!(value, Value::Int(42));
}
'''
if anchor not in text:
    raise SystemExit("valid storage test anchor not found")
tests.write_text(text.replace(anchor, replacement, 1))

readme = Path("README.md")
text = readme.read_text()
old = '''Runtime frames preserve the resolved binding contract too. Each slot records its
resolved type, mutability, and initialization state. Parameters, local bindings,
delayed `var` declarations, and match payload bindings reject duplicate binding
identities or non-conforming initial values; assignment additionally requires a
mutable slot and a conforming replacement value. Binding reads verify that the HIR
expression type and stored runtime value still agree with the slot contract. Any
such interpreter/HIR drift fails closed with `N4005`.
'''
new = '''Runtime frames preserve the resolved binding contract too. Each slot records its
resolved type, mutability, and initialization state. Parameters, local bindings,
delayed `var` declarations, and match payload bindings reject non-conforming initial
values or incompatible reuse of one binding identity; repeated execution of the same
lexical binding may refresh its slot only with identical type/mutability metadata.
Assignment additionally requires a mutable slot and a conforming replacement value.
Binding reads verify that the HIR expression type and stored runtime value still agree
with the slot contract. Any such interpreter/HIR drift fails closed with `N4005`.
'''
if old not in text:
    raise SystemExit("README frame paragraph anchor not found")
readme.write_text(text.replace(old, new, 1))

roadmap = Path("docs/roadmap.md")
text = roadmap.read_text()
old = '''- parameters, ordinary locals, delayed `var` declarations, and match payload
  bindings share one fail-closed slot-construction path that rejects duplicate
  binding identities and non-conforming initial values;
'''
new = '''- parameters, ordinary locals, delayed `var` declarations, and match payload
  bindings share one fail-closed slot-construction path that rejects non-conforming
  initial values or incompatible type/mutability reuse of one binding identity while
  allowing repeated execution of the same lexical binding;
'''
if old not in text:
    raise SystemExit("roadmap binding bullet anchor not found")
text = text.replace(old, new, 1)
old = '''- malformed-HIR regressions cover initializer drift, delayed assignment drift,
  immutable retargeting, duplicate binding identity, and match-payload binding drift,
  while a valid mutation control case locks accepted execution unchanged.
'''
new = '''- malformed-HIR regressions cover initializer drift, delayed assignment drift,
  immutable retargeting, incompatible binding-identity aliasing, and match-payload
  binding drift, while valid mutation and loop-local re-entry controls lock accepted
  execution unchanged.
'''
if old not in text:
    raise SystemExit("roadmap regression bullet anchor not found")
roadmap.write_text(text.replace(old, new, 1))
