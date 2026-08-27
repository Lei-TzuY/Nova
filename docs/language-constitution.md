# Nova Language Constitution

Status: **design constitution for Nova v0.1**
Last revised: 2026-08-28

This document records durable design constraints for Nova. It is not a claim
that every described property has been implemented, and it is not a substitute
for the normative grammar or future language specification.

Labels used below:

- **Decided**: a v0.1 direction that implementations must not contradict.
- **Provisional**: a concrete bootstrap choice that may change through the
  compatibility process.
- **Research**: an open question; documentation and code must not imply that it
  is solved.

## 1. Purpose and success criteria

**Decided.** Nova is a general-purpose, statically typed language optimized for
an unusually strong trade-off frontier:

- source readability and iteration speed comparable to high-level languages;
- safety properties suitable for systems and concurrent software;
- predictable access to native performance and low-level facilities;
- diagnostics and semantic data useful to both people and automated tooling;
- one official package, build, test, format, lint, and documentation workflow;
- reproducible builds and explicit compatibility boundaries; and
- a path to native, WebAssembly, SIMD, GPU, and interactive execution without
  splitting Nova into incompatible dialects.

No single metric dominates these goals. A feature is justified when its total
semantic and tooling cost improves the frontier, not because another language
has it.

## 2. Non-goals

**Decided.** Nova does not aim to:

- preserve source compatibility with Python, Rust, C, or C++;
- expose every backend feature directly in surface syntax;
- make all valid programs allocation free, real-time safe, or data-parallel;
- hide allocation, blocking, failure, unsafety, or platform dependence;
- guarantee that every abstraction is zero cost;
- accept ambiguous or unsupported programs by guessing user intent;
- stabilize unfinished research behind confident terminology; or
- accumulate multiple competing official build and package systems.

## 3. Syntax and lexical structure

**Decided.** Syntax should be visually quiet, regular, and locally readable.
Blocks use braces so formatting is not semantic. Newline is whitespace rather
than an automatic statement terminator. Semicolons distinguish statements from
the optional value-producing tail expression of a block.

**Provisional bootstrap decisions.** The implemented subset uses UTF-8 source,
ASCII identifiers, decimal integer literals, `//` line comments, and nested
`/* ... */` block comments. Keywords are reserved. Files with malformed UTF-8
are rejected before lexing. The compact normative details are in
[`grammar.md`](grammar.md).

**Decided.** Parsers fail closed. A parser may recover to report more errors,
but recovery must never manufacture a valid construct whose meaning differs
from the written source. All accepted syntax has a documented precedence and
associativity.

## 4. Values and types

**Decided.** Static typing is the default. Type inference should remove
redundancy without making public interfaces or effects opaque. Optional values
are represented explicitly; there is no implicit null inhabitant of every
reference-like type. Algebraic data types and exhaustive pattern matching are
core language directions, not library conventions.

**Provisional bootstrap decisions.** The current semantic core recognizes the
surface type names `Int` and `Bool`, resolves explicitly typed function
signatures, infers initialized local binding types, and checks the implemented
operators, calls, branches, returns, assignments, and definite initialization.
Accepted integer literal magnitude is currently `0..=2^63-1`; unary `-` is a
separate expression, so the most-negative signed 64-bit value has no literal
spelling in this subset.

The bootstrap interpreter provisionally executes `Int` as signed 64-bit values
with checked arithmetic. Arithmetic overflow, division by zero, and remainder
by zero fail with structured runtime diagnostics; execution never inherits
host debug/release overflow behavior. This is implementation evidence for the
numeric design, not yet a stable language-wide promise about numeric widths,
defaulting, conversions, or overflow policy for future backends.

**Research.** The project must decide, with implementation evidence:

- the primitive numeric set and defaulting rules;
- whether value restriction or another rule is needed for inference;
- layout guarantees for user-defined types; and
- the boundary between language, standard library, and target-specific types.

## 5. Bindings and mutability

**Decided.** `let` introduces an immutable binding and `var` introduces a
mutable binding. Mutability belongs to a binding or explicitly mutable view; it
must not spread invisibly through an object graph. Reads must not observe an
uninitialized local value; accepted programs require compile-time evidence that
a delayed mutable binding has been initialized on every control-flow path that
can reach the read.

**Provisional bootstrap decisions.** `let` always requires an initializer.
`var` may either be initialized immediately, with an optional type annotation,
or declared as `var identifier: Type;` for later initialization. A delayed
`var` therefore requires an explicit type; untyped `var identifier;` is
rejected. No runtime default value is manufactured.

Assignment remains deliberately narrow: `identifier = expression;` is a
statement rather than an expression, its target must resolve to a lexical
`var`, and the replacement value must preserve the binding's established type.
Function parameters and `let` bindings are immutable. Definite-initialization
state is propagated through lexical blocks and merged across `if` branches. If
both branches can continue, a binding is definitely initialized afterward only
when both continuing paths initialize it; a branch that cannot continue because
it returns does not constrain the surviving path. Chained assignment, arbitrary
lvalues, fields, indexing, and general uninitialized storage remain unsupported.

**Research.** Broader flow-sensitive facts, loops, pattern bindings, partial
aggregate initialization, ownership interactions, and diagnostics for more
complex control-flow graphs require implementation evidence before their rules
are frozen.

## 6. Names, modules, and packages

**Decided.** Name resolution will be lexical, deterministic, independent of
filesystem enumeration order, and separate from type inference. Imports must
make dependency edges inspectable. Packages and modules must have stable
identity rules that work in reproducible builds.

**Research.** File-to-module mapping, visibility defaults, namespace separation,
cyclic module handling, package manifests, lockfiles, and registry trust policy
remain unresolved. The compiler must not bake provisional filesystem behavior
into semantic identity.

## 7. Errors and effects

**Decided.** Recoverable failure is typed and visible in interfaces. Nova will
not use invisible, unchecked exception propagation as its ordinary error model.
The language will distinguish recoverable errors, process-level failure, and
violated internal invariants.

**Research.** Effect polymorphism, effect-row representation, cancellation,
panic semantics, and the interaction of effects with ABI boundaries require
prototypes before syntax is selected. The bootstrap subset contains no effect
syntax.

## 8. Memory and resource model

**Decided.** Memory safety is the default target, while resource lifetimes must
remain deterministic where correctness requires it. Operations that can break
language invariants require explicit unsafe authority.

**Research.** Nova's proposed hybrid model—compiler-inferred ownership or
regions for ordinary values, deterministic ownership for resources, and
optional managed regions for graph-shaped shared data—is not solved. Open work
includes aliasing, destruction order, region inference, cycles, pinning, FFI
roots, real-time constraints, and the cost model. Until a checked model exists,
Nova must not claim memory safety or zero-cost ownership.

## 9. Concurrency

**Decided.** Concurrency should be structured: spawned work has an explicit
scope, cancellation behavior, and join obligation. Data-race freedom is a
language-model goal rather than merely a library guideline. Async and AOT code
must share the same observable language semantics.

**Research.** Task ownership, executor abstraction, cancellation safety,
structured parallelism, `Send`-like constraints, shared mutable state, and
blocking interoperability remain open. The bootstrap subset contains no
concurrency syntax.

## 10. Unsafe capabilities and interoperability

**Decided.** Unsafe code must be syntactically explicit and narrowly auditable.
The intended direction is capability classification such as `unsafe(ffi)`,
`unsafe(memory)`, and `unsafe(pointer_arithmetic)`, rather than one undifferenced
escape hatch. Classification syntax and granularity are not yet specified.

**Decided.** C interoperability is a first-class goal, including explicit ABI,
layout, ownership, error, and unwind boundaries. It is not permission to make C
semantics the default Nova semantics.

**Research.** Capability composition, trusted intrinsics, provenance, variadic
calls, callbacks, unwinding, and bindgen policy require dedicated design work.

## 11. Compilation and execution model

**Decided.** The intended compiler pipeline is:

```text
Source -> tokens -> AST -> HIR -> resolution -> type/effect inference
       -> ownership/region analysis -> MIR -> optimization -> backend
```

The exact pass boundaries may change, but surface parsing must not become the
owner of type, effect, execution, or target semantics. Native, interactive,
WebAssembly, and GPU execution must eventually consume well-defined shared
semantic contracts and agree on observable language behavior. Target-specific
restrictions must be diagnosed, not silently translated into different
semantics.

**Provisional bootstrap decisions.** `nova run` executes only after lexical,
syntactic, name-resolution, type, and definite-assignment validation succeeds.
The first interpreter consumes typed HIR directly and supports the implemented
function, call, block, `if`, return, binding, assignment, Boolean, and integer
subset. Evaluation order is left-to-right; `&&` and `||` short-circuit. The
entry point is a zero-argument top-level `main` returning `Int` or `Bool`.
Runtime failures use structured diagnostics and recursive execution is guarded
by a finite call-depth limit. These choices provide an executable oracle for
the current subset; HIR interpretation is not the intended final backend ABI.

**Research.** HIR and MIR forms, verification rules, optimization contracts,
debug information, incremental compilation, monomorphization, backend
selection, stable entry-point conventions, and cross-backend execution
conformance remain open.

## 12. Diagnostics and tooling contracts

**Decided.** Diagnostics are structured compiler data first and rendered text
second. Stable diagnostic codes, exact source spans, primary and secondary
labels, notes, and machine-readable output are required directions. Recovery
diagnostics must be deterministic for identical input and compiler version.

**Provisional bootstrap decisions.** The current toolchain uses half-open UTF-8
byte spans and exposes human and JSON Lines rendering across lexical, syntactic,
semantic, and runtime diagnostics. Diagnostic code meaning is documented by
tests but codes are not yet covered by the language compatibility promise.

Semantic introspection for editors and AI systems must ultimately expose
resolved symbols, types, effects, ownership facts, and transformations through
versioned schemas. Compiler debug text is not that eventual protocol.

## 13. Compatibility and versioning

**Decided.** Compatibility is more important than feature count. Language,
standard-library, package-manifest, IR, and tooling-schema versions are distinct
contracts. Stabilization requires a written specification, conformance tests,
implementation experience, and a migration story.

Before Nova 1.0, breaking changes are allowed but must be called out in release
notes and should include mechanical migration tooling when practical. After a
contract is declared stable, silent semantic change is prohibited.

## 14. Implementation invariants

Every compiler and execution stage must uphold these constraints:

1. Source text is validated UTF-8 before tokenization.
2. Spans are source-qualified, half-open byte ranges on character boundaries.
3. Unsupported input produces a diagnostic and a failing result.
4. No parser recovery loop may repeat without consuming input or terminating.
5. Literal conversion is checked; bootstrap integer execution never wraps or
   truncates because of host build-profile behavior.
6. Nesting and execution-depth limits fail with diagnostics before uncontrolled
   recursion.
7. Iteration order that affects output is explicit and deterministic.
8. An implemented grammar, semantic, or execution rule has positive and
   negative tests.
9. A local read cannot observe an uninitialized binding; delayed initialization
   must be proven on every reachable continuing path before the read.
10. Optimization must preserve specified behavior and later operate on verified
    IR rather than repair invalid earlier output.
11. Roadmap documents distinguish implemented, provisional, and researched
    properties; benchmarks and safety claims require reproducible evidence.

## 15. Current unresolved research register

The highest-impact unresolved questions are:

- inference boundaries and public type annotation policy;
- primitive numeric semantics across all execution backends;
- typed error and effect representation;
- the hybrid ownership/region/managed-memory model;
- data-race freedom and cancellation in structured concurrency;
- HIR/MIR contracts shared across execution modes and targets;
- stable ABI and C ownership conventions;
- deterministic, incremental, reproducible package builds; and
- versioned semantic-introspection schemas.

These questions intentionally have no stable surface commitment in the current
bootstrap subset.
