from pathlib import Path


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:120]!r}")
    file.write_text(text.replace(old, new, 1))


patch(
    "crates/nova-interpreter/src/lib.rs",
    '''//! Deterministic bootstrap interpreter for semantically accepted Nova HIR.\n\nuse nova_diagnostics::Diagnostic;\n''',
    '''//! Deterministic bootstrap interpreter for semantically accepted Nova HIR.\n\nmod int_semantics;\n\nuse int_semantics::IntArithmeticError;\nuse nova_diagnostics::Diagnostic;\n''',
)

patch(
    "crates/nova-interpreter/src/lib.rs",
    '''            (UnaryOperator::Negate, Value::Int(value)) => value\n                .checked_neg()\n                .map(Value::Int)\n                .ok_or_else(|| self.overflow(expression)),\n''',
    '''            (UnaryOperator::Negate, Value::Int(value)) => {\n                self.int_result(int_semantics::negate(value), expression)\n            }\n''',
)

patch(
    "crates/nova-interpreter/src/lib.rs",
    '''            (BinaryOperator::Add, Value::Int(left), Value::Int(right)) => left\n                .checked_add(right)\n                .map(Value::Int)\n                .ok_or_else(|| self.overflow(expression)),\n            (BinaryOperator::Subtract, Value::Int(left), Value::Int(right)) => left\n                .checked_sub(right)\n                .map(Value::Int)\n                .ok_or_else(|| self.overflow(expression)),\n            (BinaryOperator::Multiply, Value::Int(left), Value::Int(right)) => left\n                .checked_mul(right)\n                .map(Value::Int)\n                .ok_or_else(|| self.overflow(expression)),\n            (BinaryOperator::Divide, Value::Int(_), Value::Int(0))\n            | (BinaryOperator::Remainder, Value::Int(_), Value::Int(0)) => {\n                Err(Diagnostic::error("N4003", "division by zero")\n                    .with_primary(expression.span, "zero divisor is not executable"))\n            }\n            (BinaryOperator::Divide, Value::Int(left), Value::Int(right)) => left\n                .checked_div(right)\n                .map(Value::Int)\n                .ok_or_else(|| self.overflow(expression)),\n            (BinaryOperator::Remainder, Value::Int(left), Value::Int(right)) => left\n                .checked_rem(right)\n                .map(Value::Int)\n                .ok_or_else(|| self.overflow(expression)),\n''',
    '''            (BinaryOperator::Add, Value::Int(left), Value::Int(right)) => {\n                self.int_result(int_semantics::add(left, right), expression)\n            }\n            (BinaryOperator::Subtract, Value::Int(left), Value::Int(right)) => {\n                self.int_result(int_semantics::subtract(left, right), expression)\n            }\n            (BinaryOperator::Multiply, Value::Int(left), Value::Int(right)) => {\n                self.int_result(int_semantics::multiply(left, right), expression)\n            }\n            (BinaryOperator::Divide, Value::Int(left), Value::Int(right)) => {\n                self.int_result(int_semantics::divide(left, right), expression)\n            }\n            (BinaryOperator::Remainder, Value::Int(left), Value::Int(right)) => {\n                self.int_result(int_semantics::remainder(left, right), expression)\n            }\n''',
)

patch(
    "crates/nova-interpreter/src/lib.rs",
    '''    fn overflow(&self, expression: &Expression) -> Diagnostic {\n''',
    '''    fn int_result(\n        &self,\n        result: Result<i64, IntArithmeticError>,\n        expression: &Expression,\n    ) -> Result<Value, Diagnostic> {\n        match result {\n            Ok(value) => Ok(Value::Int(value)),\n            Err(IntArithmeticError::Overflow) => Err(self.overflow(expression)),\n            Err(IntArithmeticError::ZeroDivisor) => Err(self.zero_divisor(expression)),\n        }\n    }\n\n    fn zero_divisor(&self, expression: &Expression) -> Diagnostic {\n        Diagnostic::error("N4003", "division by zero")\n            .with_primary(expression.span, "zero divisor is not executable")\n    }\n\n    fn overflow(&self, expression: &Expression) -> Diagnostic {\n''',
)

patch(
    "crates/nova-cli/tests/cli.rs",
    '''        "valid/int-boundaries.nv",\n''',
    '''        "valid/int-boundaries.nv",\n        "valid/int-division.nv",\n''',
)

patch(
    "crates/nova-cli/tests/cli.rs",
    '''        ("valid/int-boundaries.nv", "-9223372036854775808\\n"),\n''',
    '''        ("valid/int-boundaries.nv", "-9223372036854775808\\n"),\n        ("valid/int-division.nv", "-21\\n"),\n''',
)

patch(
    "crates/nova-cli/tests/cli.rs",
    '''        ("runtime/min-negate-overflow.nv", "N4002"),\n        ("runtime/divide-by-zero.nv", "N4003"),\n''',
    '''        ("runtime/min-negate-overflow.nv", "N4002"),\n        ("runtime/min-divide-overflow.nv", "N4002"),\n        ("runtime/min-remainder-overflow.nv", "N4002"),\n        ("runtime/divide-by-zero.nv", "N4003"),\n        ("runtime/remainder-by-zero.nv", "N4003"),\n''',
)

patch(
    "README.md",
    '''represents `Int` as signed 64-bit at runtime and uses checked arithmetic.\nOverflow produces `N4002`; division or remainder by zero\nproduces `N4003`. Recursive execution is guarded by a finite active-call budget\n''',
    '''represents `Int` as signed 64-bit at runtime and uses checked arithmetic.\nSigned division truncates the quotient toward zero; a non-zero remainder has the\nsame sign as the dividend and satisfies `a = (a / b) * b + (a % b)`. Both\n`i64::MIN / -1` and `i64::MIN % -1` are classified as integer overflow. Overflow\nproduces `N4002`; division or remainder by zero produces `N4003`. Arithmetic\npolicy lives in a pure interpreter contract rather than being inferred from host\noperator edge cases. Recursive execution is guarded by a finite active-call budget\n''',
)

patch(
    "docs/language-constitution.md",
    '''The bootstrap interpreter provisionally executes `Int` as signed 64-bit values\nwith checked arithmetic. Arithmetic overflow, division by zero, and remainder\nby zero fail with structured runtime diagnostics; execution never inherits\nhost debug/release overflow behavior. This is implementation evidence for the\nnumeric design, not yet a stable language-wide promise about numeric widths,\ndefaulting, conversions, or overflow policy for future backends.\n''',
    '''The bootstrap interpreter provisionally executes `Int` as signed 64-bit values\nwith checked arithmetic. Signed division truncates toward zero. The associated\nremainder has the dividend's sign when non-zero, has magnitude smaller than the\ndivisor's magnitude, and satisfies `a = (a / b) * b + (a % b)` whenever the\noperation succeeds. `Int::MIN / -1` and `Int::MIN % -1` are both overflow; zero\ndivisors are a separate runtime failure class. Arithmetic overflow, division by\nzero, and remainder by zero fail with structured runtime diagnostics, and the\ninterpreter routes these through a pure arithmetic contract rather than relying\non host debug/release behavior or undocumented edge cases. This is implementation\nevidence for the numeric design, not yet a stable language-wide promise about\nnumeric widths, defaulting, conversions, or overflow policy for future backends.\n''',
)

patch(
    "docs/roadmap.md",
    '''**Status: six vertical slices implemented; execution surface remains small.**\n''',
    '''**Status: seven vertical slices implemented; execution surface remains small.**\n''',
)

patch(
    "docs/roadmap.md",
    '''Next Phase 3 slices should deepen executable semantics without bypassing Phase 2\ncontracts:\n''',
    '''Implemented in the seventh Phase 3 slice:\n\n- bootstrap integer arithmetic policy is centralized in an interpreter-owned pure\n  `int_semantics` contract instead of scattering host `checked_*` behavior across\n  expression evaluation;\n- signed division is explicitly truncation toward zero and signed remainder is tied\n  to that quotient, carries the dividend's sign when non-zero, and obeys the usual\n  division identity for successful operations;\n- zero divisors are a distinct `ZeroDivisor` arithmetic failure mapped to runtime\n  `N4003`, while representability failures map to `Overflow` / `N4002`;\n- both `Int::MIN / -1` and `Int::MIN % -1` remain deliberate overflow edges rather\n  than accidental consequences of Rust's operators;\n- truth-table unit tests cover all sign combinations, zero divisors, extreme values,\n  the quotient/remainder identity, and checked add/subtract/multiply/negate; and\n- CLI fixtures lock negative division/remainder results plus both extreme overflow\n  and zero-divisor classes end to end without adding new syntax or numeric types.\n\nNext Phase 3 slices should deepen executable semantics without bypassing Phase 2\ncontracts:\n''',
)
