# Semantic Introspection v3

This document specifies Nova semantic-introspection schema version 3. V3 preserves the
established semantic program projection and verified CFG projection while adding an explicit
pattern fact required by payload-discard syntax. It is a tooling protocol, not HIR
serialization, runtime state, layout, or ABI.

The normative structural schema is
[`schemas/semantic-inspection-v3.schema.json`](schemas/semantic-inspection-v3.schema.json).
V3 references the frozen v1 `producer`, `source`, and `program` definitions and the v2
`controlFlowGraph` definition by schema ID. Validators therefore register v1, v2, and v3
schema files together.

## Invocation and compatibility

```text
nova inspect <file> --format json --schema-version 3 [--message-format human|json] [--fail-on-warnings]
```

Omitting `--schema-version` still selects v1. Explicit v2 still means the exact v2 contract.
Nova does not silently upgrade inspection output when source uses a newer language feature.
A program containing `Enum::Variant(_)` is valid for `check` and `run`, but v1/v2 inspection
fails closed with `N5001` because those schemas cannot distinguish an explicit discard from
an invalid missing payload binding without changing the meaning of their existing `binding`
field. Selecting v3 is therefore required for such source.

V3 uses the same schema family `nova.semantic-inspection`, changes `schema_version` to `3`,
retains `program` and `control_flow`, and adds one required top-level table:
`match_patterns`.

Inspection warning policy is schema-independent. By default, warnings remain on standard
error while the document is emitted. With `--fail-on-warnings`, they retain warning severity
while inspection returns status `1` without emitting a document.

## Match-pattern facts

`match_patterns` contains exactly one entry for every arm in the published `program.matches`
tables, in deterministic match order and then written arm order:

```json
{
  "arm": "match:0.arm:1",
  "payload_mode": "discard"
}
```

`arm` reuses the existing document-local match-arm identity. `payload_mode` is one of:

| Mode | Meaning |
| --- | --- |
| `none` | the resolved concrete variant has no payload |
| `bind` | the payload-bearing variant introduces the arm-local binding published in `program.bindings` and referenced by the arm's `binding` field |
| `discard` | the payload-bearing variant was written with `_`; the arm's `binding` remains `null` and no binding fact is created |

The inspector independently checks the HIR variant name/slot identity and payload mode before
publishing these facts. A payload-bearing arm with neither a binding nor explicit discard, a
payload-free arm marked as discard, or a bind/discard contradiction is malformed HIR and
fails with `N5001` rather than being repaired.

The new table is deliberately separate from v1's match-arm object. That keeps the published
v1/v2 meaning of `binding` intact while allowing v3 consumers to interpret `binding: null`
without ambiguity.

## Control flow and execution relationship

V3 carries the same verified CFG shape as v2. Payload discard does not introduce a binding or
an initialize event, so a discard arm contributes no match-payload binding to the CFG binding
universe. Arm reachability, exhaustive concrete-variant coverage, direct-constructor
selection, result-type joining, and definite-initialization behavior otherwise remain the
same as the underlying semantic analysis.

The runtime evaluates the scrutinee exactly once, selects the concrete variant arm, and drops
a selected discarded payload instead of creating a frame slot. This execution fact is not a
runtime-value serialization promise; v3 merely exposes the compiler's checked payload mode.

## Deliberate limits

V3 does not add catch-all/default arms, wildcard variant coverage, guards, nested patterns,
alternative patterns, literals, destructuring, usefulness matrices, ownership facts, layout,
ABI, or runtime values. `_` in the implemented language is only a payload-discard subpattern
inside an already named concrete enum variant.

Schema v3 is provisional before Nova 1.0. Any incompatible reinterpretation still requires a
later schema version rather than mutation in place.
