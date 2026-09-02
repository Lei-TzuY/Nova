# Semantic Introspection v2

This document specifies Nova's semantic-introspection schema version 2. It
preserves every schema-v1 program fact and adds the verified function-level
control-flow graph (CFG) used by bootstrap definite-initialization analysis.
It is a tooling protocol for successfully checked, single-source programs, not a
serialization of HIR, MIR, a backend IR, or an ABI.

The normative structural schema is
[`schemas/semantic-inspection-v2.schema.json`](schemas/semantic-inspection-v2.schema.json).
Because v2 reuses the complete v1 `producer`, `source`, and `program` contracts
by schema ID, consumers validating v2 must register both the v1 and v2 schema
files. The v1 file remains checked in and independently supported by the CLI.

## Invocation and compatibility

```text
nova inspect <file|-> --format json --schema-version 2 [--source-name name] [--message-format human|json] [--fail-on-warnings]
```

Omitting `--schema-version` continues to select v1. Explicit
`--schema-version 1` produces the same byte-for-byte v1 output as the default;
v2 is never selected implicitly. Unsupported versions are command-line errors.
Source naming follows the v1 contract: stdin defaults to `<stdin>`, and its display metadata
may be replaced with `--source-name` without changing source text or schema semantics.

Both versions use the schema family `nova.semantic-inspection`. Consumers must
check `schema_version` before interpreting a document. V2 adds one required top-level field, `control_flow`, and changes the envelope version
to `2`; it does not add fields to v1 or reinterpret any v1 identity or match-arm field.
The `program` member is the same strict v1 fact table, including its deterministic ordering
and prohibition on unknown fields. Consequently v2 also refuses to reinterpret a
payload-bearing arm with `binding: null` as the later `Variant(_)` discard feature; such
source receives inspection invariant `N5001` and must be inspected with explicit schema v3.

Inspection still runs the complete lexical, syntactic, name-resolution, type,
and definite-initialization pipeline. Rejected source writes diagnostics and no
document. By default, non-fatal warnings remain on standard error and do not prevent an
accepted v2 document. With `--fail-on-warnings`, they retain warning severity while
inspection returns status `1` without emitting a document. Before v2 serialization,
`nova-inspect` also checks that:

- there is exactly one CFG for every HIR function, in function order;
- each CFG and graph-local node identity matches its deterministic slot;
- CFG binding metadata exactly matches the HIR-derived binding ID, owner, name,
  and source span;
- entry, predecessor, binding-event, and normal-exit references are in range and
  have the required category; and
- every published optional span belongs to the inspected UTF-8 source.

A mismatch is invariant diagnostic `N5001`; no partial v2 document is emitted.
The semantic analyzer has already performed its stronger CFG verification before
an accepted analysis can reach this boundary. The inspector's checks protect the
cross-model references that become public in v2.

## Control-flow identities and ordering

V2 adds two identity namespaces:

| Entity | Form | Assignment rule |
| --- | --- | --- |
| Function CFG | `cfg:function:F` | Derived from the v1 `function:F` owner |
| CFG node | `cfg:function:F.node:N` | Graph-local semantic-lowering order |

`control_flow` is in v1 function order. Within a graph, `bindings` are in
semantic binding-identity order, `nodes` are in graph-local identity order,
`normal_exits` are in ascending node order, and each node's incoming edges are
ordered by predecessor node then `execution`, `diagnostic`, `backedge` class.
These identities are document-local. Source edits or a later schema may
renumber them.

Each graph carries:

- `function`, linking to the owning v1 function;
- `entry`, linking to its unique `entry` node;
- `bindings`, the exact v1 bindings participating in that function's flow;
- `normal_exits`, the terminal nodes for ordinary body completion; and
- `nodes`, the verified graph facts.

## Nodes and edges

Stable node kinds are:

| Kind | Meaning | `binding` |
| --- | --- | --- |
| `entry` | Distinguished function-entry lattice root | `null` |
| `branch` | Entry into a conditional, match, or loop path | `null` |
| `join` | Continuing-path intersection or loop header | `null` |
| `initialize` | A binding becomes definitely initialized | v1 binding ID |
| `read` | A resolved binding is read | v1 binding ID |
| `return` | Explicit function return | `null` |
| `break` | Exit from the nearest loop | `null` |
| `continue` | Re-enter the nearest loop at its condition | `null` |
| `exit` | Normal function-body completion | `null` |

Every node carries incoming `predecessors`. An `execution` edge may contribute
facts to runtime-reachable continuation. A `diagnostic` edge retains statically
checked source that cannot contribute facts to executable continuation. A
`backedge` represents executable loop fallthrough or `continue` to a verified
loop-header `join`. The verified graph is direction-canonical: `execution` and
`diagnostic` edges always point from a lower node identity to a higher one, while
`backedge` always points from a higher identity to an earlier `join`. Tooling can
therefore treat `backedge` as the only published cycle-closing edge class rather
than inferring cycles from arbitrary predecessor order. Predecessor lists are also
canonical: every non-`join` node has exactly one incoming edge, only `join` may merge
multiple paths, and no node publishes the same source/edge-class pair twice. A `join`
that receives any `backedge` also publishes at least one earlier `execution` predecessor,
so tooling can rely on every executable loop header retaining its first-entry path.
Executable continuation from a `break` node is likewise canonical: when present it targets
only a `join`; diagnostic successors may retain unreachable source but cannot masquerade as
the loop's executable exit continuation.

Node spans are source-qualified v1 spans when a source action or function
boundary owns the node; compiler-created joins may have `null`. Edge arrays do
not encode source evaluation order. HIR expression and statement relationships
remain the source for evaluation-order facts.

The fixed-point definite-initialization `IN` and `OUT` sets are intentionally
not copied into v2. The graph exposes their verified inputs—binding universe,
initialize/read events, and predecessor classes—without freezing an analysis-
result table that Nova does not yet retain as a compiler artifact.

## Deliberate v2 limits

The CFG is the current bootstrap semantic graph, not SSA or backend blocks. V2
does not expose dominance, liveness, value definitions, ownership or region
facts, effects, exceptions, async suspension, optimization legality, layout,
ABI, runtime values, module graphs, or incremental keys. Diagnostic-only nodes
are present because deterministic static checking is part of Nova's semantics;
their presence does not mean they can execute.

Schema v2 is provisional before Nova 1.0. A field removal, identity-rule change,
edge or match-field reinterpretation, or other incompatible change requires another schema
version rather than a silent v2 mutation. Schema v3 is the first version that represents
explicit enum-payload discard.
