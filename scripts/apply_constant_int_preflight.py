from pathlib import Path


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:140]!r}")
    file.write_text(text.replace(old, new, 1))


def write(path: str, content: str) -> None:
    Path(path).write_text(content)


patch(
    "crates/nova-sema/src/lib.rs",
    '''mod analyzer;\nmod flow_rules;\n''',
    '''mod analyzer;\nmod constant_int;\nmod flow_rules;\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''use crate::flow_rules::InitializationJoin;\n''',
    '''use crate::constant_int::{self, ConstantIntError};\nuse crate::flow_rules::InitializationJoin;\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''        if operand.ty.is_error() {\n            Type::Error\n        } else if expected_type_compatible(&operand.ty, &expected) {\n            expected\n        } else {\n            Type::Error\n        }\n    }\n\n    fn check_binary(\n''',
    '''        if operand.ty.is_error() {\n            Type::Error\n        } else if expected_type_compatible(&operand.ty, &expected) {\n            if self.constant_int_failure(constant_int::evaluate_unary(operator, operand), span) {\n                Type::Error\n            } else {\n                expected\n            }\n        } else {\n            Type::Error\n        }\n    }\n\n    fn check_binary(\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''            | BinaryOperator::Remainder => {\n                self.require_binary_operands(left, right, &Type::Int, span, "arithmetic operator");\n                strict_binary_result_type(&left.ty, &right.ty, &Type::Int, Type::Int)\n            }\n''',
    '''            | BinaryOperator::Remainder => {\n                self.require_binary_operands(left, right, &Type::Int, span, "arithmetic operator");\n                let ty = strict_binary_result_type(&left.ty, &right.ty, &Type::Int, Type::Int);\n                if ty == Type::Int\n                    && self.constant_int_failure(\n                        constant_int::evaluate_binary(operator, left, right),\n                        span,\n                    )\n                {\n                    Type::Error\n                } else {\n                    ty\n                }\n            }\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''    fn check_unary(\n''',
    '''    fn constant_int_failure(\n        &mut self,\n        result: Option<Result<i64, ConstantIntError>>,\n        span: Span,\n    ) -> bool {\n        let Some(Err(error)) = result else {\n            return false;\n        };\n        match error {\n            ConstantIntError::Overflow => self.diagnostics.push(\n                Diagnostic::error("N3031", "constant Int arithmetic overflow")\n                    .with_primary(\n                        span,\n                        "this closed literal arithmetic expression cannot produce a signed 64-bit Int",\n                    )\n                    .with_note(\n                        "successful constant arithmetic is validated but not folded; dynamic overflow remains runtime N4002",\n                    ),\n            ),\n            ConstantIntError::ZeroDivisor => self.diagnostics.push(\n                Diagnostic::error("N3032", "constant zero divisor")\n                    .with_primary(\n                        span,\n                        "this closed literal arithmetic expression divides or takes remainder by zero",\n                    )\n                    .with_note(\n                        "dynamic zero divisors remain runtime N4003",\n                    ),\n            ),\n        }\n        true\n    }\n\n    fn check_unary(\n''',
)

# Keep runtime diagnostics covered by making their operands dynamic.
write(
    "crates/nova-cli/tests/fixtures/runtime/overflow.nv",
    '''fn one() -> Int {\n    1\n}\n\nfn main() -> Int {\n    9223372036854775807 + one()\n}\n''',
)
write(
    "crates/nova-cli/tests/fixtures/runtime/min-negate-overflow.nv",
    '''fn minimum() -> Int {\n    -9223372036854775808\n}\n\nfn main() -> Int {\n    -minimum()\n}\n''',
)
write(
    "crates/nova-cli/tests/fixtures/runtime/divide-by-zero.nv",
    '''fn zero() -> Int {\n    0\n}\n\nfn main() -> Int {\n    10 / zero()\n}\n''',
)
write(
    "crates/nova-cli/tests/fixtures/runtime/remainder-by-zero.nv",
    '''fn zero() -> Int {\n    0\n}\n\nfn main() -> Int {\n    10 % zero()\n}\n''',
)
write(
    "crates/nova-cli/tests/fixtures/runtime/min-divide-overflow.nv",
    '''fn minimum() -> Int {\n    -9223372036854775808\n}\n\nfn minus_one() -> Int {\n    -1\n}\n\nfn main() -> Int {\n    minimum() / minus_one()\n}\n''',
)
write(
    "crates/nova-cli/tests/fixtures/runtime/min-remainder-overflow.nv",
    '''fn minimum() -> Int {\n    -9223372036854775808\n}\n\nfn minus_one() -> Int {\n    -1\n}\n\nfn main() -> Int {\n    minimum() % minus_one()\n}\n''',
)

patch(
    "crates/nova-cli/tests/cli.rs",
    '''        ("invalid/integer-magnitude-overflow.nv", "N1004"),\n''',
    '''        ("invalid/integer-magnitude-overflow.nv", "N1004"),\n        ("invalid/constant-overflow.nv", "N3031"),\n        ("invalid/constant-zero-divisor.nv", "N3032"),\n''',
)

patch(
    "crates/nova-interpreter/src/lib.rs",
    '''    fn rejects_checked_integer_overflow() {\n        let error = execute_text("fn main() -> Int { 9223372036854775807 + 1 }")\n            .expect_err("overflow must fail");\n        assert_eq!(error.code, "N4002");\n    }\n\n    #[test]\n    fn rejects_zero_divisor() {\n        for text in ["fn main() -> Int { 10 / 0 }", "fn main() -> Int { 10 % 0 }"] {\n            let error = execute_text(text).expect_err("zero divisor must fail");\n            assert_eq!(error.code, "N4003");\n        }\n    }\n''',
    '''    fn rejects_checked_integer_overflow() {\n        for text in [\n            "fn one() -> Int { 1 } fn main() -> Int { 9223372036854775807 + one() }",\n            "fn minimum() -> Int { -9223372036854775808 } fn main() -> Int { -minimum() }",\n            "fn minimum() -> Int { -9223372036854775808 } fn minus_one() -> Int { -1 } fn main() -> Int { minimum() / minus_one() }",\n            "fn minimum() -> Int { -9223372036854775808 } fn minus_one() -> Int { -1 } fn main() -> Int { minimum() % minus_one() }",\n        ] {\n            let error = execute_text(text).expect_err("dynamic overflow must fail at runtime");\n            assert_eq!(error.code, "N4002");\n        }\n    }\n\n    #[test]\n    fn rejects_zero_divisor() {\n        for text in [\n            "fn zero() -> Int { 0 } fn main() -> Int { 10 / zero() }",\n            "fn zero() -> Int { 0 } fn main() -> Int { 10 % zero() }",\n        ] {\n            let error = execute_text(text).expect_err("dynamic zero divisor must fail at runtime");\n            assert_eq!(error.code, "N4003");\n        }\n    }\n''',
)

patch(
    "README.md",
    '''`i64::MIN / -1` and `i64::MIN % -1` are classified as integer overflow. Overflow\nproduces `N4002`; division or remainder by zero produces `N4003`. Arithmetic\npolicy lives in a pure interpreter contract rather than being inferred from host\noperator edge cases. Recursive execution is guarded by a finite active-call budget\n''',
    '''`i64::MIN / -1` and `i64::MIN % -1` are classified as integer overflow. Before\nexecution, semantic analysis also preflights closed arithmetic trees made entirely\nfrom `Int` literals and arithmetic operators: statically certain overflow is `N3031`\nand a statically certain zero divisor is `N3032`. Successful constant arithmetic is\nnot folded, and any expression with a dynamic operand remains runtime checked. Such\ndynamic overflow produces `N4002`; dynamic division or remainder by zero produces\n`N4003`. Arithmetic policy lives in explicit semantic/runtime contracts rather than\nbeing inferred from host operator edge cases. Recursive execution is guarded by a\nfinite active-call budget\n''',
)

patch(
    "docs/language-constitution.md",
    '''operation succeeds. `Int::MIN / -1` and `Int::MIN % -1` are both overflow; zero\ndivisors are a separate runtime failure class. Arithmetic overflow, division by\nzero, and remainder by zero fail with structured runtime diagnostics, and the\ninterpreter routes these through a pure arithmetic contract rather than relying\non host debug/release behavior or undocumented edge cases. This is implementation\nevidence for the numeric design, not yet a stable language-wide promise about\nnumeric widths, defaulting, conversions, or overflow policy for future backends.\n''',
    '''operation succeeds. `Int::MIN / -1` and `Int::MIN % -1` are both overflow.\nSemantic analysis preflights only closed arithmetic trees composed entirely of\n`Int` literals and arithmetic operators: a provable overflow is `N3031`, while a\nprovable zero divisor is `N3032`. Successful trees are deliberately not folded,\nand names, calls, blocks, or other dynamic operands stop this preflight rather than\ntriggering general constant propagation. Dynamic arithmetic remains checked by the\ninterpreter: overflow is `N4002`, and division or remainder by zero is `N4003`.\nBoth layers use explicit arithmetic contracts rather than relying on host\ndebug/release behavior or undocumented edge cases. This is implementation evidence\nfor the numeric design, not yet a stable language-wide promise about numeric widths,\ndefaulting, conversions, or overflow policy for future backends.\n''',
)

patch(
    "docs/roadmap.md",
    '''**Status: twenty-one vertical slices implemented; broader type-system work remains.**\n''',
    '''**Status: twenty-two vertical slices implemented; broader type-system work remains.**\n''',
)

marker = '''The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n'''
addition = '''Implemented in the twenty-second Phase 2 slice:\n\n- semantic analysis preflights closed `Int` arithmetic trees made only from literal\n  values and arithmetic operators, without introducing name propagation, function\n  evaluation, block evaluation, or a general constant folder;\n- statically certain signed-64 overflow is rejected as `N3031`, while a statically\n  certain division/remainder zero divisor is rejected as `N3032`;\n- successful constant arithmetic preserves its original unary/binary HIR so semantic\n  validation does not change runtime evaluation shape or execution-step accounting;\n- dynamic operands stop preflight and retain the interpreter's `N4002` overflow and\n  `N4003` zero-divisor checks, keeping compile-time and runtime failure boundaries\n  independently exercised;\n- constant failures become Error-typed through the existing operator fail-closed\n  recovery path, so rejected source cannot export reachable flow facts; and\n- semantic integration tests plus CLI static/runtime fixtures lock overflow, zero\n  divisors, extreme signed edges, non-folding, and dynamic deferral end to end.\n\n'''
patch("docs/roadmap.md", marker, addition + marker)
