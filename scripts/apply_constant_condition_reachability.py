from pathlib import Path


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:200]!r}")
    file.write_text(text.replace(old, new, 1))


def write(path: str, content: str) -> None:
    file = Path(path)
    file.parent.mkdir(parents=True, exist_ok=True)
    file.write_text(content)


write(
    "crates/nova-sema/src/constant_condition.rs",
    '''use crate::constant_int;\nuse crate::hir::{Expression, ExpressionKind, Type};\nuse nova_parser::ast::{BinaryOperator, UnaryOperator};\n\n/// Evaluates only side-effect-free, closed bootstrap conditions whose value is\n/// already determined by literal Bool/Int operations. The HIR is never folded.\npub(crate) fn evaluate(expression: &Expression) -> Option<bool> {\n    if expression.ty != Type::Bool {\n        return None;\n    }\n\n    match &expression.kind {\n        ExpressionKind::Boolean(value) => Some(*value),\n        ExpressionKind::Unary {\n            operator: UnaryOperator::Not,\n            operand,\n        } => evaluate(operand).map(|value| !value),\n        ExpressionKind::Binary {\n            operator,\n            left,\n            right,\n        } => evaluate_binary(*operator, left, right),\n        _ => None,\n    }\n}\n\nfn evaluate_binary(\n    operator: BinaryOperator,\n    left: &Expression,\n    right: &Expression,\n) -> Option<bool> {\n    match operator {\n        BinaryOperator::And => match evaluate(left) {\n            Some(false) => Some(false),\n            Some(true) => evaluate(right),\n            None => None,\n        },\n        BinaryOperator::Or => match evaluate(left) {\n            Some(true) => Some(true),\n            Some(false) => evaluate(right),\n            None => None,\n        },\n        BinaryOperator::Equal | BinaryOperator::NotEqual => {\n            let equal = match (&left.ty, &right.ty) {\n                (Type::Int, Type::Int) => int_value(left)? == int_value(right)?,\n                (Type::Bool, Type::Bool) => evaluate(left)? == evaluate(right)?,\n                _ => return None,\n            };\n            Some(if operator == BinaryOperator::Equal {\n                equal\n            } else {\n                !equal\n            })\n        }\n        BinaryOperator::Less => Some(int_value(left)? < int_value(right)?),\n        BinaryOperator::LessEqual => Some(int_value(left)? <= int_value(right)?),\n        BinaryOperator::Greater => Some(int_value(left)? > int_value(right)?),\n        BinaryOperator::GreaterEqual => Some(int_value(left)? >= int_value(right)?),\n        _ => None,\n    }\n}\n\nfn int_value(expression: &Expression) -> Option<i64> {\n    constant_int::evaluate(expression)?.ok()\n}\n''',
)

patch(
    "crates/nova-sema/src/lib.rs",
    '''mod analyzer;\nmod constant_int;\nmod flow_rules;\n''',
    '''mod analyzer;\nmod constant_condition;\nmod constant_int;\nmod flow_rules;\n''',
)

patch(
    "crates/nova-sema/src/constant_int.rs",
    '''fn evaluate(expression: &Expression) -> Option<Result<i64, ConstantIntError>> {\n''',
    '''pub(crate) fn evaluate(expression: &Expression) -> Option<Result<i64, ConstantIntError>> {\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''                let left_literal = match &left.kind {\n                    ExpressionKind::Boolean(value) => Some(*value),\n                    _ => None,\n                };\n''',
    '''                let left_literal = crate::constant_condition::evaluate(&left);\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''                let condition_literal = match &condition.kind {\n                    ExpressionKind::Boolean(value) => Some(*value),\n                    _ => None,\n                };\n''',
    '''                let condition_literal = crate::constant_condition::evaluate(&condition);\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''                let post_condition_scopes = self.scopes.clone();\n                let guaranteed_entry = matches!(&condition.kind, ExpressionKind::Boolean(true));\n                self.loop_stack.push(LoopContext {\n                    visible_scope_count: self.scopes.len(),\n                    break_states: Vec::new(),\n                });\n                let body = self.lower_block(body, return_type, true);\n''',
    '''                let post_condition_scopes = self.scopes.clone();\n                let condition_literal = crate::constant_condition::evaluate(&condition);\n                let guaranteed_entry = condition_literal == Some(true);\n                self.loop_stack.push(LoopContext {\n                    visible_scope_count: self.scopes.len(),\n                    break_states: Vec::new(),\n                });\n                let body = if condition_literal == Some(false) {\n                    self.lower_block_for_diagnostics(body, return_type, true)\n                } else {\n                    self.lower_block(body, return_type, true)\n                };\n''',
)

write(
    "crates/nova-cli/tests/fixtures/valid/constant-condition-flow.nv",
    '''fn main() -> Int {\n    var value: Int;\n\n    if 1 + 1 == 2 {\n        value = 40;\n        ()\n    } else {\n        ()\n    };\n\n    (2 < 1) || {\n        value = value + 2;\n        true\n    };\n\n    while 2 > 3 {\n        1 / 0;\n    }\n\n    value\n}\n''',
)

patch(
    "crates/nova-cli/tests/cli.rs",
    '''        "valid/literal-if-flow.nv",\n        "valid/literal-match-flow.nv",\n''',
    '''        "valid/literal-if-flow.nv",\n        "valid/constant-condition-flow.nv",\n        "valid/literal-match-flow.nv",\n''',
)
patch(
    "crates/nova-cli/tests/cli.rs",
    '''        ("valid/literal-if-flow.nv", "42\\n"),\n        ("valid/literal-match-flow.nv", "42\\n"),\n''',
    '''        ("valid/literal-if-flow.nv", "42\\n"),\n        ("valid/constant-condition-flow.nv", "42\\n"),\n        ("valid/literal-match-flow.nv", "42\\n"),\n''',
)

patch(
    "README.md",
    '''Source lowered only for\ndiagnostics because control flow proves it unreachable does not manufacture these\nexecution-failure diagnostics. Successful constant arithmetic is not folded, and any\nexpression with a dynamic operand remains runtime checked. Such\n''',
    '''Source lowered only for\ndiagnostics because control flow proves it unreachable does not manufacture these\nexecution-failure diagnostics. The same side-effect-free literal Bool/Int evaluator\nmay determine `if`, `while`, and short-circuit reachability from closed comparisons\nand Boolean operations; this changes flow analysis only and never folds the retained\nHIR. Names, calls, blocks, aggregates, and other dynamic operands stop the proof.\nSuccessful constant arithmetic is not folded, and any expression with a dynamic\noperand remains runtime checked. Such\n''',
)

patch(
    "docs/language-constitution.md",
    '''Source lowered only for diagnostics on a statically\nunreachable path is excluded from these execution-failure diagnostics. Successful\ntrees are deliberately not folded, and names, calls, blocks, or other dynamic\noperands stop this preflight rather than triggering general constant propagation.\n''',
    '''Source lowered only for diagnostics on a statically\nunreachable path is excluded from these execution-failure diagnostics. Semantic\nreachability may also evaluate closed, side-effect-free Bool/Int literal expressions\nfor `if`, `while`, and `&&`/`||`, including checked arithmetic feeding comparisons.\nThis evaluator never propagates names, executes calls or blocks, inspects aggregates,\nor folds the retained HIR. A `while` condition proven false therefore lowers its body\nonly for static diagnostics, while a proven true condition participates in the same\nguaranteed-loop reasoning previously reserved for literal `true`. Successful arithmetic\ntrees are deliberately not folded, and dynamic operands stop these proofs rather than\ntriggering general constant propagation.\n''',
)

patch(
    "docs/roadmap.md",
    '''**Status: twenty-four vertical slices implemented; broader type-system work remains.**\n''',
    '''**Status: twenty-five vertical slices implemented; broader type-system work remains.**\n''',
)
marker = '''The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n'''
addition = '''Implemented in the twenty-fifth Phase 2 slice:\n\n- semantic reachability gains a pure closed-condition evaluator over already typed HIR,\n  recognizing Bool literals, Boolean negation/short-circuiting, and Int/Bool equality\n  or Int ordering when all required operands are side-effect-free known values;\n- checked literal arithmetic feeding those comparisons reuses the existing constant-Int\n  evaluator and shared `nova-int-semantics` policy instead of inventing another numeric\n  implementation;\n- `if`, `while`, and `&&`/`||` share the derived truth result, extending earlier direct-\n  literal flow precision without propagating names or executing calls, blocks, matches,\n  field access, or aggregate construction;\n- a loop condition proven false lowers its body in diagnostic-only mode under a lexical\n  loop context, so dead execution failures such as `1 / 0` do not manufacture N3032\n  while `break`/`continue` remain statically legal and ordinary type/name diagnostics run;\n- a condition proven true extends guaranteed-loop/noncontinuation reasoning beyond raw\n  `true`, and derived short-circuit truths control optional RHS dataflow exactly as the\n  interpreter does; and\n- semantic regressions plus a CLI check/run fixture lock flow precision, dead-path\n  execution-diagnostic suppression, dynamic-boundary conservatism, and HIR non-folding.\n\n'''
patch("docs/roadmap.md", marker, addition + marker)
