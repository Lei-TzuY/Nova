from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected one anchor in {path}, found {count}: {old[:100]!r}")
    p.write_text(text.replace(old, new, 1))

# Normative grammar: `_` is legal only as the payload subpattern of a concrete enum variant.
replace_once(
    "docs/grammar.md",
    '''match_arm           = enum_pattern , "=>" , expression ;\nenum_pattern        = identifier , "::" , identifier ,\n                      [ "(" , identifier , ")" ] ;\n''',
    '''match_arm           = enum_pattern , "=>" , expression ;\nenum_pattern        = identifier , "::" , identifier ,\n                      [ "(" , payload_pattern , ")" ] ;\npayload_pattern     = identifier | "_" ;\n''',
)

# Enum/pattern semantics.
replace_once(
    "docs/enums-and-patterns.md",
    '''match value {\n    OptionInt::None => 0,\n    OptionInt::Some(inner) => inner,\n}\n''',
    '''match value {\n    OptionInt::None => 0,\n    OptionInt::Some(_) => 1,\n}\n''',
)
replace_once(
    "docs/enums-and-patterns.md",
    '''The scrutinee must have an enum type. Every arm must qualify a variant with that\nsame nominal enum, and every declared variant must occur exactly once. A variant\nwith a payload requires one binding; a payload-free variant forbids a binding.\nThe payload binding is immutable, definitely initialized, and visible only in\nits arm expression. Different arms therefore may reuse the same spelling\nwithout sharing identity.\n''',
    '''The scrutinee must have an enum type. Every arm must qualify a variant with that\nsame nominal enum, and every declared variant must occur exactly once. A payload-bearing\nvariant must either bind its payload with `Variant(name)` or explicitly discard it with\n`Variant(_)`; silently omitting the payload position is still `N3022`. A payload-free\nvariant accepts neither a binding nor `_`. A payload binding is immutable, definitely\ninitialized, and visible only in its arm expression. A discard creates no binding and the\npayload value is unavailable to the arm. Different binding arms may reuse the same spelling\nwithout sharing identity. `_` is not a catch-all arm in this slice: a bare `_ => ...` pattern\nis not implemented and every concrete enum variant must still occur exactly once.\n''',
)
replace_once(
    "docs/enums-and-patterns.md",
    '''HIR assigns each enum an `EnumId` and each variant its declaration-order slot.\nConstructors and match arms retain the source-resolved variant spelling alongside that\nslot, allowing trusted consumers to verify that the two still identify the same declared\nmember. Runtime enum values remain compact and carry only the `EnumId`, variant slot,\nand optional boxed payload; the retained spelling does not become a runtime layout or\nABI field. Semantic inspection independently checks the name/slot pair before publishing\nits existing stable variant IDs, so schema v1/v2 do not gain a new field.\n''',
    '''HIR assigns each enum an `EnumId` and each variant its declaration-order slot.\nConstructors and match arms retain the source-resolved variant spelling alongside that\nslot, allowing trusted consumers to verify that the two still identify the same declared\nmember. Match arms additionally retain whether a payload was explicitly discarded, so a\nmalformed HIR mutation that merely deletes a real binding cannot be reinterpreted as `_`.\nRuntime enum values remain compact and carry only the `EnumId`, variant slot, and optional\nboxed payload; a selected discard arm consumes that payload without creating a frame slot.\n\nSemantic-inspection v1 and v2 keep their published meaning: they do not reinterpret a\npayload-bearing arm with `binding: null` as discard and therefore fail closed with `N5001`\nwhen source uses `Variant(_)`. Explicit schema v3 preserves the established program and CFG\nprojections and adds `match_patterns`, whose `payload_mode` is `none`, `bind`, or `discard`.\nThis makes the new language fact representable without silently mutating older protocol\nversions.\n''',
)
replace_once(
    "docs/enums-and-patterns.md",
    '''This slice has no wildcard, default arm, guard, literal pattern, nested pattern,\nalternative pattern, multi-payload variant, named variant fields, record\ndestructuring, exhaustiveness usefulness warning, or stable enum layout. Enums\nwith payload variants and records do not yet receive recursively derived value\nequality. Those features require separate semantic and diagnostic designs rather\nthan syntactic shortcuts.\n''',
    '''This slice has no catch-all/default arm, guard, literal pattern, nested pattern,\nalternative pattern, multi-payload variant, named variant fields, record destructuring,\npattern-usefulness analysis, or stable enum layout. `_` exists only as the payload-discard\nsubpattern of an already resolved concrete variant; it does not cover other variants. Enums\nwith payload variants and records do not yet receive recursively derived value equality.\nThose features require separate semantic and diagnostic designs rather than syntactic\nshortcuts.\n''',
)

# Constitution: bind-or-discard is a provisional bootstrap decision, not general wildcard semantics.
replace_once(
    "docs/language-constitution.md",
    '''`match` currently\nsupports only qualified variant patterns with an optional single immutable\npayload binding. Every variant of the scrutinee's nominal enum must appear\nexactly once, and every continuing arm must produce a compatible type. The\nscrutinee runs once and only the selected arm runs. These rules establish a\nsmall executable algebraic-data-type core without selecting wildcard, guard,\nnested-pattern, usefulness, layout, or ownership semantics prematurely.\n''',
    '''`match` currently supports only qualified concrete-variant patterns. A payload-bearing\nvariant may introduce one immutable payload binding or explicitly discard that payload with\n`_`; payload-free variants accept neither. Every variant of the scrutinee's nominal enum\nmust still appear exactly once, and every continuing arm must produce a compatible type.\nThe scrutinee runs once and only the selected arm runs. Payload `_` is deliberately not a\ncatch-all variant wildcard, so these rules extend the executable algebraic-data-type core\nwithout selecting default-arm, guard, nested-pattern, usefulness, layout, or ownership\nsemantics prematurely.\n''',
)

# README feature/status and tooling contract.
replace_once(
    "README.md",
    '''  explicit aggregate construction, exhaustive enum matching, field projection,\n''',
    '''  explicit aggregate construction, exhaustive enum matching with payload discard, field projection,\n''',
)
replace_once(
    "README.md",
    '''  plus explicitly selected v2 documents that add the verified CFG without\n  exposing debug HIR or compiler-owned graph layouts as a protocol.\n''',
    '''  plus explicitly selected v2 documents that add the verified CFG and v3 documents\n  that additionally expose explicit match payload modes without reinterpreting v1/v2 fields.\n''',
)
replace_once(
    "README.md",
    '''[the semantic-introspection v1 contract](docs/semantic-introspection.md) and\n[v2 CFG extension](docs/semantic-introspection-v2.md) for the machine-readable\ntooling boundary,\n''',
    '''[the semantic-introspection v1 contract](docs/semantic-introspection.md),\n[v2 CFG extension](docs/semantic-introspection-v2.md), and\n[v3 pattern extension](docs/semantic-introspection-v3.md) for the machine-readable\ntooling boundary,\n''',
)
replace_once(
    "README.md",
    '''variant must occur exactly once. Payload bindings are immutable and scoped to\none arm. HIR retains the source-resolved variant spelling alongside the nominal enum\nidentity and declaration-order slot for both constructors and match arms. Runtime and\nsemantic-inspection consumers recheck that name/slot pair, so malformed HIR cannot\nsilently retarget a constructor or pattern to a same-shaped sibling variant. Wildcards,\nguards, nested patterns, multi-payload variants, equality for payload-bearing enums,\nlayout, and ABI guarantees are not implemented.\n''',
    '''variant must occur exactly once. A payload-bearing arm may bind the payload immutably\nfor that arm or write `_` to discard it without introducing a binding. Payload-free variants\naccept neither form. HIR retains the source-resolved variant spelling/slot and explicit\ndiscard intent, so runtime and inspection trust boundaries reject identity or payload-mode\ndrift instead of treating a deleted binding as discard. `_` here is not a catch-all arm: a\nbare wildcard/default pattern, guards, nested patterns, multi-payload variants, equality for\npayload-bearing enums, layout, and ABI guarantees remain unimplemented. Semantic-inspection\nv1/v2 deliberately reject discard-bearing matches with `N5001`; explicit v3 adds per-arm\n`none`/`bind`/`discard` facts while preserving the older schema meanings.\n''',
)

# Preserve v1/v2 compatibility text explicitly.
replace_once(
    "docs/semantic-introspection.md",
    '''- `matches` records the nominal enum, scrutinee, written arms, resolved variant\n  and payload binding, result types, and `exhaustive: true` proof outcome.\n''',
    '''- `matches` records the nominal enum, scrutinee, written arms, resolved variant\n  and payload binding, result types, and `exhaustive: true` proof outcome. Schema v1's\n  payload-bearing arms always have a binding; it does not reinterpret `binding: null` as\n  the later payload-discard feature.\n''',
)
replace_once(
    "docs/semantic-introspection.md",
    '''Match facts report exhaustive\ncoverage for the implemented qualified single-variant patterns; they do not\npredict future wildcard, guard, or pattern-usefulness models.\n''',
    '''Match facts report exhaustive coverage for the original qualified single-variant\nbind-or-no-payload model. Source that explicitly discards a payload with `Variant(_)` is\nvalid Nova source but cannot be represented by schema v1 without reinterpreting an existing\nfield, so v1 inspection fails closed with `N5001`; callers must explicitly select schema v3.\nSchema v1 still does not predict catch-all/default arms, guards, nested patterns, or future\npattern-usefulness models.\n''',
)
replace_once(
    "docs/semantic-introspection-v2.md",
    '''V2 adds one required\ntop-level field, `control_flow`, and changes the envelope version to `2`; it does\nnot add fields to v1 or reinterpret any v1 identity. The `program` member is the\nsame strict v1 fact table, including its deterministic ordering and prohibition\non unknown fields.\n''',
    '''V2 adds one required top-level field, `control_flow`, and changes the envelope version\nto `2`; it does not add fields to v1 or reinterpret any v1 identity or match-arm field.\nThe `program` member is the same strict v1 fact table, including its deterministic ordering\nand prohibition on unknown fields. Consequently v2 also refuses to reinterpret a\npayload-bearing arm with `binding: null` as the later `Variant(_)` discard feature; such\nsource receives inspection invariant `N5001` and must be inspected with explicit schema v3.\n''',
)
replace_once(
    "docs/semantic-introspection-v2.md",
    '''Schema v2 is provisional before Nova 1.0. A field removal, identity-rule change,\nedge reinterpretation, or other incompatible change requires another schema\nversion rather than a silent v2 mutation.\n''',
    '''Schema v2 is provisional before Nova 1.0. A field removal, identity-rule change,\nedge or match-field reinterpretation, or other incompatible change requires another schema\nversion rather than a silent v2 mutation. Schema v3 is the first version that represents\nexplicit enum-payload discard.\n''',
)

# New v3 protocol contract.
Path("docs/semantic-introspection-v3.md").write_text(r'''# Semantic Introspection v3

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
nova inspect <file> --format json --schema-version 3 [--message-format human|json]
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
''')

# Roadmap counts + slices.
replace_once(
    "docs/roadmap.md",
    "**Status: three vertical slices implemented; broader grammar work remains.**",
    "**Status: four vertical slices implemented; broader grammar work remains.**",
)
phase1_anchor = '''- parser regressions cover recursive signatures and depth failure, while the CLI fixture\n  exercises the syntax through the complete executable pipeline.\n\nNext Phase 1 refinements'''
phase1_add = '''- parser regressions cover recursive signatures and depth failure, while the CLI fixture\n  exercises the syntax through the complete executable pipeline.\n\nImplemented in the fourth Phase 1 slice:\n\n- qualified enum payload patterns accept `_` in the existing single payload slot, so\n  `Enum::Variant(_)` explicitly discards a payload without introducing a new catch-all arm;\n- AST patterns retain discard intent separately from an absent payload position, preserving\n  exact pattern spans and making later trust-boundary validation possible;\n- the syntax remains deliberately concrete-variant-only: bare `_`, guards, nested patterns,\n  alternatives, and default arms are not admitted by the grammar; and\n- parser plus CLI regressions exercise discard syntax through semantic analysis and runtime\n  execution while the existing enum-pattern grammar remains fail closed elsewhere.\n\nNext Phase 1 refinements'''
replace_once("docs/roadmap.md", phase1_anchor, phase1_add)
replace_once(
    "docs/roadmap.md",
    "**Status: fifty vertical slices implemented; broader type-system work remains.**",
    "**Status: fifty-one vertical slices implemented; broader type-system work remains.**",
)
phase2_anchor = '''- focused semantic tests plus an end-to-end `nova run` program lock parameter, return,\n  local-storage, and invocation behavior with the final result `42`.\n\nThe next Phase 2 slices'''
phase2_add = '''- focused semantic tests plus an end-to-end `nova run` program lock parameter, return,\n  local-storage, and invocation behavior with the final result `42`.\n\nImplemented in the fifty-first Phase 2 slice:\n\n- payload-bearing concrete enum patterns may either introduce the existing immutable arm-local\n  binding or explicitly discard that payload with `_`; omission remains `N3022`, and a\n  payload-free variant rejects discard rather than treating `_` as a general wildcard;\n- HIR match arms retain explicit discard intent alongside resolved enum/variant identity, so\n  downstream consumers can distinguish valid discard from a corrupted missing binding;\n- exhaustiveness, duplicate-variant rejection, direct-constructor reachability, CFG shape,\n  result-type joining, and definite-initialization continue to operate on the same concrete\n  variant slots because `_` does not cover additional variants;\n- semantic-inspection v1/v2 remain semantically frozen and fail with `N5001` rather than\n  reinterpreting their existing nullable binding field; explicit schema v3 preserves the\n  program/CFG projections and adds deterministic `none`/`bind`/`discard` match-pattern facts; and\n- semantic, inspection, CLI, schema, and malformed-HIR regressions lock both the new language\n  fact and backward-compatible tooling version boundary without introducing catch-all\n  usefulness semantics.\n\nThe next Phase 2 slices'''
replace_once("docs/roadmap.md", phase2_anchor, phase2_add)
replace_once(
    "docs/roadmap.md",
    "**Status: twenty-one vertical slices implemented; execution surface remains small.**",
    "**Status: twenty-two vertical slices implemented; execution surface remains small.**",
)
phase3_anchor = '''- focused adversarial regressions plus a nested record/enum/match positive control and\n  all-targets Clippy coverage lock the contract without changing HIR shape, semantic-inspection\n  schemas, runtime value representation, syntax, layout, ABI, or valid-source behavior.\n\nNext Phase 3 slices'''
phase3_add = '''- focused adversarial regressions plus a nested record/enum/match positive control and\n  all-targets Clippy coverage lock the contract without changing HIR shape, semantic-inspection\n  schemas, runtime value representation, syntax, layout, ABI, or valid-source behavior.\n\nImplemented in the twenty-second Phase 3 slice:\n\n- a selected payload-bearing match arm may explicitly discard its runtime payload without\n  allocating or initializing an arm-local frame slot;\n- the interpreter validates the resolved payload mode against the concrete variant declaration\n  before dispatch, distinguishing bind, discard, and payload-free arms under the existing\n  `N4005` fail-closed invariant policy;\n- deleting a real payload binding in malformed HIR is not silently treated as discard because\n  explicit discard intent is retained independently; and\n- focused execution regressions plus an end-to-end CLI fixture return `42` through a discarded\n  payload while preserving scrutinee-once evaluation, concrete variant selection, structured\n  control flow, runtime enum representation, layout, and ABI non-claims.\n\nNext Phase 3 slices'''
replace_once("docs/roadmap.md", phase3_anchor, phase3_add)
