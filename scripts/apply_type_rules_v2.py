from pathlib import Path


def block(lines):
    return "\n".join(lines) + "\n"


def patch(path, old, new):
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:120]!r}")
    file.write_text(text.replace(old, new, 1))


patch(
    "crates/nova-sema/src/lib.rs",
    "mod analyzer;\n\npub mod hir;\n",
    "mod analyzer;\nmod type_rules;\n\npub mod hir;\n",
)

analyzer = Path("crates/nova-sema/src/analyzer.rs")
text = analyzer.read_text()
old = block([
    "use crate::hir::{",
    "    self, BindingId, EnumId, EnumType, ExpressionKind, FunctionId, FunctionType, MatchArm,",
    "    RecordFieldValue, RecordId, RecordType, StatementKind, Type,",
    "};",
    "use nova_diagnostics::{Diagnostic, LabelStyle};",
])
new = block([
    "use crate::hir::{",
    "    self, BindingId, EnumId, EnumType, ExpressionKind, FunctionId, FunctionType, MatchArm,",
    "    RecordFieldValue, RecordId, RecordType, StatementKind, Type,",
    "};",
    "use crate::type_rules::{",
    "    JoinObservation, TypeJoin, expected_type_compatible, strict_binary_result_type,",
    "};",
    "use nova_diagnostics::{Diagnostic, LabelStyle};",
])
if text.count(old) != 1:
    raise SystemExit("analyzer import block did not match exactly once")
text = text.replace(old, new, 1)

branch_old = """    fn join_branch_types(
        &mut self,
        then_type: &Type,
        then_span: Span,
        else_type: &Type,
        else_span: Span,
    ) -> Type {
        if then_type.is_never() {
            return else_type.clone();
        }
        if else_type.is_never() {
            return then_type.clone();
        }
        if then_type.is_error() {
            return else_type.clone();
        }
        if else_type.is_error() {
            return then_type.clone();
        }
        if then_type == else_type {
            return then_type.clone();
        }

        self.diagnostics.push(
            Diagnostic::error(\"N3004\", \"type mismatch\")
                .with_primary(
                    else_span,
                    format!(\"else branch has type {else_type}, expected {then_type}\"),
                )
                .with_secondary(then_span, format!(\"then branch has type {then_type}\")),
        );
        Type::Error
    }
"""
branch_new = """    fn join_branch_types(
        &mut self,
        then_type: &Type,
        then_span: Span,
        else_type: &Type,
        else_span: Span,
    ) -> Type {
        let mut join = TypeJoin::default();
        let _ = join.observe(then_type);
        if let JoinObservation::Mismatch { expected, found } = join.observe(else_type) {
            self.diagnostics.push(
                Diagnostic::error(\"N3004\", \"type mismatch\")
                    .with_primary(
                        else_span,
                        format!(\"else branch has type {found}, expected {expected}\"),
                    )
                    .with_secondary(then_span, format!(\"then branch has type {expected}\")),
            );
        }
        join.finish()
    }
"""
if text.count(branch_old) != 1:
    raise SystemExit("join_branch_types block did not match exactly once")
text = text.replace(branch_old, branch_new, 1)

match_old = """    fn join_match_arm_types(&mut self, arms: &[(Type, Span)]) -> Type {
        let mut expected = None::<(Type, Span)>;
        let mut saw_error = false;
        let mut mismatch = false;

        for (ty, span) in arms {
            if ty.is_never() {
                continue;
            }
            if ty.is_error() {
                saw_error = true;
                continue;
            }
            if let Some((expected_type, expected_span)) = &expected {
                if ty != expected_type {
                    self.diagnostics.push(
                        Diagnostic::error(\"N3004\", \"type mismatch\")
                            .with_primary(
                                *span,
                                format!(\"match arm has type {ty}, expected {expected_type}\"),
                            )
                            .with_secondary(
                                *expected_span,
                                format!(\"first continuing arm has type {expected_type}\"),
                            ),
                    );
                    mismatch = true;
                }
            } else {
                expected = Some((ty.clone(), *span));
            }
        }

        if mismatch || (expected.is_none() && saw_error) {
            Type::Error
        } else if let Some((ty, _)) = expected {
            ty
        } else {
            Type::Never
        }
    }
"""
match_new = """    fn join_match_arm_types(&mut self, arms: &[(Type, Span)]) -> Type {
        let mut join = TypeJoin::default();
        let mut anchor_span = None;

        for (ty, span) in arms {
            match join.observe(ty) {
                JoinObservation::Anchor(_) => anchor_span = Some(*span),
                JoinObservation::Mismatch { expected, found } => {
                    self.diagnostics.push(
                        Diagnostic::error(\"N3004\", \"type mismatch\")
                            .with_primary(
                                *span,
                                format!(\"match arm has type {found}, expected {expected}\"),
                            )
                            .with_secondary(
                                anchor_span.expect(\"a mismatch requires a concrete anchor\"),
                                format!(\"first continuing arm has type {expected}\"),
                            ),
                    );
                }
                JoinObservation::Never
                | JoinObservation::Error
                | JoinObservation::Compatible => {}
            }
        }

        join.finish()
    }
"""
if text.count(match_old) != 1:
    raise SystemExit("join_match_arm_types block did not match exactly once")
text = text.replace(match_old, match_new, 1)

helper_old = """fn types_compatible(actual: &Type, expected: &Type) -> bool {
    actual.is_error() || expected.is_error() || actual.is_never() || actual == expected
}

fn binary_result_type(left: &hir::Expression, right: &hir::Expression, success: Type) -> Type {
    if left.ty.is_never() || right.ty.is_never() {
        Type::Never
    } else if left.ty.is_error() || right.ty.is_error() {
        Type::Error
    } else {
        success
    }
}

"""
if text.count(helper_old) != 1:
    raise SystemExit("legacy type helper block did not match exactly once")
text = text.replace(helper_old, "", 1)

if text.count("types_compatible(") == 0:
    raise SystemExit("expected legacy compatibility call sites")
text = text.replace("types_compatible(", "expected_type_compatible(")
for result in ("Int", "Bool"):
    old_call = f"binary_result_type(left, right, Type::{result})"
    new_call = f"strict_binary_result_type(&left.ty, &right.ty, Type::{result})"
    if old_call not in text:
        raise SystemExit(f"missing binary result call for {result}")
    text = text.replace(old_call, new_call)
if "fn types_compatible(" in text or "fn binary_result_type(" in text:
    raise SystemExit("legacy helper definition remains")
if "binary_result_type(left, right," in text:
    raise SystemExit("legacy binary result call remains")
analyzer.write_text(text)

roadmap = "docs/roadmap.md"
patch(
    roadmap,
    "**Status: thirteen vertical slices implemented; broader type-system work remains.**\n",
    "**Status: fourteen vertical slices implemented; broader type-system work remains.**\n",
)
marker = block([
    "The next Phase 2 slices should address semantic depth rather than widen syntax",
    "prematurely. In particular:",
])
addition = block([
    "Implemented in the fourteenth Phase 2 slice:",
    "",
    "- recovery-aware expected-type compatibility is centralized in an internal pure",
    "  `type_rules` contract instead of being duplicated inside analyzer helpers;",
    "- strict binary result typing explicitly records that reachable non-continuation",
    "  (`!`) dominates recovery errors while ordinary successful operands produce the",
    "  operator's declared result type;",
    "- `if` and exhaustive `match` now share one `TypeJoin` state machine in which `!` is",
    "  bottom, `<error>` is recovery-neutral when a concrete alternative exists, and the",
    "  first concrete continuing type remains the diagnostic anchor;",
    "- analyzer-owned source spans and N3004 wording remain unchanged while pure type",
    "  decisions are separated from diagnostic rendering; and",
    "- truth-table tests lock primitive, `Unit`, nominal, `!`, `<error>`, compatible, and",
    "  mismatched joins so later type-system work has an executable semantic contract.",
    "",
])
patch(roadmap, marker, addition + marker)
