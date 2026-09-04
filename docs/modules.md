# Module-ready identity contract

Status: **implemented compiler foundation; no module or import syntax**

This document specifies the module boundary already represented by Nova's semantic
pipeline. It deliberately does not specify source module paths, imports, exports,
visibility, filesystem discovery, packages, or cross-module linking.

## Source and parser meaning

Every source accepted by the current CLI remains one complete compilation unit. The
implemented grammar has no `module`, `import`, or qualified module-path construct, so
the parser produces the same source-file AST as before this contract.

The CLI asks semantic analysis to place that AST in `ModuleId::ROOT`. Compiler clients
preparing a future multi-source loader may instead call `analyze_in_module` with a
session-assigned `ModuleId`. The caller supplies identity only; a numeric ID does not
invent a source spelling, filesystem location, dependency edge, or visibility rule.

## Resolution and typed HIR

Semantic analysis owns an explicit per-module declaration scope. Records and enums
share that module's type namespace, while functions use its value namespace. Built-in
types remain language-defined and cannot be redefined. Forward function references and
forward or recursive nominal type references still resolve after the corresponding
module namespace has been collected.

`FunctionId`, `RecordId`, `EnumId`, `ClosureId`, and `BindingId` are pairs of:

- the owning `ModuleId`; and
- the declaration-, analysis-, or traversal-order index within that module.

Consequently, `module:3/function:0` and `module:4/function:0` are different semantic
declarations even though their local indices match. Nominal `Record` and `Enum` types
carry those qualified identities. Function references, aggregate targets, pattern
targets, closure capture tables, and binding references retain them rather than
re-resolving source spellings later.

The HIR `Program` currently contains exactly one `Module` and requires its span to equal
the complete program span. This one-module container is an honest bootstrap limit, not
a representation of an import graph.

## Control flow and type flow

Every function CFG, closure CFG, flow binding, and binding read/initialize event must
belong to the same module as its callable owner. Module qualification does not change
reachability, type joins, definite initialization, or `!` propagation. It prevents a
same-index declaration from another module from being admitted as if it were local.

The CFG verifier rejects a cross-module binding event as invalid internal state. It does
not use the local index after the module check fails, so malformed data cannot inject
initialization facts for another declaration.

## Interpreter boundary

The bootstrap interpreter supports any internally consistent session-assigned module,
including the root module used by `nova run`. Before indexing declaration tables or
materializing a closure environment, it independently checks that function, record,
enum, closure, and binding identities belong to the program module.

A forged identity with a valid local index but a different module fails closed with
runtime invariant diagnostic `N4005`. It cannot retarget a call, aggregate, pattern,
capture, or local access. Module qualification does not define runtime layout, linkage,
symbol mangling, allocation, ownership, or ABI.

## Semantic inspection

Inspection schemas v1 through v5 retain their existing document-local IDs and byte
shape for root-module programs. They reject a non-root HIR module with `N5001` rather
than silently erase its ownership.

Explicit schema v6 preserves all v5 facts and adds one `module` object containing:

- the compiler-session module ID and associated source ID;
- whether it is the current CLI's implicit root;
- the complete module span; and
- the record, enum, function, binding, and closure IDs owned by it.

Schema v6 also validates HIR and CFG module ownership independently. Its module ID is
session-local inspection evidence, not a stable package coordinate or import path.

## Deliberately unresolved

The following remain research or later vertical slices:

- source syntax for module paths and imports;
- file-to-module mapping and multi-file compilation;
- exported and private declarations;
- namespace lookup across dependency edges;
- cycles, initialization order, incremental keys, and reproducible module graphs;
- package manifests, version selection, registries, and lockfiles; and
- native linker names, layout, ABI, ownership, or memory-safety guarantees.

Any future import slice must build and verify a module graph before resolution. It must
not reinterpret source paths or filesystem enumeration order as semantic identity.
