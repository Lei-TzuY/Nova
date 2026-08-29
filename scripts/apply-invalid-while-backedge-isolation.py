from pathlib import Path

path = Path("crates/nova-sema/src/analyzer.rs")
text = path.read_text()
old = '''                let diagnostic_body = condition.ty.is_never() || guaranteed_skip;
                let body = if diagnostic_body {
                    self.lower_block_for_diagnostics(body, return_type, true)
                } else {
                    self.lower_block(body, return_type, true)
                };
                let loop_context = self
                    .loop_stack
                    .pop()
                    .expect("while lowering must own one loop context");
                if !diagnostic_body && !body.ty.is_never() {
                    let body_exit = self.flow_cursor();
                    self.flow
                        .as_mut()
                        .expect("semantic lowering must own a function flow graph")
                        .add_backedge(body_exit, loop_context.header);
                }
                if condition.ty == Type::Bool && !guaranteed_skip {
                    for continue_cursor in &loop_context.continue_cursors {
                        self.flow
                            .as_mut()
                            .expect("semantic lowering must own a function flow graph")
                            .add_backedge(*continue_cursor, loop_context.header);
                    }
                }
'''
new = '''                let diagnostic_body = condition.ty.is_never() || guaranteed_skip;
                let executable_body = condition.ty == Type::Bool && !guaranteed_skip;
                let body = if diagnostic_body {
                    self.lower_block_for_diagnostics(body, return_type, true)
                } else {
                    self.lower_block(body, return_type, true)
                };
                let loop_context = self
                    .loop_stack
                    .pop()
                    .expect("while lowering must own one loop context");
                if executable_body && !body.ty.is_never() {
                    let body_exit = self.flow_cursor();
                    self.flow
                        .as_mut()
                        .expect("semantic lowering must own a function flow graph")
                        .add_backedge(body_exit, loop_context.header);
                }
                if executable_body {
                    for continue_cursor in &loop_context.continue_cursors {
                        self.flow
                            .as_mut()
                            .expect("semantic lowering must own a function flow graph")
                            .add_backedge(*continue_cursor, loop_context.header);
                    }
                }
'''
if text.count(old) != 1:
    raise SystemExit("expected exactly one while backedge block")
path.write_text(text.replace(old, new))
