from pathlib import Path
import re

ANALYZER = Path("crates/nova-sema/src/analyzer.rs")
LIB = Path("crates/nova-sema/src/lib.rs")
CONTROL_DOC = Path("docs/control-flow.md")
CONSTITUTION = Path("docs/language-constitution.md")
ROADMAP = Path("docs/roadmap.md")
README = Path("README.md")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 occurrence, found {count}")
    return text.replace(old, new, 1)


def regex_once(text: str, pattern: str, replacement: str, label: str) -> str:
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 regex match, found {count}")
    return updated


source = ANALYZER.read_text()
source = replace_once(
    source,
    "use crate::flow_rules::InitializationJoin;\n",
    "",
    "remove InitializationJoin import",
)
source = replace_once(
    source,
    "    initialized: bool,\n",
    "",
    "remove LocalSymbol initialized bit",
)
source = replace_once(
    source,
    "    visible_scope_count: usize,\n",
    "",
    "remove loop visible-scope initialization state",
)
source = replace_once(
    source,
    "                    visible_scope_count: self.scopes.len(),\n",
    "",
    "remove loop context visible-scope capture",
)

true_calls = source.count("self.insert_local(&binding, true);")
false_calls = source.count("self.insert_local(&binding, false);")
if true_calls != 4 or false_calls != 1:
    raise SystemExit(
        f"unexpected insert_local call shape: true={true_calls}, false={false_calls}"
    )
source = source.replace(
    "self.insert_local(&binding, true);",
    "self.insert_local(&binding);\n            self.record_initialization(binding.id, binding.span);",
)
source = source.replace(
    "self.insert_local(&binding, false);",
    "self.insert_local(&binding);",
)

source = replace_once(
    source,
    "                self.insert_local(&binding);\n            self.record_initialization(binding.id, binding.span);\n                let diverges = initializer.ty.is_never();",
    "                self.insert_local(&binding);\n                if !initializer.ty.is_never() {\n                    self.record_initialization(binding.id, binding.span);\n                }\n                let diverges = initializer.ty.is_never();",
    "guard initialized declaration event after noncontinuing initializer",
)

source = replace_once(
    source,
    "                        self.mark_initialized(&target.text, target.span);",
    "                        self.record_initialization(symbol.id, target.span);",
    "assignment initialization event",
)

source = replace_once(
    source,
    "            if !symbol.initialized {\n                return (ExpressionKind::Binding(symbol.id), Type::Error);\n            }\n            return (ExpressionKind::Binding(symbol.id), symbol.ty);",
    "            return (ExpressionKind::Binding(symbol.id), symbol.ty);",
    "binding reads retain declared type",
)

source = regex_once(
    source,
    r"    fn mark_initialized\(&mut self, name: &str, assignment_span: Span\) \{.*?\n    \}\n\n    fn record_loop_break_exit",
    "    fn record_initialization(&mut self, binding: BindingId, span: Span) {\n"
    "        self.flow_advance(FlowNodeKind::Initialize(binding), Some(span));\n"
    "    }\n\n"
    "    fn record_loop_break_exit",
    "replace mutable inline initialization with CFG event",
)

source = regex_once(
    source,
    r"    fn record_loop_break_exit\(&mut self\) \{.*?\n    \}\n\n    fn record_loop_continue",
    "    fn record_loop_break_exit(&mut self) {\n"
    "        let state = self.capture_scope_flow_state();\n"
    "        self.loop_stack\n"
    "            .last_mut()\n"
    "            .expect(\"a legal break must have an active loop context\")\n"
    "            .break_states\n"
    "            .push(state);\n"
    "    }\n\n"
    "    fn record_loop_continue",
    "remove break initialization snapshot",
)

source = source.replace("merge_loop_break_initialization", "merge_loop_break_flow")
source = source.replace("merge_optional_execution_initialization", "merge_optional_execution_flow")
source = source.replace("merge_branch_initialization", "merge_branch_flow")
source = source.replace("merge_match_initialization", "merge_match_flow")

source = regex_once(
    source,
    r"    fn merge_loop_break_flow\(.*?\n    \}\n\n    fn merge_optional_execution_flow",
    "    fn merge_loop_break_flow(\n"
    "        &mut self,\n"
    "        entry: &ScopeFlowState,\n"
    "        break_states: &[ScopeFlowState],\n"
    "    ) {\n"
    "        debug_assert!(!break_states.is_empty());\n"
    "        self.scopes = entry.scopes.clone();\n"
    "        self.flow_join(break_states.iter().map(|state| state.flow_cursor), None);\n"
    "    }\n\n"
    "    fn merge_optional_execution_flow",
    "simplify loop break merge",
)

source = regex_once(
    source,
    r"    fn merge_optional_execution_flow\(.*?\n    \}\n\n    fn merge_branch_flow",
    "    fn merge_optional_execution_flow(\n"
    "        &mut self,\n"
    "        entry: &ScopeFlowState,\n"
    "        executed: &ScopeFlowState,\n"
    "        executed_never: bool,\n"
    "    ) {\n"
    "        self.scopes = entry.scopes.clone();\n"
    "        let mut predecessors = vec![entry.flow_cursor];\n"
    "        if !executed_never {\n"
    "            predecessors.push(executed.flow_cursor);\n"
    "        }\n"
    "        self.flow_join(predecessors, None);\n"
    "    }\n\n"
    "    fn merge_branch_flow",
    "simplify optional execution merge",
)

source = regex_once(
    source,
    r"    fn merge_branch_flow\(.*?\n    \}\n\n    fn merge_match_flow",
    "    fn merge_branch_flow(\n"
    "        &mut self,\n"
    "        entry: &ScopeFlowState,\n"
    "        then_state: &ScopeFlowState,\n"
    "        then_never: bool,\n"
    "        else_state: &ScopeFlowState,\n"
    "        else_never: bool,\n"
    "    ) {\n"
    "        self.scopes = entry.scopes.clone();\n"
    "        let mut predecessors = Vec::with_capacity(2);\n"
    "        if !then_never {\n"
    "            predecessors.push(then_state.flow_cursor);\n"
    "        }\n"
    "        if !else_never {\n"
    "            predecessors.push(else_state.flow_cursor);\n"
    "        }\n"
    "        if predecessors.is_empty() {\n"
    "            self.set_flow_cursor(entry.flow_cursor);\n"
    "        } else {\n"
    "            self.flow_join(predecessors, None);\n"
    "        }\n"
    "    }\n\n"
    "    fn merge_match_flow",
    "simplify branch merge",
)

source = regex_once(
    source,
    r"    fn merge_match_flow\(.*?\n    \}\n\n    fn constant_int_failure",
    "    fn merge_match_flow(\n"
    "        &mut self,\n"
    "        entry: &ScopeFlowState,\n"
    "        branches: &[(ScopeFlowState, bool)],\n"
    "    ) {\n"
    "        self.scopes = entry.scopes.clone();\n"
    "        let predecessors = branches\n"
    "            .iter()\n"
    "            .filter(|(_, never)| !never)\n"
    "            .map(|(state, _)| state.flow_cursor)\n"
    "            .collect::<Vec<_>>();\n"
    "        if predecessors.is_empty() {\n"
    "            self.set_flow_cursor(entry.flow_cursor);\n"
    "        } else {\n"
    "            self.flow_join(predecessors, None);\n"
    "        }\n"
    "    }\n\n"
    "    fn constant_int_failure",
    "simplify match merge",
)

source = regex_once(
    source,
    r"    fn insert_local\(&mut self, binding: &hir::Binding, initialized: bool\) \{.*?\n    \}\n\}\n\n#\[cfg\(test\)\]",
    "    fn insert_local(&mut self, binding: &hir::Binding) {\n"
    "        let scope = self\n"
    "            .scopes\n"
    "            .last_mut()\n"
    "            .expect(\"semantic analysis must always have a lexical scope\");\n"
    "        if let Some(previous) = scope.get(&binding.name) {\n"
    "            self.diagnostics.push(\n"
    "                Diagnostic::error(\"N3002\", \"duplicate definition\")\n"
    "                    .with_primary(\n"
    "                        binding.span,\n"
    "                        format!(\"`{}` is already defined in this scope\", binding.name),\n"
    "                    )\n"
    "                    .with_secondary(previous.span, \"first definition is here\"),\n"
    "            );\n"
    "            return;\n"
    "        }\n"
    "        scope.insert(\n"
    "            binding.name.clone(),\n"
    "            LocalSymbol {\n"
    "                id: binding.id,\n"
    "                ty: binding.ty.clone(),\n"
    "                mutable: binding.mutable,\n"
    "                span: binding.span,\n"
    "            },\n"
    "        );\n"
    "    }\n"
    "}\n\n"
    "#[cfg(test)]",
    "make lexical insertion flow-independent",
)

ANALYZER.write_text(source)

lib = LIB.read_text()
lib = replace_once(lib, "mod flow_rules;\n", "", "remove flow_rules module")
LIB.write_text(lib)

control_doc = CONTROL_DOC.read_text()
control_doc = replace_once(
    control_doc,
    "The graph currently owns diagnostic `N3009` (a local may be uninitialized).\nThe structured lowerer temporarily retains a parallel Boolean initialization\nfact only to choose recovery HIR types after an erroneous read. That recovery\nfact does not emit `N3009` and is not the accepted-program proof. Removing this\ntransitional duplication requires a later separation of HIR construction from\nall flow-dependent recovery typing.\n",
    "The graph exclusively owns definite-initialization state and diagnostic `N3009`.\nA resolved binding read keeps the binding's declared HIR type even when CFG analysis\nlater rejects that read as maybe uninitialized. Type checking and flow checking are\ntherefore orthogonal: an independently ill-typed use may report its ordinary type\ndiagnostic alongside `N3009`, rather than relying on a hidden inline initialization\nbit to turn the read into recovery `<error>`.\n",
    "control-flow ownership paragraph",
)
control_doc = replace_once(
    control_doc,
    "The next architectural step should separate recovery HIR typing from the legacy\ninline initialization flag, then migrate additional flow-sensitive checks only\nwhen each has a specified lattice, verifier invariants, and adversarial tests.\n",
    "Additional flow-sensitive checks should migrate onto explicit analyses only when\neach has a specified lattice, verifier invariants, and adversarial tests; lexical\nresolution and HIR typing must not grow parallel hidden flow facts again.\n",
    "control-flow next-step paragraph",
)
CONTROL_DOC.write_text(control_doc)

constitution = CONSTITUTION.read_text()
constitution = replace_once(
    constitution,
    "Reads must not observe an\nuninitialized local value; accepted programs require compile-time evidence that\na delayed mutable binding has been initialized on every control-flow path that\ncan reach the read.\n",
    "Reads must not observe an\nuninitialized local value; accepted programs require compile-time evidence from the\nverified function CFG that a delayed mutable binding has been initialized on every\ncontrol-flow path that can reach the read. Binding HIR keeps its declared type\nindependently of that proof, so a maybe-uninitialized read can still participate in\nordinary type checking and can receive both an independent type diagnostic and\n`N3009` when both rules are violated.\n",
    "constitution initialization ownership",
)
CONSTITUTION.write_text(constitution)

roadmap = ROADMAP.read_text()
roadmap = replace_once(
    roadmap,
    "**Status: twenty-seven vertical slices implemented; broader type-system work remains.**",
    "**Status: twenty-eight vertical slices implemented; broader type-system work remains.**",
    "roadmap status",
)
marker = "\nThe next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n\n- separate recovery HIR typing from the remaining inline initialization flag now that\n  `N3009` proof and diagnostics are owned by the explicit CFG;\n"
section = "\nImplemented in the twenty-eighth Phase 2 slice:\n\n- lexical `LocalSymbol` state no longer carries a parallel `initialized` Boolean;\n  definite initialization exists only as verified CFG events and fixed-point facts;\n- resolved binding reads always retain their declared HIR type, while `N3009` remains\n  independently produced by CFG dataflow, allowing orthogonal type diagnostics when\n  source violates both typing and initialization rules;\n- the obsolete structured `InitializationJoin` lattice and `flow_rules` module are\n  removed, and branch/loop/match helpers now merge only CFG continuation cursors;\n- initialized parameters, completed declarations, payload bindings, and successful\n  assignments emit explicit `Initialize` nodes without mutating lexical symbols;\n- a declaration whose initializer is already non-continuing still enters lexical scope\n  for deterministic dead-source diagnostics but emits no impossible execution\n  initialization after the transfer, eliminating the corresponding N3999 graph error;\n- semantic regression tests lock HIR type preservation, independent N3004/N3009\n  reporting, and noncontinuing-initializer graph validity; and\n- the CFG is now the single source of truth for definite initialization in both accepted\n  and recovery analysis, completing the transition introduced by slice twenty-seven.\n\nThe next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n"
roadmap = replace_once(roadmap, marker, section, "roadmap slice 28")
ROADMAP.write_text(roadmap)

readme = README.read_text()
readme = replace_once(
    readme,
    "- Function CFGs are verified before publication; `N3009` is produced by their\n  fixed-point must analysis rather than ad-hoc diagnostic emission during name lookup.\n",
    "- Function CFGs are verified before publication; `N3009` is produced by their\n  fixed-point must analysis rather than ad-hoc diagnostic emission during name lookup.\n- Definite initialization has no parallel lexical Boolean: binding HIR preserves the\n  declared type while CFG read/initialize events exclusively own flow validity.\n",
    "README CFG single-source invariant",
)
README.write_text(readme)

flow_rules = Path("crates/nova-sema/src/flow_rules.rs")
if not flow_rules.exists():
    raise SystemExit("flow_rules.rs missing before cleanup")
flow_rules.unlink()
