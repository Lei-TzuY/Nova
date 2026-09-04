# Nova semantic introspection v6

Status: **implemented, provisional tooling contract**

Schema v6 extends v5 with explicit ownership by the single module represented in the
current semantic program. Select it with:

```console
nova inspect --format json --schema-version 6 program.nv
```

The schema is published at
[`schemas/semantic-inspection-v6.schema.json`](schemas/semantic-inspection-v6.schema.json).

## Compatibility

V6 preserves v5's `program`, function `control_flow`, `match_patterns`, `closures`, and
`closure_control_flow` projections. It adds the required top-level `module` field. V1
remains the CLI default; no caller receives v6 implicitly.

V6's published type/expression enums and immutable-source capture relation remain
frozen. An accepted program containing `UInt` or a closure that snapshots an enclosing
mutable `var` therefore fails v6 inspection with `N5001` and no partial document; callers
must explicitly select
[`schema v7`](semantic-introspection-v7.md). This is a versioned tooling limitation,
not rejection of the source by semantic analysis or execution.

For the CLI's implicit root module, v1-v5 output remains unchanged. A compiler client
that analyzes an AST under a non-root `ModuleId` must request v6: older schemas fail
closed with inspection diagnostic `N5001` because they cannot publish module ownership.

## Module fact

The `module` object contains:

- `id`: `module:N`, where `N` is the compiler-session `ModuleId`;
- `source`: the document-local source ID, currently `source:0`;
- `implicit_root`: whether the module is `ModuleId::ROOT`;
- `span`: the complete source/module span; and
- `records`, `enums`, `functions`, `bindings`, and `closures`: complete ordered lists
  of document-local identities owned by the module.

The lists are exhaustive for the v6 document. Their identities retain the established
v1-v5 spelling because the document still contains one module; the enclosing module
fact supplies their qualification.

## Validation

The builder emits no document unless:

- semantic analysis succeeded;
- the HIR module span equals the program span and both belong to the inspected source;
- every declaration, local, closure, nominal type, resolved reference, capture, CFG
  owner, flow binding, and binding event carries the same `ModuleId`; and
- all earlier structural, type, scope, capture, match, and CFG invariants pass.

Failure produces `N5001` and no partial JSON. Output order and pretty printing remain
deterministic.

## Non-claims

V6 describes compiler-session identity only. It does not describe module paths,
imports, exports, dependency graphs, packages, filesystem mapping, linkage, layouts,
ABI, incremental cache keys, cross-module execution, the later `UInt` projection, or
mutable-source snapshot capture semantics.
