from pathlib import Path


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:120]!r}")
    file.write_text(text.replace(old, new, 1))


patch(
    "crates/nova-lexer/src/lib.rs",
    '''    /// A checked decimal integer literal.\n    Integer(i64),\n''',
    '''    /// A checked decimal integer magnitude. Signed meaning is assigned later.\n    Integer(u64),\n''',
)

patch(
    "crates/nova-lexer/src/lib.rs",
    '''        let value = text\n            .bytes()\n            .filter(|byte| *byte != b'_')\n            .try_fold(0_i64, |value, byte| {\n                value.checked_mul(10)?.checked_add(i64::from(byte - b'0'))\n            });\n\n        if let Some(value) = value {\n            self.tokens.push(Token {\n                kind: TokenKind::Integer(value),\n                span: self.span(start, self.offset),\n            });\n        } else {\n            self.diagnostics.push(\n                Diagnostic::error("N1004", "integer literal is out of range")\n                    .with_primary(\n                        self.span(start, self.offset),\n                        "the current frontend accepts magnitudes up to 9223372036854775807",\n                    )\n                    .with_note(\n                        "integer widths and overflow semantics are provisional in Nova v0.1",\n                    ),\n            );\n        }\n''',
    '''        const MAX_SIGNED_INT_MAGNITUDE: u64 = 1_u64 << 63;\n        let value = text\n            .bytes()\n            .filter(|byte| *byte != b'_')\n            .try_fold(0_u64, |value, byte| {\n                value.checked_mul(10)?.checked_add(u64::from(byte - b'0'))\n            })\n            .filter(|value| *value <= MAX_SIGNED_INT_MAGNITUDE);\n\n        if let Some(value) = value {\n            self.tokens.push(Token {\n                kind: TokenKind::Integer(value),\n                span: self.span(start, self.offset),\n            });\n        } else {\n            self.diagnostics.push(\n                Diagnostic::error("N1004", "integer literal magnitude is out of range")\n                    .with_primary(\n                        self.span(start, self.offset),\n                        "the bootstrap frontend accepts decimal magnitudes up to 9223372036854775808",\n                    )\n                    .with_note(\n                        "the largest accepted magnitude is reserved for the signed Int minimum under prefix `-`",\n                    ),\n            );\n        }\n''',
)

patch(
    "crates/nova-lexer/src/lib.rs",
    '''        let valid = lex(&source("9_223_372_036_854_775_807"));\n        assert_eq!(valid.tokens[0].kind, TokenKind::Integer(i64::MAX));\n        assert!(valid.diagnostics.is_empty());\n\n        let overflow = lex(&source("9223372036854775808"));\n        assert_eq!(overflow.diagnostics[0].code, "N1004");\n        assert_eq!(\n            overflow.tokens,\n            vec![super::Token {\n                kind: TokenKind::Eof,\n                span: source("9223372036854775808").eof_span(),\n            }]\n        );\n''',
    '''        let max = lex(&source("9_223_372_036_854_775_807"));\n        assert_eq!(max.tokens[0].kind, TokenKind::Integer(i64::MAX as u64));\n        assert!(max.diagnostics.is_empty());\n\n        let min_magnitude = lex(&source("9_223_372_036_854_775_808"));\n        assert_eq!(\n            min_magnitude.tokens[0].kind,\n            TokenKind::Integer(1_u64 << 63)\n        );\n        assert!(min_magnitude.diagnostics.is_empty());\n\n        let overflow_source = source("9223372036854775809");\n        let overflow = lex(&overflow_source);\n        assert_eq!(overflow.diagnostics[0].code, "N1004");\n        assert_eq!(\n            overflow.tokens,\n            vec![super::Token {\n                kind: TokenKind::Eof,\n                span: overflow_source.eof_span(),\n            }]\n        );\n''',
)

patch(
    "crates/nova-parser/src/ast.rs",
    '''    /// Checked decimal integer value.\n    Integer(i64),\n''',
    '''    /// Checked decimal integer magnitude. Signed Int meaning is assigned semantically.\n    Integer(u64),\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''struct Analyzer {\n''',
    '''const SIGNED_INT_MIN_MAGNITUDE: u64 = 1_u64 << 63;\n\nstruct Analyzer {\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''            ast::ExpressionKind::Integer(value) => (ExpressionKind::Integer(*value), Type::Int),\n''',
    '''            ast::ExpressionKind::Integer(value) => {\n                self.lower_integer_literal(*value, expression.span)\n            }\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''            ast::ExpressionKind::Unary { operator, operand } => {\n                let operator_entry_state = self.capture_reachable_state();\n''',
    '''            ast::ExpressionKind::Unary {\n                operator: UnaryOperator::Negate,\n                operand,\n            } if matches!(\n                operand.kind,\n                ast::ExpressionKind::Integer(SIGNED_INT_MIN_MAGNITUDE)\n            ) => (ExpressionKind::Integer(i64::MIN), Type::Int),\n            ast::ExpressionKind::Unary { operator, operand } => {\n                let operator_entry_state = self.capture_reachable_state();\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''    fn lower_expression_for_diagnostics(\n''',
    '''    fn lower_integer_literal(&mut self, magnitude: u64, span: Span) -> (ExpressionKind, Type) {\n        if magnitude <= i64::MAX as u64 {\n            return (ExpressionKind::Integer(magnitude as i64), Type::Int);\n        }\n\n        self.diagnostics.push(\n            Diagnostic::error("N3030", "integer literal is outside signed Int range")\n                .with_primary(\n                    span,\n                    "a positive Int literal cannot exceed 9223372036854775807",\n                )\n                .with_note(\n                    "write `-9223372036854775808` for the minimum signed 64-bit bootstrap Int",\n                ),\n        );\n        (ExpressionKind::Error, Type::Error)\n    }\n\n    fn lower_expression_for_diagnostics(\n''',
)

patch(
    "crates/nova-cli/tests/cli.rs",
    '''        "valid/enums-match.nv",\n''',
    '''        "valid/enums-match.nv",\n        "valid/int-boundaries.nv",\n''',
)

patch(
    "crates/nova-cli/tests/cli.rs",
    '''        ("valid/enums-match.nv", "42\\n"),\n''',
    '''        ("valid/enums-match.nv", "42\\n"),\n        ("valid/int-boundaries.nv", "-9223372036854775808\\n"),\n''',
)

patch(
    "crates/nova-cli/tests/cli.rs",
    '''        ("runtime/overflow.nv", "N4002"),\n''',
    '''        ("runtime/overflow.nv", "N4002"),\n        ("runtime/min-negate-overflow.nv", "N4002"),\n''',
)

patch(
    "crates/nova-cli/tests/cli.rs",
    '''        ("invalid/integer-overflow.nv", "N1004"),\n''',
    '''        ("invalid/integer-overflow.nv", "N3030"),\n        ("invalid/integer-magnitude-overflow.nv", "N1004"),\n''',
)

patch(
    "docs/grammar.md",
    '''Integer separators cannot lead, trail, or repeat. The frontend checks decimal\nconversion and rejects magnitudes above `9223372036854775807`; it never wraps or\ntruncates a literal.\n''',
    '''Integer separators cannot lead, trail, or repeat. Lexing preserves a checked decimal\nmagnitude and rejects magnitudes above `9223372036854775808`; it never wraps or\ntruncates. Semantic lowering interprets `Int` as signed for this bootstrap slice:\npositive literals end at `9223372036854775807`, while the otherwise-reserved\n`9223372036854775808` magnitude is accepted only under prefix `-`, giving the exact\nminimum value `-9223372036854775808`.\n''',
)

patch(
    "docs/language-constitution.md",
    '''Accepted integer literal magnitude is currently `0..=2^63-1`; unary `-` is a\nseparate expression, so the most-negative signed 64-bit value has no literal\nspelling in this subset.\n\nThe bootstrap interpreter provisionally executes `Int` as signed 64-bit values\n''',
    '''The bootstrap frontend preserves decimal integer magnitudes through parsing and\nassigns signed meaning during semantic lowering. Positive `Int` literals are\n`0..=2^63-1`; the magnitude `2^63` is reserved for prefix negation, so\n`-9223372036854775808` denotes the exact signed 64-bit minimum. A positive `2^63`\nexpression is rejected as semantic diagnostic `N3030`, and larger magnitudes are\nrejected lexically as `N1004`. No literal is wrapped or truncated.\n\nThe bootstrap interpreter provisionally executes `Int` as signed 64-bit values\n''',
)

patch(
    "README.md",
    '''For deterministic execution while the numeric design remains provisional, the\nbootstrap interpreter represents `Int` as signed 64-bit at runtime and uses\nchecked arithmetic. Overflow produces `N4002`; division or remainder by zero\n''',
    '''For deterministic execution while the numeric design remains provisional, the\nbootstrap frontend now covers the complete signed 64-bit literal endpoints: positive\nliterals end at `9223372036854775807`, while `-9223372036854775808` is normalized\nduring semantic lowering to the exact minimum `Int`. Positive `9223372036854775808`\nis `N3030`; any larger decimal magnitude is lexical `N1004`. The interpreter\nrepresents `Int` as signed 64-bit at runtime and uses checked arithmetic.\nOverflow produces `N4002`; division or remainder by zero\n''',
)

roadmap = "docs/roadmap.md"
marker = '''The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n'''
addition = '''Implemented in the twenty-first Phase 2 slice:\n\n- lexer and AST integer tokens preserve checked unsigned decimal magnitude through\n  parsing instead of prematurely forcing every literal through positive `i64`;\n- the bootstrap lexical ceiling becomes `2^63`, with larger magnitudes remaining\n  deterministic `N1004` errors and no wrapping or truncation;\n- semantic lowering accepts positive magnitudes only through `i64::MAX`, reports\n  `N3030` for positive `2^63`, and normalizes prefix `-2^63` to exact `i64::MIN`;\n- outer operations remain explicit HIR, so negating the minimum again still reaches\n  the interpreter's checked arithmetic and reports runtime `N4002`;\n- CLI fixtures cover both signed endpoints, semantic-vs-lexical overflow separation,\n  and minimum-value negation end to end; and\n- the change remains a bootstrap signed-64 contract, not a decision on future numeric\n  families, conversions, literal suffixes, or backend-wide overflow policy.\n\n'''
patch(roadmap, marker, addition + marker)
