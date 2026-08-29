from pathlib import Path

path = Path("crates/nova-sema/src/analyzer.rs")
text = path.read_text()
old = '''                for continue_cursor in &loop_context.continue_cursors {
                    self.flow
                        .as_mut()
                        .expect("semantic lowering must own a function flow graph")
                        .add_backedge(*continue_cursor, loop_context.header);
                }
'''
new = '''                if condition.ty == Type::Bool && !guaranteed_skip {
                    for continue_cursor in &loop_context.continue_cursors {
                        self.flow
                            .as_mut()
                            .expect("semantic lowering must own a function flow graph")
                            .add_backedge(*continue_cursor, loop_context.header);
                    }
                }
'''
if text.count(old) != 1:
    raise SystemExit("expected exactly one continue-backedge loop")
path.write_text(text.replace(old, new))
