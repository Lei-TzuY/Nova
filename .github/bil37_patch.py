from pathlib import Path

# Keep numeric-surface recursion compatible with the richer AST call node.
p = Path("crates/nova-sema/src/numeric_surface.rs")
s = p.read_text()
old = '''        ExpressionKind::Call { callee, arguments } => {
            rewrite_expression(callee);'''
new = '''        ExpressionKind::Call {
            callee, arguments, ..
        } => {
            rewrite_expression(callee);'''
assert old in s, "numeric call anchor missing"
p.write_text(s.replace(old, new, 1))

p = Path("crates/nova-sema/src/analyzer.rs")
s = p.read_text()
old = '''            ast::ExpressionKind::Call { callee, arguments } => {
                let call_entry_state = self.capture_reachable_state();
                let callee = self.lower_expression(callee, return_type);'''
new = '''            ast::ExpressionKind::Call {
                callee,
                type_arguments,
                arguments,
            } => {
                let call_entry_state = self.capture_reachable_state();
                let callee = self.lower_expression(callee, return_type);
                let explicit_type_arguments = type_arguments
                    .iter()
                    .map(|argument| self.resolve_type_ref(argument))
                    .collect::<Vec<_>>();'''
assert old in s, "analyzer call lowering anchor missing"
s = s.replace(old, new, 1)

old = '''                let ty = self.check_call(&callee, &lowered_arguments, expression.span);'''
new = '''                let ty = self.check_call(
                    &callee,
                    &explicit_type_arguments,
                    &lowered_arguments,
                    expression.span,
                );'''
assert old in s, "check_call invocation anchor missing"
s = s.replace(old, new, 1)

old = '''    fn check_call(
        &mut self,
        callee: &hir::Expression,
        arguments: &[hir::Expression],
        span: Span,
    ) -> Type {'''
new = '''    fn check_call(
        &mut self,
        callee: &hir::Expression,
        explicit_type_arguments: &[Type],
        arguments: &[hir::Expression],
        span: Span,
    ) -> Type {'''
assert old in s, "check_call signature anchor missing"
s = s.replace(old, new, 1)

old = '''        let generic_set = generic_parameters.iter().cloned().collect::<BTreeSet<_>>();

        let arity_matches = arguments.len() == signature.parameters.len();'''
new = '''        let generic_set = generic_parameters.iter().cloned().collect::<BTreeSet<_>>();
        let explicit_arity_matches = explicit_type_arguments.is_empty()
            || explicit_type_arguments.len() == generic_parameters.len();
        if !explicit_arity_matches {
            self.diagnostics.push(
                Diagnostic::error("N3039", "wrong number of generic type arguments").with_primary(
                    span,
                    format!(
                        "expected {} type argument(s), found {}",
                        generic_parameters.len(),
                        explicit_type_arguments.len()
                    ),
                ),
            );
        }

        let arity_matches = arguments.len() == signature.parameters.len();'''
assert old in s, "generic arity anchor missing"
s = s.replace(old, new, 1)

old = '''        let mut substitutions = BTreeMap::new();
        let mut argument_types_match = true;'''
new = '''        let mut substitutions = BTreeMap::new();
        if explicit_arity_matches && !explicit_type_arguments.is_empty() {
            for (name, ty) in generic_parameters.iter().zip(explicit_type_arguments) {
                substitutions.insert(name.clone(), ty.clone());
            }
        }
        let mut argument_types_match = explicit_arity_matches;'''
assert old in s, "substitution anchor missing"
s = s.replace(old, new, 1)

old = '''                    .with_note(
                        "explicit type arguments are not implemented in this bootstrap slice",
                    ),'''
new = '''                    .with_note(
                        "provide explicit call type arguments, for example `function<Int>(...)`",
                    ),'''
assert old in s, "N3038 note anchor missing"
p.write_text(s.replace(old, new, 1))
