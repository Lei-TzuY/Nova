from pathlib import Path
import re


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:120]!r}")
    file.write_text(text.replace(old, new, 1))


analyzer = Path("crates/nova-sema/src/analyzer.rs")
text = analyzer.read_text()
pattern = re.compile(r"    fn lower_field_access\([\s\S]*?\n    \}\n\n(?=    fn )")
replacement = '''    fn lower_field_access(
        &mut self,
        base: &ast::Expression,
        field: &ast::Name,
        return_type: &Type,
    ) -> (ExpressionKind, Type) {
        let access_entry_state = self.capture_reachable_state();
        let base = self.lower_expression(base, return_type);
        if base.ty.is_never() {
            return (ExpressionKind::Error, Type::Never);
        }

        let Type::Record(record_type) = base.ty.clone() else {
            if !base.ty.is_error() {
                self.diagnostics
                    .push(Diagnostic::error("N3004", "type mismatch").with_primary(
                        field.span,
                        format!("field access requires a record value, found {}", base.ty),
                    ));
            }
            self.restore_reachable_state(access_entry_state);
            return (ExpressionKind::Error, Type::Error);
        };

        let definition = self.record_definitions[record_type.id.index()].clone();
        let Some(field_index) = definition
            .fields
            .iter()
            .position(|declared| declared.name == field.text)
        else {
            self.diagnostics.push(
                Diagnostic::error("N3011", "unknown record field")
                    .with_primary(
                        field.span,
                        format!(
                            "record `{}` has no field named `{}`",
                            definition.name, field.text
                        ),
                    )
                    .with_secondary(definition.span, "record declared here"),
            );
            self.restore_reachable_state(access_entry_state);
            return (ExpressionKind::Error, Type::Error);
        };
        let ty = definition.fields[field_index].ty.clone();
        (
            ExpressionKind::FieldAccess {
                base: Box::new(base),
                record: record_type.id,
                field_index,
            },
            ty,
        )
    }

'''
text, count = pattern.subn(replacement, text, count=1)
if count != 1:
    raise SystemExit(f"lower_field_access replacement count was {count}")
analyzer.write_text(text)

patch(
    "README.md",
    "Rejected calls are fail-closed for continuing flow recovery. Callees and arguments\nare still lowered left-to-right for deterministic diagnostics, but a non-callable\ncallee, wrong arity, argument type mismatch, or erroneous argument yields Error HIR\nand cannot export assignments or loop-exit facts. An actually evaluated child that\nis already non-continuing keeps its `!` flow.\n\n",
    "Rejected calls are fail-closed for continuing flow recovery. Callees and arguments\nare still lowered left-to-right for deterministic diagnostics, but a non-callable\ncallee, wrong arity, argument type mismatch, or erroneous argument yields Error HIR\nand cannot export assignments or loop-exit facts. An actually evaluated child that\nis already non-continuing keeps its `!` flow.\n\nRejected field access follows the same recovery discipline. The base is evaluated\nfirst, but a continuing non-record base or unknown field yields Error HIR and cannot\nexport flow facts produced only inside the rejected access. A base that is already\nnon-continuing keeps `!` without a secondary field-type cascade.\n\n",
)

patch(
    "docs/language-constitution.md",
    "A continuing call rejected because its callee is not callable, its arity is wrong,\nan argument has the wrong type, or an evaluated argument is erroneous is fail-closed\nfor flow recovery. Callee/argument diagnostics and HIR children are retained, but\nassignments and loop-exit facts produced only inside that rejected call do not become\npost-call facts. An actually evaluated non-continuing callee or argument keeps `!`\nprecedence.\n\n",
    "A continuing call rejected because its callee is not callable, its arity is wrong,\nan argument has the wrong type, or an evaluated argument is erroneous is fail-closed\nfor flow recovery. Callee/argument diagnostics and HIR children are retained, but\nassignments and loop-exit facts produced only inside that rejected call do not become\npost-call facts. An actually evaluated non-continuing callee or argument keeps `!`\nprecedence.\n\nA continuing field access rejected because its base is not a record or the named field\ndoes not exist is likewise fail-closed for flow recovery. The base is still lowered\nfor deterministic diagnostics, but its assignments and loop-exit facts do not become\npost-access facts. A base expression that is already `!` remains non-continuing and\ndoes not acquire a secondary record-type diagnostic.\n\n",
)

roadmap = "docs/roadmap.md"
patch(
    roadmap,
    "**Status: seventeen vertical slices implemented; broader type-system work remains.**\n",
    "**Status: eighteen vertical slices implemented; broader type-system work remains.**\n",
)
marker = "The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n"
addition = '''Implemented in the eighteenth Phase 2 slice:

- field-access lowering snapshots reachable state before evaluating its base, so a
  continuing rejected projection cannot export assignments or loop-exit facts;
- non-record bases and unknown record fields remain source-diagnosed and produce
  `Type::Error` HIR, while valid record projection keeps its established slot/type;
- a base expression that is already non-continuing yields `!` directly instead of
  receiving a cascading N3004 record-type error and being flattened to recovery Error;
- the fix is deliberately local to field access rather than declaring a global
  transactional policy for every erroneous expression category; and
- red-to-green adversarial tests lock definite-initialization, conditional break,
  unknown-field, recovery typing, and non-continuation behavior.

'''
patch(roadmap, marker, addition + marker)
