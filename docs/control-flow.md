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

The graph exclusively owns definite-initialization state and diagnostic `N3009`.
A resolved binding read keeps the binding's declared HIR type even when CFG analysis
later rejects that read as maybe uninitialized. Type checking and flow checking are
therefore orthogonal: an independently ill-typed use may report its ordinary type
diagnostic alongside `N3009`, rather than relying on a hidden inline initialization
bit to turn the read into recovery `<error>`.

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
A `while` whose condition is rejected as non-`Bool` may likewise retain body
nodes for static and lexical loop-control diagnostics, but that recovery-only
body is not an executable loop iteration. Neither ordinary body fallthrough nor
a retained `continue` receives a `Backedge` successor to the condition header;
rejected control flow must not reconnect discarded recovery paths to reachable
continuation.

## Verification

Before a graph becomes part of `AnalysisOutput`, the verifier rejects it unless:

- entry and predecessor identities are in range;
- node identities equal their deterministic vector positions;
- only the entry lacks predecessors;
- every node is graph-reachable from the entry, including diagnostic source;
- every node reachable from entry without crossing a `Diagnostic` edge has only
  non-diagnostic predecessors that are themselves reachable on such executable flow;
- every read/initialization event names graph binding metadata;
- the normal-exit table contains each `Exit` node exactly once and no other node;
- every declared normal exit is executable-reachable and every `Exit` is terminal;
- `return`, `break`, and `continue` successor edge classes respect their transfer
  behavior; and
- a syntactic parent transfer does not append an execution node when evaluating its
  child expression has already transferred control.

An internal verification failure is fail-closed diagnostic `N3999`; no invalid
graph is published for that function.
The verifier computes this executable-reachability set independently of lowering.
This makes diagnostic isolation a graph invariant rather than an analyzer convention:
a discarded recovery subtree may branch away from executable flow, but it cannot feed
an executable join, exit, or loop header through any edge class. The fixed-point
must-analysis can therefore safely intersect every recorded predecessor without
allowing recovery-only facts to constrain reachable continuation.
Normal completion has a similarly closed contract. `normal_exits` is not advisory
metadata: it must exactly enumerate the graph's `Exit` nodes without duplicates, each
such node must belong to executable-reachable flow, and an `Exit` has no successor of
any edge class. A diagnostic-only recovery path therefore cannot be mislabeled as a
successful function completion, and post-exit diagnostic nodes cannot extend a graph
past its terminal boundary.

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

Additional flow-sensitive checks should migrate onto explicit analyses only when
each has a specified lattice, verifier invariants, and adversarial tests; lexical
resolution and HIR typing must not grow parallel hidden flow facts again.
