from pathlib import Path


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:220]!r}")
    file.write_text(text.replace(old, new, 1))


def write(path: str, content: str) -> None:
    file = Path(path)
    file.parent.mkdir(parents=True, exist_ok=True)
    file.write_text(content)


patch(
    "crates/nova-sema/src/analyzer.rs",
    '''                let body = if condition_literal == Some(false) {\n                    self.lower_block_for_diagnostics(body, return_type, true)\n                } else {\n                    self.lower_block(body, return_type, true)\n                };\n''',
    '''                let body = if condition.ty.is_never() || condition_literal == Some(false) {\n                    self.lower_block_for_diagnostics(body, return_type, true)\n                } else {\n                    self.lower_block(body, return_type, true)\n                };\n''',
)

old_if = '''                let (then_branch, then_scopes, else_branch, else_scopes) = match condition_literal {\n                    Some(true) => {\n                        let then_branch = self.lower_block(then_branch, return_type, true);\n                        let then_scopes = self.scopes.clone();\n                        let then_loop_stack = self.loop_stack.clone();\n\n                        self.scopes = entry_scopes.clone();\n                        self.loop_stack = post_condition_loop_stack.clone();\n                        let else_branch =\n                            self.lower_expression_for_diagnostics(else_branch, return_type);\n                        let else_scopes = self.scopes.clone();\n\n                        self.scopes = then_scopes.clone();\n                        self.loop_stack = then_loop_stack;\n                        (then_branch, then_scopes, else_branch, else_scopes)\n                    }\n                    Some(false) => {\n                        let then_branch =\n                            self.lower_block_for_diagnostics(then_branch, return_type, true);\n                        let then_scopes = self.scopes.clone();\n\n                        self.scopes = entry_scopes.clone();\n                        self.loop_stack = post_condition_loop_stack.clone();\n                        let else_branch = self.lower_expression(else_branch, return_type);\n                        let else_scopes = self.scopes.clone();\n                        (then_branch, then_scopes, else_branch, else_scopes)\n                    }\n                    None => {\n                        let then_branch = self.lower_block(then_branch, return_type, true);\n                        let then_scopes = self.scopes.clone();\n\n                        self.scopes = entry_scopes.clone();\n                        let else_branch = self.lower_expression(else_branch, return_type);\n                        let else_scopes = self.scopes.clone();\n                        (then_branch, then_scopes, else_branch, else_scopes)\n                    }\n                };\n'''
new_if = '''                let (then_branch, then_scopes, else_branch, else_scopes) =\n                    if condition.ty.is_never() {\n                        let then_branch =\n                            self.lower_block_for_diagnostics(then_branch, return_type, true);\n                        let then_scopes = self.scopes.clone();\n\n                        self.scopes = entry_scopes.clone();\n                        self.loop_stack = post_condition_loop_stack.clone();\n                        let else_branch =\n                            self.lower_expression_for_diagnostics(else_branch, return_type);\n                        let else_scopes = self.scopes.clone();\n                        (then_branch, then_scopes, else_branch, else_scopes)\n                    } else {\n                        match condition_literal {\n                            Some(true) => {\n                                let then_branch = self.lower_block(then_branch, return_type, true);\n                                let then_scopes = self.scopes.clone();\n                                let then_loop_stack = self.loop_stack.clone();\n\n                                self.scopes = entry_scopes.clone();\n                                self.loop_stack = post_condition_loop_stack.clone();\n                                let else_branch =\n                                    self.lower_expression_for_diagnostics(else_branch, return_type);\n                                let else_scopes = self.scopes.clone();\n\n                                self.scopes = then_scopes.clone();\n                                self.loop_stack = then_loop_stack;\n                                (then_branch, then_scopes, else_branch, else_scopes)\n                            }\n                            Some(false) => {\n                                let then_branch = self.lower_block_for_diagnostics(\n                                    then_branch,\n                                    return_type,\n                                    true,\n                                );\n                                let then_scopes = self.scopes.clone();\n\n                                self.scopes = entry_scopes.clone();\n                                self.loop_stack = post_condition_loop_stack.clone();\n                                let else_branch = self.lower_expression(else_branch, return_type);\n                                let else_scopes = self.scopes.clone();\n                                (then_branch, then_scopes, else_branch, else_scopes)\n                            }\n                            None => {\n                                let then_branch =\n                                    self.lower_block(then_branch, return_type, true);\n                                let then_scopes = self.scopes.clone();\n\n                                self.scopes = entry_scopes.clone();\n                                let else_branch = self.lower_expression(else_branch, return_type);\n                                let else_scopes = self.scopes.clone();\n                                (then_branch, then_scopes, else_branch, else_scopes)\n                            }\n                        }\n                    };\n'''
patch("crates/nova-sema/src/analyzer.rs", old_if, new_if)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''            let value = if selected_variant_index.is_some() && !selected_arm {\n                self.lower_expression_for_diagnostics(&arm.value, return_type)\n            } else {\n                self.lower_expression(&arm.value, return_type)\n            };\n''',
    '''            let value = if scrutinee.ty.is_never()\n                || (selected_variant_index.is_some() && !selected_arm)\n            {\n                self.lower_expression_for_diagnostics(&arm.value, return_type)\n            } else {\n                self.lower_expression(&arm.value, return_type)\n            };\n''',
)

write(
    "crates/nova-cli/tests/fixtures/valid/noncontinuing-successors.nv",
    '''fn main() -> Int {\n    if {\n        return 42;\n        false\n    } {\n        1 / 0\n    } else {\n        2 / 0\n    }\n}\n''',
)

patch(
    "crates/nova-cli/tests/cli.rs",
    '''        "valid/constant-condition-flow.nv",\n        "valid/literal-match-flow.nv",\n''',
    '''        "valid/constant-condition-flow.nv",\n        "valid/noncontinuing-successors.nv",\n        "valid/literal-match-flow.nv",\n''',
)
patch(
    "crates/nova-cli/tests/cli.rs",
    '''        ("valid/constant-condition-flow.nv", "42\\n"),\n        ("valid/literal-match-flow.nv", "42\\n"),\n''',
    '''        ("valid/constant-condition-flow.nv", "42\\n"),\n        ("valid/noncontinuing-successors.nv", "42\\n"),\n        ("valid/literal-match-flow.nv", "42\\n"),\n''',
)

patch(
    "README.md",
    '''HIR. Names, calls, blocks, aggregates, and other dynamic operands stop the proof.\nSuccessful constant arithmetic is not folded, and any expression with a dynamic\noperand remains runtime checked. Such\n''',
    '''HIR. Names, calls, blocks, aggregates, and other dynamic operands stop the proof.\nMore generally, when an `if`/`while` condition or `match` scrutinee is already\nnon-continuing (`!`), its successor branches/body/arms are lowered only for static\ndiagnostics: execution-only constant failures and flow mutations cannot come from a\npath runtime control never reaches. Successful constant arithmetic is not folded, and\nany expression with a dynamic operand remains runtime checked. Such\n''',
)

patch(
    "docs/language-constitution.md",
    '''guaranteed-loop reasoning previously reserved for literal `true`. Successful arithmetic\ntrees are deliberately not folded, and dynamic operands stop these proofs rather than\ntriggering general constant propagation.\n''',
    '''guaranteed-loop reasoning previously reserved for literal `true`. Independently of\nconstant evaluation, an `if`/`while` condition or `match` scrutinee already typed `!`\nproves every successor branch/body/arm unreachable; those successors remain statically\nchecked but are lowered in diagnostic-only mode so execution-failure diagnostics and\nflow mutations cannot escape a path that cannot run. Successful arithmetic trees are\ndeliberately not folded, and dynamic operands stop constant proofs rather than\ntriggering general constant propagation.\n''',
)

patch(
    "docs/roadmap.md",
    '''**Status: twenty-five vertical slices implemented; broader type-system work remains.**\n''',
    '''**Status: twenty-six vertical slices implemented; broader type-system work remains.**\n''',
)
marker = '''The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n'''
addition = '''Implemented in the twenty-sixth Phase 2 slice:\n\n- successor lowering now treats an already non-continuing (`!`) discriminator as a\n  first-class reachability boundary for `if`, `while`, and `match`;\n- both `if` branches, a `while` body, and every match arm are still lowered for static\n  name/type/pattern/exhaustiveness diagnostics when their predecessor cannot continue,\n  but use diagnostic-only state so execution-failure preflight and flow mutations do\n  not leak from runtime-impossible successors;\n- dead loop bodies retain their lexical loop context, so `break`/`continue` stay legal\n  even though their exit facts are discarded along with the unreachable path;\n- reachable successors continue to report N3031/N3032 normally, preventing the\n  diagnostic-only mechanism from becoming a blanket constant-error suppression rule;\n- the change complements closed constant-condition reachability without broadening the\n  constant evaluator or changing HIR/schema shape; and\n- semantic regressions plus a CLI check/run fixture lock noncontinuing conditions and\n  scrutinees, static-diagnostic preservation, runtime return propagation, and reachable\n  constant-error behavior end to end.\n\n'''
patch("docs/roadmap.md", marker, addition + marker)
