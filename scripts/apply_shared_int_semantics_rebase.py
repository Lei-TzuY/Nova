from pathlib import Path


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:180]!r}")
    file.write_text(text.replace(old, new, 1))


def write(path: str, content: str) -> None:
    Path(path).write_text(content)


patch(
    "Cargo.toml",
    '''    "crates/nova-inspect",\n    "crates/nova-interpreter",\n''',
    '''    "crates/nova-inspect",\n    "crates/nova-int-semantics",\n    "crates/nova-interpreter",\n''',
)

patch(
    "crates/nova-sema/Cargo.toml",
    '''[dependencies]\nnova-diagnostics = { path = "../nova-diagnostics" }\n''',
    '''[dependencies]\nnova-diagnostics = { path = "../nova-diagnostics" }\nnova-int-semantics = { path = "../nova-int-semantics" }\n''',
)

patch(
    "crates/nova-interpreter/Cargo.toml",
    '''[dependencies]\nnova-diagnostics = { path = "../nova-diagnostics" }\n''',
    '''[dependencies]\nnova-diagnostics = { path = "../nova-diagnostics" }\nnova-int-semantics = { path = "../nova-int-semantics" }\n''',
)

write(
    "crates/nova-sema/src/constant_int.rs",
    '''use crate::hir::{Expression, ExpressionKind};\nuse nova_parser::ast::{BinaryOperator, UnaryOperator};\n\npub(crate) use nova_int_semantics::IntArithmeticError as ConstantIntError;\n\npub(crate) fn evaluate_unary(\n    operator: UnaryOperator,\n    operand: &Expression,\n) -> Option<Result<i64, ConstantIntError>> {\n    let operand = evaluate(operand)?;\n    match operator {\n        UnaryOperator::Negate => Some(operand.and_then(nova_int_semantics::negate)),\n        UnaryOperator::Not => None,\n    }\n}\n\npub(crate) fn evaluate_binary(\n    operator: BinaryOperator,\n    left: &Expression,\n    right: &Expression,\n) -> Option<Result<i64, ConstantIntError>> {\n    if !matches!(\n        operator,\n        BinaryOperator::Add\n            | BinaryOperator::Subtract\n            | BinaryOperator::Multiply\n            | BinaryOperator::Divide\n            | BinaryOperator::Remainder\n    ) {\n        return None;\n    }\n\n    let left = evaluate(left)?;\n    let right = evaluate(right)?;\n    Some(match (left, right) {\n        (Err(error), _) | (_, Err(error)) => Err(error),\n        (Ok(left), Ok(right)) => apply_binary(operator, left, right),\n    })\n}\n\nfn evaluate(expression: &Expression) -> Option<Result<i64, ConstantIntError>> {\n    match &expression.kind {\n        ExpressionKind::Integer(value) => Some(Ok(*value)),\n        ExpressionKind::Unary { operator, operand } => evaluate_unary(*operator, operand),\n        ExpressionKind::Binary {\n            operator,\n            left,\n            right,\n        } => evaluate_binary(*operator, left, right),\n        _ => None,\n    }\n}\n\nfn apply_binary(\n    operator: BinaryOperator,\n    left: i64,\n    right: i64,\n) -> Result<i64, ConstantIntError> {\n    match operator {\n        BinaryOperator::Add => nova_int_semantics::add(left, right),\n        BinaryOperator::Subtract => nova_int_semantics::subtract(left, right),\n        BinaryOperator::Multiply => nova_int_semantics::multiply(left, right),\n        BinaryOperator::Divide => nova_int_semantics::divide(left, right),\n        BinaryOperator::Remainder => nova_int_semantics::remainder(left, right),\n        _ => unreachable!("constant Int evaluator only dispatches arithmetic operators"),\n    }\n}\n''',
)

patch(
    "crates/nova-interpreter/src/lib.rs",
    '''mod int_semantics;\n\nuse int_semantics::IntArithmeticError;\nuse nova_diagnostics::Diagnostic;\n''',
    '''use nova_diagnostics::Diagnostic;\nuse nova_int_semantics as int_semantics;\nuse nova_int_semantics::IntArithmeticError;\n''',
)
Path("crates/nova-interpreter/src/int_semantics.rs").unlink()

patch(
    "README.md",
    '''dynamic overflow produces `N4002`; dynamic division or remainder by zero produces\n`N4003`. Arithmetic policy lives in explicit semantic/runtime contracts rather than\nbeing inferred from host operator edge cases. Recursive execution is guarded by a\nfinite active-call budget\n''',
    '''dynamic overflow produces `N4002`; dynamic division or remainder by zero produces\n`N4003`. The arithmetic truth table itself lives once in the dependency-free\n`nova-int-semantics` leaf crate; semantic preflight supplies only closed-HIR traversal\nand the interpreter supplies only runtime diagnostic mapping. This keeps both layers\non one checked signed-64 contract rather than duplicating host-edge-case policy.\nRecursive execution is guarded by a finite active-call budget\n''',
)
patch(
    "README.md",
    '''  -> nova-sema          typed HIR, nominal identity, resolution, typing, dataflow\n      -> nova-inspect       versioned facts and fail-closed JSON projection\n      -> nova-interpreter   deterministic checked, bounded HIR execution\n\nnova-cli                check/run/ast/inspect orchestration and presentation\n\nnova-diagnostics        shared structured diagnostic model and renderers\n''',
    '''  -> nova-sema          typed HIR, nominal identity, resolution, typing, dataflow\n      -> nova-inspect       versioned facts and fail-closed JSON projection\n      -> nova-interpreter   deterministic checked, bounded HIR execution\n\nnova-int-semantics      dependency-free checked signed-64 arithmetic truth table\nnova-cli                check/run/ast/inspect orchestration and presentation\n\nnova-diagnostics        shared structured diagnostic model and renderers\n''',
)

patch(
    "docs/language-constitution.md",
    '''Both layers use explicit arithmetic contracts rather than relying on host\ndebug/release behavior or undocumented edge cases. This is implementation evidence\nfor the numeric design, not yet a stable language-wide promise about numeric widths,\ndefaulting, conversions, or overflow policy for future backends.\n''',
    '''Both layers consume the same dependency-free `nova-int-semantics` arithmetic\ncontract: semantic analysis owns only closed-HIR discovery and diagnostic policy,\nwhile the interpreter owns only runtime evaluation and diagnostic mapping. The shared\nleaf therefore prevents static/runtime drift without making HIR or diagnostics part of\nthe numeric core. This is implementation evidence for the numeric design, not yet a\nstable language-wide promise about numeric widths, defaulting, conversions, or\noverflow policy for future backends.\n''',
)

patch(
    "docs/roadmap.md",
    '''**Status: twenty-three vertical slices implemented; broader type-system work remains.**\n''',
    '''**Status: twenty-four vertical slices implemented; broader type-system work remains.**\n''',
)
marker = '''The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n'''
addition = '''Implemented in the twenty-fourth Phase 2 slice:\n\n- checked signed-64 arithmetic policy moves into a dependency-free\n  `nova-int-semantics` leaf crate consumed by both semantic preflight and execution;\n- the shared contract uniquely owns overflow, zero-divisor, truncating division,\n  signed remainder, and `Int::MIN / -1` / `% -1` behavior plus their truth-table\n  tests, eliminating two previously duplicated implementations;\n- `nova-sema::constant_int` retains only HIR-closure traversal and operator dispatch,\n  while analyzer diagnostics remain separate and the new `nova-inspect` tooling\n  boundary continues to consume accepted HIR without owning arithmetic policy;\n- `nova-interpreter` retains only runtime value evaluation and N4002/N4003 diagnostic\n  mapping instead of carrying a private arithmetic copy;\n- the shared crate depends on no parser, HIR, source, diagnostic, inspection, or\n  interpreter type, preserving a one-way dependency graph and keeping numeric policy\n  representation-independent; and\n- structural gates plus the full existing static/runtime and semantic-inspection suites\n  verify the refactor is behavior-preserving while removing future drift risk.\n\n'''
patch("docs/roadmap.md", marker, addition + marker)
