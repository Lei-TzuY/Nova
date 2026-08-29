from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
ANALYZER = ROOT / "crates/nova-sema/src/analyzer.rs"
CONDITION = ROOT / "crates/nova-sema/src/constant_condition.rs"
INTERPRETER = ROOT / "crates/nova-interpreter/src/lib.rs"
CLI = ROOT / "crates/nova-cli/tests/cli.rs"
README = ROOT / "README.md"
CONSTITUTION = ROOT / "docs/language-constitution.md"
ROADMAP = ROOT / "docs/roadmap.md"


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


def apply_fix() -> None:
    text = ANALYZER.read_text()
    text = replace_once(
        text,
        "if other.is_never() || matches!(other, Type::Int | Type::Bool) {",
        "if other.is_never() || matches!(other, Type::Int | Type::Bool | Type::Unit) {",
        "Never equality compatibility",
    )
    text = text.replace(
        "equality requires Int or Bool operands, found {} and {}",
        "equality requires Int, Bool, or Unit operands, found {} and {}",
    )
    text = replace_once(
        text,
        "let primitive = matches!(left.ty, Type::Int | Type::Bool);\n        if primitive && left.ty == right.ty {",
        "let comparable = matches!(left.ty, Type::Int | Type::Bool | Type::Unit);\n        if comparable && left.ty == right.ty {",
        "equality comparable types",
    )
    text = text.replace(
        "equality requires matching Int or Bool operands, found {} and {}",
        "equality requires matching Int, Bool, or Unit operands, found {} and {}",
    )
    ANALYZER.write_text(text)

    text = CONDITION.read_text()
    text = replace_once(
        text,
        '''                (Type::Bool, Type::Bool) => evaluate(left)? == evaluate(right)?,\n                _ => return None,\n''',
        '''                (Type::Bool, Type::Bool) => evaluate(left)? == evaluate(right)?,\n                (Type::Unit, Type::Unit) => {\n                    unit_literal(left)?;\n                    unit_literal(right)?;\n                    true\n                }\n                _ => return None,\n''',
        "constant Unit equality",
    )
    if "fn unit_literal" not in text:
        text += '''\nfn unit_literal(expression: &Expression) -> Option<()> {\n    matches!(expression.kind, ExpressionKind::Unit).then_some(())\n}\n'''
    CONDITION.write_text(text)

    text = INTERPRETER.read_text()
    text = replace_once(
        text,
        '''            (BinaryOperator::NotEqual, Value::Bool(left), Value::Bool(right)) => {\n                Ok(Value::Bool(left != right))\n            }\n            (BinaryOperator::And, Value::Bool(left), Value::Bool(right)) => {\n''',
        '''            (BinaryOperator::NotEqual, Value::Bool(left), Value::Bool(right)) => {\n                Ok(Value::Bool(left != right))\n            }\n            (BinaryOperator::Equal, Value::Unit, Value::Unit) => Ok(Value::Bool(true)),\n            (BinaryOperator::NotEqual, Value::Unit, Value::Unit) => Ok(Value::Bool(false)),\n            (BinaryOperator::And, Value::Bool(left), Value::Bool(right)) => {\n''',
        "runtime Unit equality",
    )
    INTERPRETER.write_text(text)

    text = CLI.read_text()
    text = replace_once(
        text,
        '''        "valid/unit.nv",\n        "valid/records.nv",\n''',
        '''        "valid/unit.nv",\n        "valid/unit-equality.nv",\n        "valid/records.nv",\n''',
        "CLI check fixture",
    )
    text = replace_once(
        text,
        '''        ("valid/unit.nv", "42\\n"),\n        ("valid/records.nv", "42\\n"),\n''',
        '''        ("valid/unit.nv", "42\\n"),\n        ("valid/unit-equality.nv", "true\\n"),\n        ("valid/records.nv", "42\\n"),\n''',
        "CLI run fixture",
    )
    CLI.write_text(text)

    text = README.read_text()
    if "Matching `Int`, `Bool`, and `Unit` values support `==` and `!=`" not in text:
        marker = '''Rejected unary and binary operators are also fail-closed. Concrete operand type\nmismatches produce Error HIR rather than a nominal success type, and flow facts from\na continuing rejected operator are rolled back. Non-continuation from an operand that\nmust be evaluated keeps `!` precedence; short-circuit operators retain their existing\nconditional right-hand evaluation rules.\n\n'''
        addition = marker + '''Matching `Int`, `Bool`, and `Unit` values support `==` and `!=`. `Unit` has a\nsingle runtime value, so Unit equality is always true and Unit inequality is always false\nonce both operands have evaluated normally. Record, enum, and function values remain\nnon-comparable. Closed-condition analysis recognizes only literal `()` equality; Unit\nlocals and calls remain dynamic and are still evaluated at runtime.\n\n'''
        text = replace_once(text, marker, addition, "README Unit equality")
    README.write_text(text)

    text = CONSTITUTION.read_text()
    if "Matching `Int`, `Bool`, and `Unit` values are equality-comparable" not in text:
        marker = '''The bootstrap frontend preserves decimal integer magnitudes through parsing and\n'''
        addition = '''Matching `Int`, `Bool`, and `Unit` values are equality-comparable with `==` and\n`!=`. Unit has exactly one bootstrap value, so two normally evaluated Unit values compare\nequal. Nominal records, enums, and function values remain non-comparable. Closed-condition\nreasoning may prove equality only for literal `()` operands; it does not erase evaluation\nof Unit-returning calls, names, or blocks.\n\n'''
        text = replace_once(text, marker, addition + marker, "constitution Unit equality")
    CONSTITUTION.write_text(text)

    text = ROADMAP.read_text()
    text = text.replace(
        "**Status: thirty-four vertical slices implemented; broader type-system work remains.**",
        "**Status: thirty-five vertical slices implemented; broader type-system work remains.**",
        1,
    )
    if "Implemented in the thirty-fifth Phase 2 slice:" not in text:
        marker = '''The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n'''
        section = '''Implemented in the thirty-fifth Phase 2 slice:\n\n- `Unit` joins `Int` and `Bool` as an equality-comparable bootstrap value type without\n  making nominal aggregates or function references comparable;\n- semantic `==` / `!=` accepts matching Unit operands, including parameters and call\n  results, while preserving existing Never/Error recovery precedence;\n- closed-condition reasoning recognizes only literal `()` equality and inequality, so\n  known Unit comparisons can refine reachability without treating Unit-returning calls,\n  locals, or blocks as compile-time values;\n- semantic regressions lock Unit comparison, literal-condition flow, dynamic-call\n  conservatism, and continued rejection of record/function equality; and\n- no parser, HIR, CFG shape, or semantic-inspection schema change is required.\n\n'''
        text = replace_once(text, marker, section + marker, "Phase 2 roadmap")

    text = text.replace(
        "**Status: seven vertical slices implemented; execution surface remains small.**",
        "**Status: eight vertical slices implemented; execution surface remains small.**",
        1,
    )
    if "Implemented in the eighth Phase 3 slice:" not in text:
        marker = '''Next Phase 3 slices should deepen executable semantics without bypassing Phase 2\ncontracts:\n'''
        section = '''Implemented in the eighth Phase 3 slice:\n\n- the interpreter executes equality and inequality for the existing first-class Unit\n  runtime value, yielding `true` for Unit equality and `false` for Unit inequality after\n  normal left-to-right operand evaluation;\n- Unit parameters and call results use the same equality path as literal `()`, with no\n  special source-only shortcut;\n- record, enum, and function values remain rejected by semantic analysis rather than\n  acquiring structural or identity equality accidentally; and\n- a CLI check/run fixture locks Unit equality end to end while preserving the existing\n  `main -> Int | Bool` entry-point contract.\n\n'''
        text = replace_once(text, marker, section + marker, "Phase 3 roadmap")
    ROADMAP.write_text(text)


if sys.argv[1:] != ["fix"]:
    raise SystemExit("usage: validate_unit_equality.py fix")
apply_fix()
