# Bootstrap Control-Flow Contract

This document specifies the function-level control-flow graph (CFG) implemented
by `nova-sema`. It is a compiler contract for the current executable subset, not
a stable serialization format, MIR, backend IR, or promise about future public
compiler APIs.

## Purpose and boundary

The analyzer builds one CFG per HIR function while it lowers source. Building
the graph during lowering is deliberate: rejected aggregate/operator/call paths
and statically skipped source may lose executable HIR structure, but Nova must
still retain those paths for deterministic diagnostics without exporting their
facts into reachable continuation.

The graph currently owns diagnostic `N3009` (a local may be uninitialized).
The structured lowerer temporarily retains a parallel Boolean initialization
fact only to choose recovery HIR types after an erroneous read. That recovery
fact does not emit `N3009` and is not the accepted-program proof. Removing this
transitional duplication requires a later separation of HIR construction from
all flow-dependent recovery typing.

CFG data is exposed as a read-only Rust model on `AnalysisOutput`. It is not part
of semantic-inspection schema v1; changing that tooling schema requires an
independent versioned proposal.

## Graph shape

Each graph has one `Entry`, zero or one normal `Exit` in the current structured
lowerer, and deterministic graph-local node identities. Nodes represent:

- branch-path entry and continuation joins;
- binding initialization and resolved binding reads;
- `return`, `break`, and `continue` transfers; and
- normal function completion.

Predecessor edges are recorded on each node. `Execution` edges represent a
possible reachable continuation, `Diagnostic` edges retain statically checked
source whose facts are discarded before reachable continuation, and `Backedge`
edges return loop fallthrough or `continue` to the pre-test condition header.
Closed side-effect-free condition refinements for `if`, `while`, and
short-circuit operators, direct enum-constructor match selection, and
non-continuing discriminators use diagnostic edges for successors that cannot
execute.

An invalid construct may leave a reachable-from-entry diagnostic subgraph with
no continuation edge. This is intentional: rollback moves the lowering cursor
back to the last valid state, but does not erase source events or binding
identities needed for diagnostics.

## Verification

Before a graph becomes part of `AnalysisOutput`, the verifier rejects it unless:

- entry and predecessor identities are in range;
- node identities equal their deterministic vector positions;
- only the entry lacks predecessors;
- every node is graph-reachable from the entry, including diagnostic source;
- every read/initialization event names graph binding metadata;
- declared normal exits name `Exit` nodes; and
- `return`, `break`, `continue`, and `Exit` successor edge classes respect their
  transfer behavior.

An internal verification failure is fail-closed diagnostic `N3999`; no invalid
graph is published for that function.

## Definite-initialization dataflow

For binding set `B`, the solver starts non-entry nodes at `B` and iterates
downward to a fixed point. For each node `n`:

```text
IN[entry] = {}
IN[n]     = intersection(OUT[p] for each predecessor p)
OUT[n]    = IN[n] union {b}   when n initializes b
OUT[n]    = IN[n]             otherwise
```

A read of `b` is accepted only when `b` belongs to `IN` at that read. Starting
from the binding universe computes the conservative must-analysis fixed point;
the function-entry path prevents loop-only initialization from becoming a fact
on a zero-iteration exit. Parameters and initialized declarations create
explicit initialization nodes. A successful, mutable, type-correct assignment
does the same.

Diagnostic-only reads are still checked and can produce `N3009`, matching Nova's
policy that unreachable source receives deterministic static diagnostics.
Diagnostic-only initialization nodes remain confined to discarded paths and are
never joined into a reachable continuation by the structured builder.

## Deliberate limitations

The CFG currently carries only facts needed for local definite initialization.
It does not encode value SSA, dominance, liveness, borrow/region facts, effects,
exceptions, async suspension, pattern usefulness, optimization legality, or
backend blocks. Loop reasoning remains the documented bootstrap rule: ordinary
pre-test loops preserve the zero-iteration path, while a side-effect-free closed
condition proven true may continue only through reachable `break` exits.

The next architectural step should separate recovery HIR typing from the legacy
inline initialization flag, then migrate additional flow-sensitive checks only
when each has a specified lattice, verifier invariants, and adversarial tests.
