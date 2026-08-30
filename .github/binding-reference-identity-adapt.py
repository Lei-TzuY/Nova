from pathlib import Path


def replace_exact(path, old, new, count=1):
    p = Path(path)
    text = p.read_text()
    actual = text.count(old)
    if actual != count:
        raise SystemExit(f"{path}: expected {count} occurrences, found {actual}: {old[:100]!r}")
    p.write_text(text.replace(old, new, count))

analyzer = "crates/nova-sema/src/analyzer.rs"
replace_exact(
    analyzer,
    """        assert_eq!(*target, Some(binding.id));\n""",
    """        let target = target.as_ref().expect(\"resolved assignment target\");\n        assert_eq!(target.binding, binding.id);\n        assert_eq!(target.binding_name, binding.name);\n        assert_eq!(target.declaration_span, binding.span);\n""",
)

inspect = "crates/nova-inspect/src/lib.rs"
replace_exact(
    inspect,
    """        let second_tail = cross_owner.functions[1]\n            .body\n            .tail\n            .as_deref_mut()\n            .expect(\"second has a tail expression\");\n        second_tail.kind = hir::ExpressionKind::Binding(hir::BindingId::new(0));\n""",
    """        let foreign = match &cross_owner.functions[0].body.statements[0].kind {\n            hir::StatementKind::Binding { binding, .. } => binding.clone(),\n            _ => panic!(\"expected foreign binding\"),\n        };\n        let second_tail = cross_owner.functions[1]\n            .body\n            .tail\n            .as_deref_mut()\n            .expect(\"second has a tail expression\");\n        second_tail.kind = hir::ExpressionKind::Binding(hir::BindingReference {\n            binding: foreign.id,\n            binding_name: foreign.name,\n            declaration_span: foreign.span,\n        });\n""",
)
replace_exact(
    inspect,
    """        let tail = escaped_scope.functions[0]\n            .body\n            .tail\n            .as_deref_mut()\n            .expect(\"main has a tail expression\");\n        tail.kind = hir::ExpressionKind::Binding(hir::BindingId::new(0));\n""",
    """        let hidden = match &escaped_scope.functions[0].body.statements[0].kind {\n            hir::StatementKind::Expression(expression) => {\n                let hir::ExpressionKind::Block(block) = &expression.kind else {\n                    panic!(\"expected nested block\");\n                };\n                match &block.statements[0].kind {\n                    hir::StatementKind::Binding { binding, .. } => binding.clone(),\n                    _ => panic!(\"expected hidden binding\"),\n                }\n            }\n            _ => panic!(\"expected block expression statement\"),\n        };\n        let tail = escaped_scope.functions[0]\n            .body\n            .tail\n            .as_deref_mut()\n            .expect(\"main has a tail expression\");\n        tail.kind = hir::ExpressionKind::Binding(hir::BindingReference {\n            binding: hidden.id,\n            binding_name: hidden.name,\n            declaration_span: hidden.span,\n        });\n""",
)
replace_exact(
    inspect,
    """        let hir::StatementKind::Assignment { target, .. } =\n            &mut program.functions[0].body.statements[2].kind\n        else {\n            panic!(\"expected assignment HIR\");\n        };\n        *target = Some(hir::BindingId::new(0));\n""",
    """        let fixed = match &program.functions[0].body.statements[0].kind {\n            hir::StatementKind::Binding { binding, .. } => binding.clone(),\n            _ => panic!(\"expected immutable binding\"),\n        };\n        let hir::StatementKind::Assignment { target, .. } =\n            &mut program.functions[0].body.statements[2].kind\n        else {\n            panic!(\"expected assignment HIR\");\n        };\n        *target = Some(hir::BindingReference {\n            binding: fixed.id,\n            binding_name: fixed.name,\n            declaration_span: fixed.span,\n        });\n""",
)
