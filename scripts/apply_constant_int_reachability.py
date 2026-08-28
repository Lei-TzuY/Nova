from pathlib import Path


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:140]!r}")
    file.write_text(text.replace(old, new, 1))


patch(
    "crates/nova-sema/src/analyzer.rs",
    '''    next_binding: usize,\n    loop_stack: Vec<LoopContext>,\n}\n''',
    '''    next_binding: usize,\n    loop_stack: Vec<LoopContext>,\n    diagnostic_only_depth: usize,\n}\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''            next_binding: 0,\n            loop_stack: Vec::new(),\n        }\n''',
    '''            next_binding: 0,\n            loop_stack: Vec::new(),\n            diagnostic_only_depth: 0,\n        }\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''        let reachable_state = self.capture_reachable_state();\n        let lowered = self.lower_expression(expression, return_type);\n        self.restore_reachable_state(reachable_state);\n        lowered\n    }\n\n    fn lower_block_for_diagnostics(\n''',
    '''        let reachable_state = self.capture_reachable_state();\n        self.diagnostic_only_depth += 1;\n        let lowered = self.lower_expression(expression, return_type);\n        self.diagnostic_only_depth -= 1;\n        self.restore_reachable_state(reachable_state);\n        lowered\n    }\n\n    fn lower_block_for_diagnostics(\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''        let reachable_state = self.capture_reachable_state();\n        let lowered = self.lower_block(block, return_type, push_scope);\n        self.restore_reachable_state(reachable_state);\n        lowered\n    }\n\n    fn lower_rejected_record_fields(\n''',
    '''        let reachable_state = self.capture_reachable_state();\n        self.diagnostic_only_depth += 1;\n        let lowered = self.lower_block(block, return_type, push_scope);\n        self.diagnostic_only_depth -= 1;\n        self.restore_reachable_state(reachable_state);\n        lowered\n    }\n\n    fn lower_rejected_record_fields(\n''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''    ) -> bool {\n        let Some(Err(error)) = result else {\n            return false;\n        };\n''',
    '''    ) -> bool {\n        if self.diagnostic_only_depth > 0 {\n            return false;\n        }\n        let Some(Err(error)) = result else {\n            return false;\n        };\n''',
)
