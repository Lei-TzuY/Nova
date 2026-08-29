from pathlib import Path

path = Path("crates/nova-sema/src/analyzer.rs")
text = path.read_text()
old = '''                if !diagnostic_body && !body.ty.is_never() {
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
new = '''                let body_can_execute = condition.ty == Type::Bool && !guaranteed_skip;
                if body_can_execute && !body.ty.is_never() {
                    let body_exit = self.flow_cursor();
                    self.flow
                        .as_mut()
                        .expect("semantic lowering must own a function flow graph")
                        .add_backedge(body_exit, loop_context.header);
                }
                if body_can_execute {
                    for continue_cursor in &loop_context.continue_cursors {
                        self.flow
                            .as_mut()
                            .expect("semantic lowering must own a function flow graph")
                            .add_backedge(*continue_cursor, loop_context.header);
                    }
                }
'''
if text.count(old) != 1:
    raise SystemExit("expected exactly one partial while-backedge block")
path.write_text(text.replace(old, new))
