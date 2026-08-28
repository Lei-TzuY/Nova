from pathlib import Path


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:100]!r}")
    file.write_text(text.replace(old, new, 1))


patch(
    "crates/nova-sema/src/lib.rs",
    "mod analyzer;\nmod type_rules;\n",
    "mod analyzer;\nmod flow_rules;\nmod type_rules;\n",
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    "use crate::hir::{\n    self, BindingId, EnumId, EnumType, ExpressionKind, FunctionId, FunctionType, MatchArm,\n    RecordFieldValue, RecordId, RecordType, StatementKind, Type,\n};\nuse crate::type_rules::{\n",
    "use crate::flow_rules::InitializationJoin;\nuse crate::hir::{\n    self, BindingId, EnumId, EnumType, ExpressionKind, FunctionId, FunctionType, MatchArm,\n    RecordFieldValue, RecordId, RecordType, StatementKind, Type,\n};\nuse crate::type_rules::{\n",
)

loop_old = '''    fn merge_loop_break_initialization(
        &mut self,
        entry_scopes: &[Scope],
        break_states: &[ScopeState],
    ) {
        debug_assert!(!break_states.is_empty());
        self.scopes = entry_scopes.to_vec();
        for (scope_index, entry_scope) in entry_scopes.iter().enumerate() {
            for name in entry_scope.keys() {
                let initialized = break_states.iter().all(|break_scopes| {
                    break_scopes
                        .get(scope_index)
                        .and_then(|scope| scope.get(name))
                        .is_some_and(|symbol| symbol.initialized)
                });
                if let Some(symbol) = self.scopes[scope_index].get_mut(name) {
                    symbol.initialized = initialized;
                }
            }
        }
    }
'''
loop_new = '''    fn merge_loop_break_initialization(
        &mut self,
        entry_scopes: &[Scope],
        break_states: &[ScopeState],
    ) {
        debug_assert!(!break_states.is_empty());
        self.scopes = entry_scopes.to_vec();
        for (scope_index, entry_scope) in entry_scopes.iter().enumerate() {
            for (name, entry_symbol) in entry_scope {
                let mut join = InitializationJoin::default();
                for break_scopes in break_states {
                    let initialized = break_scopes
                        .get(scope_index)
                        .and_then(|scope| scope.get(name))
                        .is_some_and(|symbol| symbol.initialized);
                    join.observe(initialized, true);
                }
                if let Some(symbol) = self.scopes[scope_index].get_mut(name) {
                    symbol.initialized = join.finish(entry_symbol.initialized);
                }
            }
        }
    }
'''
patch("crates/nova-sema/src/analyzer.rs", loop_old, loop_new)

optional_old = '''    fn merge_optional_execution_initialization(
        &mut self,
        entry_scopes: &[Scope],
        executed_scopes: &[Scope],
        executed_never: bool,
    ) {
        self.scopes = entry_scopes.to_vec();
        if executed_never {
            return;
        }
        for (scope_index, entry_scope) in entry_scopes.iter().enumerate() {
            for (name, entry_symbol) in entry_scope {
                let executed_initialized = executed_scopes
                    .get(scope_index)
                    .and_then(|scope| scope.get(name))
                    .is_some_and(|symbol| symbol.initialized);
                if let Some(symbol) = self.scopes[scope_index].get_mut(name) {
                    symbol.initialized = entry_symbol.initialized && executed_initialized;
                }
            }
        }
    }
'''
optional_new = '''    fn merge_optional_execution_initialization(
        &mut self,
        entry_scopes: &[Scope],
        executed_scopes: &[Scope],
        executed_never: bool,
    ) {
        self.scopes = entry_scopes.to_vec();
        for (scope_index, entry_scope) in entry_scopes.iter().enumerate() {
            for (name, entry_symbol) in entry_scope {
                let executed_initialized = executed_scopes
                    .get(scope_index)
                    .and_then(|scope| scope.get(name))
                    .is_some_and(|symbol| symbol.initialized);
                let mut join = InitializationJoin::default();
                join.observe(entry_symbol.initialized, true);
                join.observe(executed_initialized, !executed_never);
                if let Some(symbol) = self.scopes[scope_index].get_mut(name) {
                    symbol.initialized = join.finish(entry_symbol.initialized);
                }
            }
        }
    }
'''
patch("crates/nova-sema/src/analyzer.rs", optional_old, optional_new)

branch_old = '''    fn merge_branch_initialization(
        &mut self,
        entry_scopes: &[Scope],
        then_scopes: &[Scope],
        then_never: bool,
        else_scopes: &[Scope],
        else_never: bool,
    ) {
        self.scopes = entry_scopes.to_vec();
        for (scope_index, entry_scope) in entry_scopes.iter().enumerate() {
            for (name, entry_symbol) in entry_scope {
                let then_initialized = then_scopes
                    .get(scope_index)
                    .and_then(|scope| scope.get(name))
                    .is_some_and(|symbol| symbol.initialized);
                let else_initialized = else_scopes
                    .get(scope_index)
                    .and_then(|scope| scope.get(name))
                    .is_some_and(|symbol| symbol.initialized);
                let initialized = match (then_never, else_never) {
                    (true, true) => entry_symbol.initialized,
                    (true, false) => else_initialized,
                    (false, true) => then_initialized,
                    (false, false) => then_initialized && else_initialized,
                };
                if let Some(symbol) = self.scopes[scope_index].get_mut(name) {
                    symbol.initialized = initialized;
                }
            }
        }
    }
'''
branch_new = '''    fn merge_branch_initialization(
        &mut self,
        entry_scopes: &[Scope],
        then_scopes: &[Scope],
        then_never: bool,
        else_scopes: &[Scope],
        else_never: bool,
    ) {
        self.scopes = entry_scopes.to_vec();
        for (scope_index, entry_scope) in entry_scopes.iter().enumerate() {
            for (name, entry_symbol) in entry_scope {
                let then_initialized = then_scopes
                    .get(scope_index)
                    .and_then(|scope| scope.get(name))
                    .is_some_and(|symbol| symbol.initialized);
                let else_initialized = else_scopes
                    .get(scope_index)
                    .and_then(|scope| scope.get(name))
                    .is_some_and(|symbol| symbol.initialized);
                let mut join = InitializationJoin::default();
                join.observe(then_initialized, !then_never);
                join.observe(else_initialized, !else_never);
                if let Some(symbol) = self.scopes[scope_index].get_mut(name) {
                    symbol.initialized = join.finish(entry_symbol.initialized);
                }
            }
        }
    }
'''
patch("crates/nova-sema/src/analyzer.rs", branch_old, branch_new)

match_old = '''    fn merge_match_initialization(
        &mut self,
        entry_scopes: &[Scope],
        branches: &[(ScopeState, bool)],
    ) {
        self.scopes = entry_scopes.to_vec();
        let continuing = branches
            .iter()
            .filter(|(_, never)| !never)
            .map(|(scopes, _)| scopes)
            .collect::<Vec<_>>();
        if continuing.is_empty() {
            return;
        }

        for (scope_index, entry_scope) in entry_scopes.iter().enumerate() {
            for name in entry_scope.keys() {
                let initialized = continuing.iter().all(|branch_scopes| {
                    branch_scopes
                        .get(scope_index)
                        .and_then(|scope| scope.get(name))
                        .is_some_and(|symbol| symbol.initialized)
                });
                if let Some(symbol) = self.scopes[scope_index].get_mut(name) {
                    symbol.initialized = initialized;
                }
            }
        }
    }
'''
match_new = '''    fn merge_match_initialization(
        &mut self,
        entry_scopes: &[Scope],
        branches: &[(ScopeState, bool)],
    ) {
        self.scopes = entry_scopes.to_vec();
        for (scope_index, entry_scope) in entry_scopes.iter().enumerate() {
            for (name, entry_symbol) in entry_scope {
                let mut join = InitializationJoin::default();
                for (branch_scopes, never) in branches {
                    let initialized = branch_scopes
                        .get(scope_index)
                        .and_then(|scope| scope.get(name))
                        .is_some_and(|symbol| symbol.initialized);
                    join.observe(initialized, !never);
                }
                if let Some(symbol) = self.scopes[scope_index].get_mut(name) {
                    symbol.initialized = join.finish(entry_symbol.initialized);
                }
            }
        }
    }
'''
patch("crates/nova-sema/src/analyzer.rs", match_old, match_new)

roadmap = "docs/roadmap.md"
patch(
    roadmap,
    "**Status: fourteen vertical slices implemented; broader type-system work remains.**\n",
    "**Status: fifteen vertical slices implemented; broader type-system work remains.**\n",
)
marker = "The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n"
addition = '''Implemented in the fifteenth Phase 2 slice:

- definite-initialization joins now share an internal pure `InitializationJoin`
  contract instead of encoding continuing-path intersection separately for loops,
  optional execution, `if`, and exhaustive `match`;
- only paths that can reach the join point participate, and a binding is considered
  initialized exactly when every such path reports it initialized;
- when every alternative is non-continuing, the entry fact is retained for later
  diagnostic-only lowering rather than inventing a reachable initialization fact;
- scope lookup and binding identity remain analyzer responsibilities, keeping the
  reusable flow rule independent of lexical representation; and
- truth-table tests plus the existing loop, short-circuit, branch, match, and invalid-
  aggregate adversarial suites lock the refactor to behavior-preserving dataflow.

'''
patch(roadmap, marker, addition + marker)
