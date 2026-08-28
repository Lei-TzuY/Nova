from pathlib import Path


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:120]!r}")
    file.write_text(text.replace(old, new, 1))


analyzer = Path("crates/nova-sema/src/analyzer.rs")
text = analyzer.read_text()
old = '''    fn lower_record_literal(
        &mut self,
        name: &ast::Name,
        fields: &[ast::RecordLiteralField],
        return_type: &Type,
        _span: Span,
    ) -> (ExpressionKind, Type) {
        let Some(symbol) = self.types.get(&name.text).copied() else {
            let mut can_continue = true;
            for field in fields {
                let value = if can_continue {
                    self.lower_expression(&field.value, return_type)
                } else {
                    self.lower_expression_for_diagnostics(&field.value, return_type)
                };
                if can_continue && value.ty.is_never() {
                    can_continue = false;
                }
            }
            self.diagnostics.push(
                Diagnostic::error("N3001", "unknown type")
                    .with_primary(name.span, format!("unknown record type `{}`", name.text)),
            );
            return (ExpressionKind::Error, Type::Error);
        };
        let TypeDefinition::Record(record_id) = symbol.definition else {
            let mut can_continue = true;
            for field in fields {
                let value = if can_continue {
                    self.lower_expression(&field.value, return_type)
                } else {
                    self.lower_expression_for_diagnostics(&field.value, return_type)
                };
                if can_continue && value.ty.is_never() {
                    can_continue = false;
                }
            }
            self.diagnostics.push(
                Diagnostic::error("N3004", "type mismatch")
                    .with_primary(
                        name.span,
                        format!("`{}` is an enum, not a record", name.text),
                    )
                    .with_secondary(symbol.span, "enum declared here"),
            );
            return (ExpressionKind::Error, Type::Error);
        };
        let definition = self.record_definitions[record_id.index()].clone();
        let aggregate_entry_state = self.capture_reachable_state();
'''
new = '''    fn lower_record_literal(
        &mut self,
        name: &ast::Name,
        fields: &[ast::RecordLiteralField],
        return_type: &Type,
        _span: Span,
    ) -> (ExpressionKind, Type) {
        let aggregate_entry_state = self.capture_reachable_state();
        let Some(symbol) = self.types.get(&name.text).copied() else {
            let contains_never = self.lower_rejected_record_fields(fields, return_type);
            self.diagnostics.push(
                Diagnostic::error("N3001", "unknown type")
                    .with_primary(name.span, format!("unknown record type `{}`", name.text)),
            );
            let ty = if contains_never {
                Type::Never
            } else {
                self.restore_reachable_state(aggregate_entry_state);
                Type::Error
            };
            return (ExpressionKind::Error, ty);
        };
        let TypeDefinition::Record(record_id) = symbol.definition else {
            let contains_never = self.lower_rejected_record_fields(fields, return_type);
            self.diagnostics.push(
                Diagnostic::error("N3004", "type mismatch")
                    .with_primary(
                        name.span,
                        format!("`{}` is an enum, not a record", name.text),
                    )
                    .with_secondary(symbol.span, "enum declared here"),
            );
            let ty = if contains_never {
                Type::Never
            } else {
                self.restore_reachable_state(aggregate_entry_state);
                Type::Error
            };
            return (ExpressionKind::Error, ty);
        };
        let definition = self.record_definitions[record_id.index()].clone();
'''
if text.count(old) != 1:
    raise SystemExit("lower_record_literal head did not match exactly once")
text = text.replace(old, new, 1)

marker = '''    fn lower_record_literal(
'''
helper = '''    fn lower_rejected_record_fields(
        &mut self,
        fields: &[ast::RecordLiteralField],
        return_type: &Type,
    ) -> bool {
        let mut can_continue = true;
        let mut contains_never = false;
        for field in fields {
            let value = if can_continue {
                self.lower_expression(&field.value, return_type)
            } else {
                self.lower_expression_for_diagnostics(&field.value, return_type)
            };
            contains_never |= value.ty.is_never();
            if can_continue && value.ty.is_never() {
                can_continue = false;
            }
        }
        contains_never
    }

'''
if text.count(marker) != 1:
    raise SystemExit("lower_record_literal marker did not match exactly once")
text = text.replace(marker, helper + marker, 1)
analyzer.write_text(text)

patch(
    "README.md",
    "Rejected aggregate construction is fail-closed for continuing flow recovery. A\nrecord with invalid field shape/type or an enum constructor with invalid payload\nshape/type may still emit deterministic child diagnostics, but assignments or loop\n",
    "Rejected aggregate construction is fail-closed for continuing flow recovery. A\nrecord with an unresolved/non-record target or invalid field shape/type, or an enum\nconstructor with invalid target/payload shape/type, may still emit deterministic\nchild diagnostics, but assignments or loop\n",
)

patch(
    "docs/language-constitution.md",
    "Continuing record or enum construction that is itself rejected by structural or\npayload/field type validation is fail-closed for flow recovery: assignments and\n",
    "Continuing record or enum construction that is itself rejected by type-head,\nstructural, or payload/field type validation is fail-closed for flow recovery:\nassignments and\n",
)

roadmap = "docs/roadmap.md"
patch(
    roadmap,
    "**Status: fifteen vertical slices implemented; broader type-system work remains.**\n",
    "**Status: sixteen vertical slices implemented; broader type-system work remains.**\n",
)
marker = "The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n"
addition = '''Implemented in the sixteenth Phase 2 slice:

- record construction now captures reachable state before resolving its nominal
  target, matching the fail-closed policy already applied to enum constructors;
- unresolved record names and enum-as-record category errors still lower field
  expressions in written order for deterministic diagnostics, but continuing
  assignments and loop-exit facts from the rejected construction are rolled back;
- a field expression that is already non-continuing keeps `!` precedence, so a
  reachable `return`, `break`, or `continue` is not hidden by the invalid record head;
- record-head recovery now returns `Type::Never` for an actually non-continuing child
  instead of unconditionally collapsing the rejected expression to `Type::Error`; and
- adversarial tests lock definite-initialization, conditional break-exit, HIR type,
  and non-continuation behavior for both unknown targets and enum-as-record misuse.

'''
patch(roadmap, marker, addition + marker)
