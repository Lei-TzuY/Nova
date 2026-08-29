from pathlib import Path

ANALYZER = Path("crates/nova-sema/src/analyzer.rs")
ROADMAP = Path("docs/roadmap.md")
CONTROL = Path("docs/control-flow.md")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 occurrence, found {count}")
    return text.replace(old, new, 1)


source = ANALYZER.read_text()
old_return = '''            ast::StatementKind::Return(expression) => {
                let expression = self.lower_expression(expression, return_type);
                self.require_type(
                    &expression.ty,
                    return_type,
                    expression.span,
                    "return expression",
                );
                self.flow_advance(
                    FlowNodeKind::Transfer(FlowTransfer::Return),
                    Some(statement.span),
                );
                (StatementKind::Return(expression), true)
            }
'''
new_return = '''            ast::StatementKind::Return(expression) => {
                let expression = self.lower_expression(expression, return_type);
                self.require_type(
                    &expression.ty,
                    return_type,
                    expression.span,
                    "return expression",
                );
                if !expression.ty.is_never() {
                    self.flow_advance(
                        FlowNodeKind::Transfer(FlowTransfer::Return),
                        Some(statement.span),
                    );
                }
                (StatementKind::Return(expression), true)
            }
'''
source = replace_once(source, old_return, new_return, "guard parent return transfer")
ANALYZER.write_text(source)

roadmap = ROADMAP.read_text()
roadmap = replace_once(
    roadmap,
    "**Status: twenty-eight vertical slices implemented; broader type-system work remains.**",
    "**Status: twenty-nine vertical slices implemented; broader type-system work remains.**",
    "roadmap status",
)
marker = '''
The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:
'''
section = '''
Implemented in the twenty-ninth Phase 2 slice:

- a `return expression;` emits its own CFG `Return` transfer only when evaluating the
  expression can complete normally;
- if the return expression already transfers control through an inner `return`,
  `break`, or `continue`, the parent statement remains non-continuing in HIR but does
  not append an impossible execution successor after that child transfer;
- nested-return CFGs therefore satisfy the verifier's transfer-successor invariant
  instead of failing closed with internal diagnostic `N3999`;
- a `break` reached while evaluating a return expression continues to target and exit
  the nearest lexical loop rather than being overwritten by the syntactic parent
  return;
- ordinary completed return expressions still emit exactly one `Return` transfer; and
- focused CFG regressions plus the full workspace suite lock child-transfer precedence
  without changing syntax, runtime semantics, or semantic-inspection schema v1.

The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:
'''
roadmap = replace_once(roadmap, marker, section, "roadmap slice 29")
ROADMAP.write_text(roadmap)

control = CONTROL.read_text()
needle = '''- `return`, `break`, `continue`, and `Exit` successor edge classes respect their
  transfer behavior.
'''
replacement = '''- `return`, `break`, `continue`, and `Exit` successor edge classes respect their
  transfer behavior; and
- a syntactic parent transfer does not append an execution node when evaluating its
  child expression has already transferred control.
'''
control = replace_once(control, needle, replacement, "control-flow transfer invariant")
CONTROL.write_text(control)
