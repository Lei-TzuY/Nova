from pathlib import Path


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:120]!r}")
    file.write_text(text.replace(old, new, 1))


patch(
    "crates/nova-sema/src/analyzer.rs",
    '''            ast::StatementKind::While { condition, body } => {
                let condition = self.lower_expression(condition, return_type);
''',
    '''            ast::StatementKind::While { condition, body } => {
                let condition_entry_state = self.capture_reachable_state();
                let condition = self.lower_expression(condition, return_type);
''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''                let diverges = if condition.ty.is_never() {
                    self.scopes = post_condition_scopes;
                    true
                } else if guaranteed_entry {
''',
    '''                let diverges = if condition.ty.is_never() {
                    self.scopes = post_condition_scopes;
                    true
                } else if condition.ty != Type::Bool {
                    self.restore_reachable_state(condition_entry_state);
                    false
                } else if guaranteed_entry {
''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''            ast::ExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = self.lower_expression(condition, return_type);
''',
    '''            ast::ExpressionKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_entry_state = self.capture_reachable_state();
                let condition = self.lower_expression(condition, return_type);
''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''                let ty = if condition.ty.is_never() {
                    self.scopes = entry_scopes;
                    self.loop_stack = post_condition_loop_stack;
                    Type::Never
                } else {
''',
    '''                let ty = if condition.ty.is_never() {
                    self.scopes = entry_scopes;
                    self.loop_stack = post_condition_loop_stack;
                    Type::Never
                } else if condition.ty != Type::Bool {
                    self.restore_reachable_state(condition_entry_state);
                    Type::Error
                } else {
''',
)

patch(
    "README.md",
    '''Rejected unary and binary operators are also fail-closed. Concrete operand type
mismatches produce Error HIR rather than a nominal success type, and flow facts from
a continuing rejected operator are rolled back. Non-continuation from an operand that
must be evaluated keeps `!` precedence; short-circuit operators retain their existing
conditional right-hand evaluation rules.

''',
    '''Rejected unary and binary operators are also fail-closed. Concrete operand type
mismatches produce Error HIR rather than a nominal success type, and flow facts from
a continuing rejected operator are rolled back. Non-continuation from an operand that
must be evaluated keeps `!` precedence; short-circuit operators retain their existing
conditional right-hand evaluation rules.

Invalid continuing control conditions are fail-closed too. A non-Bool or erroneous
`if` condition makes the expression Error-typed and discards condition/branch flow
facts; a rejected `while` condition likewise cannot export pre-test initialization or
loop exits. A condition that is already `!` keeps its non-continuation, while valid
Bool conditions retain the established pre-test and branch dataflow rules.

''',
)

patch(
    "docs/language-constitution.md",
    '''A continuing unary or binary operator rejected by operand typing is fail-closed for
flow recovery as well. Concrete mismatches yield `<error>` rather than retaining the
operator's nominal result type, and assignments or loop-exit facts created only while
lowering that rejected operator are discarded. A definitely evaluated operand that
is already `!` retains non-continuation precedence; `&&` and `||` continue to model
conditional right-hand evaluation rather than treating every lowered RHS as reachable.

''',
    '''A continuing unary or binary operator rejected by operand typing is fail-closed for
flow recovery as well. Concrete mismatches yield `<error>` rather than retaining the
operator's nominal result type, and assignments or loop-exit facts created only while
lowering that rejected operator are discarded. A definitely evaluated operand that
is already `!` retains non-continuation precedence; `&&` and `||` continue to model
conditional right-hand evaluation rather than treating every lowered RHS as reachable.

A continuing `if` or `while` whose condition is not a valid `Bool` is also fail-closed
for flow recovery. The condition and nested branches/body are still lowered for
source diagnostics and lexical loop-control checking, but assignments and loop-exit
facts created only inside the rejected control construct do not become post-construct
facts. Invalid `if` conditions make the expression `<error>`-typed. A condition that
is already `!` retains non-continuation precedence rather than being flattened to a
continuing recovery error.

''',
)

roadmap = "docs/roadmap.md"
patch(
    roadmap,
    "**Status: nineteen vertical slices implemented; broader type-system work remains.**\n",
    "**Status: twenty vertical slices implemented; broader type-system work remains.**\n",
)
marker = '''The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:
'''
addition = '''Implemented in the twentieth Phase 2 slice:

- `if` and `while` capture reachable state before evaluating their condition so a
  continuing rejected condition cannot export assignments or loop-exit facts;
- concrete non-Bool and recovery-Error `if` conditions make the whole expression
  `Type::Error` instead of retaining a normal branch join type;
- invalid `while` conditions still lower their body under a lexical loop context for
  deterministic diagnostics, but condition/body flow is rolled back afterward;
- nested invalid loop conditions cannot manufacture break exits for an enclosing
  otherwise-infinite loop;
- conditions already typed `!` retain non-continuation precedence, while valid Bool
  pre-test initialization and ordinary branch merging remain unchanged; and
- red-to-green adversarial tests lock condition-side initialization, branch flow,
  nested break exits, Error typing, valid pre-test facts, and `!` behavior.

'''
patch(roadmap, marker, addition + marker)
