# Semantic Introspection v5

Status: **implemented tooling contract for closures and immutable-source captures**

Schema v5 preserves the v4 `program`, top-level `control_flow`, and
`match_patterns` meanings. It is the first schema that can publish anonymous-function
expressions without omitting their callable body or lexical environment contract.

Select it explicitly:

```console
nova inspect --format json --schema-version 5 program.nv
```

## Additions

- `program.expressions[].kind` may be `closure`; its `target` is `closure:N`.
- `closures` lists every anonymous callable in deterministic semantic preorder. Each fact
  links the creating expression, structural function type, explicit return type, parameter
  bindings, body block, and complete span.
- `closures[].captures` lists existing outer binding IDs in first lexical-use order. Every
  entry includes its resolved type and first-use span. Captures do not manufacture duplicate
  binding declarations.
- binding, block, statement, expression, and match `owner` values may be either
  `function:N` or `closure:N`.
- `closure_control_flow` publishes one verified CFG per closure. Its node and edge meanings
  are identical to v2, but identities use `cfg:closure:N.node:M`; the binding table contains
  both closure-owned bindings and the immutable outer bindings admitted by the capture table.

The normative JSON Schema is
[`schemas/semantic-inspection-v5.schema.json`](schemas/semantic-inspection-v5.schema.json).

## Fail-closed rules

Inspection rejects rather than repairs any closure whose ID order, expression/signature,
span, parameter ownership, capture name/id/span/type, first-use order, free-reference set,
or CFG binding table disagrees. A cross-callable binding reference is valid only when the
innermost closure's capture table names that exact immutable binding. Extra captures and
missing captures are both invariant failures.

Schemas v1 through v4 remain frozen and return CLI diagnostic `N5001` for a program that
contains a closure. Schema v5 must be requested explicitly; no existing document changes
shape or interpretation merely because the compiler learned closure semantics. V5 also
remains frozen after mutable-source snapshot reads were added: such a capture requires
schema v7, whose explicit capture mode avoids widening v5's admitted binding relation.
