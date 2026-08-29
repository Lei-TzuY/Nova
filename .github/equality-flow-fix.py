from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if text.count(old) != 1:
        raise SystemExit(f"{label}: expected exactly one anchor, found {text.count(old)}")
    return text.replace(old, new, 1)


path = Path("crates/nova-interpreter/src/lib.rs")
text = path.read_text()
old = '''        if matches!(operator, BinaryOperator::Equal | BinaryOperator::NotEqual)
            && !self.equality_types_match(&left.ty, &right.ty)
        {
'''
new = '''        if matches!(operator, BinaryOperator::Equal | BinaryOperator::NotEqual)
            && !left.ty.is_never()
            && !right.ty.is_never()
            && !self.equality_types_match(&left.ty, &right.ty)
        {
'''
text = replace_once(text, old, new, "equality value-path gate")
path.write_text(text)

path = Path("README.md")
text = path.read_text()
old = '''function, aggregate, or frame-storage boundary. Equality adds an operator-level
precondition as well: the two resolved operand types must satisfy the same shared
semantic comparability rule, including the declaration-wide payload-free requirement
for enums. Malformed HIR therefore cannot compare a payload-free variant of an enum
whose other variants carry payloads. Structured `return`, `break`, and `continue`
propagation is not a runtime value and therefore remains outside the value postcondition.
Any such interpreter/HIR contract drift fails closed with `N4005`.
'''
new = '''function, aggregate, or frame-storage boundary. Equality adds an operator-level
precondition on ordinary value-producing paths as well: when both operands can complete
normally, their resolved types must satisfy the same shared semantic comparability rule,
including the declaration-wide payload-free requirement for enums. Malformed HIR therefore
cannot compare a payload-free variant of an enum whose other variants carry payloads.
A `Never` operand still evaluates normally for structured `return`, `break`, or `continue`
propagation and never reaches the comparison itself. Any interpreter/HIR contract drift
on a value-producing equality path fails closed with `N4005`.
'''
text = replace_once(text, old, new, "README equality flow wording")
path.write_text(text)

path = Path("docs/roadmap.md")
text = path.read_text()
old = '''- equality execution now validates the resolved HIR operand types against the shared
  semantic equality-admissibility rule before evaluating either operand;
'''
new = '''- equality execution validates resolved HIR operand types against the shared semantic
  equality-admissibility rule whenever both operands can complete with ordinary values;
'''
text = replace_once(text, old, new, "roadmap equality gate wording")
old = '''- ordinary expression result conformance still validates each produced value, while the
  new operator precondition protects a distinct invariant: whether those types are legal
  operands for equality at all;
'''
new = '''- ordinary expression result conformance still validates each produced value, while the
  new operator precondition protects a distinct invariant: whether those types are legal
  operands for equality at all; `Never` operands deliberately bypass that value-only gate
  so structured return/break/continue propagation remains unchanged;
'''
text = replace_once(text, old, new, "roadmap noncontinuation wording")
path.write_text(text)
