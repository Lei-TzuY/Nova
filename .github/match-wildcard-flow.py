from pathlib import Path
p = Path('.github/match-wildcard-core.py')
text = p.read_text()
old = '''            let branch_state = (self.capture_scope_flow_state(), value.ty.is_never());
            if selected_arm {
                selected_branch = Some((branch_state.0.clone(), value.ty.clone()));
            }
            branch_states.push(branch_state);
            branch_types.push((value.ty.clone(), value.span));
'''
new = '''            let branch_state = (self.capture_scope_flow_state(), value.ty.is_never());
            if selected_arm {
                selected_branch = Some((branch_state.0.clone(), value.ty.clone()));
            }
            if arm_reachable {
                branch_states.push(branch_state);
            }
            branch_types.push((value.ty.clone(), value.span));
'''
if text.count(old) != 1:
    raise RuntimeError('match branch-state anchor changed')
p.write_text(text.replace(old, new, 1))
