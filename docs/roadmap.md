# Nova Roadmap

This roadmap orders work by semantic dependency. A phase is complete only when
its implemented behavior is documented, tested, and exercised by CI. Later
phases may be researched in isolation, but they do not become product claims or
stable syntax ahead of their prerequisites.

## Phase 0 — Language constitution

**Status: initial baseline in this repository.**

- goals, non-goals, and design constraints;
- provisional lexical and syntax policy;
- value, mutability, name, error, effect, memory, concurrency, module, package,
  unsafe, compatibility, and implementation directions; and
- an explicit unresolved-research register.

Completion of the initial document does not freeze unfinished semantics.

## Phase 1 — Executable frontend foundation

**Status: first vertical slice implemented; broader grammar work remains.**

- Rust workspace and official `nova` CLI bootstrap;
- source identity, exact spans, and locations;
- structured human and JSON diagnostics;
- lexer and parser for the grammar in `docs/grammar.md`;
- AST inspection with `nova ast`;
- positive, negative, span, precedence, overflow, comment, recovery, and depth
  tests; and
- Rust 1.85 MSRV checking plus current-stable formatting, Clippy, test, build,
  and rustdoc CI.

Next Phase 1 refinements should be driven by the needs of later semantic work,
not by adding unrelated syntax.

## Phase 2 — Semantic core

**Status: three vertical slices implemented; broader type-system work remains.**

Implemented in the first Phase 2 slice:

- a purpose-built resolved, typed HIR in `nova-sema`;
- deterministic source-order function identities and analysis-order binding
  identities;
- lexical scopes with function predeclaration for forward calls and recursion;
- same-scope duplicate rejection, unknown-name diagnostics, and nested-block
  shadowing as an explicit bootstrap policy;
- primitive `Int` and `Bool` type-name resolution;
- function signatures, local initializer inference, optional local annotations,
  calls, operators, block tails, and `if` typing;
- explicit-return checking plus rejection of value-returning functions that can
  fall through;
- semantic diagnostics in the existing human and JSON Lines formats; and
- `nova check` upgraded from syntax validation to semantic validation while
  `nova ast` remains a parser-inspection command.

Implemented in the second Phase 2 slice:

- narrow assignment syntax as the statement `identifier = expression;`;
- assignment kept outside expression precedence, so chaining and embedded
  assignment are rejected by construction;
- `let` bindings and function parameters treated as immutable, while `var`
  bindings are assignable;
- assignment targets resolved lexically to stable binding identities;
- assignment RHS values checked against each binding's established type;
- unknown, function, and immutable assignment targets rejected deterministically
  with semantic diagnostics; and
- parser, semantic, CLI, fixture, example, grammar, constitution, and README
  coverage kept in sync.

Implemented in the third Phase 2 slice:

- typed delayed initialization with `var identifier: Type;`, while `let` and
  untyped `var identifier;` declarations remain rejected;
- per-binding definite-initialization state tracked through semantic analysis;
- diagnostic `N3009` for reads that can observe an uninitialized local;
- successful, type-correct assignment transitions a delayed `var` to initialized;
- `if` branches analyzed from the same entry state and merged by intersection
  across paths that can continue;
- noncontinuing branches such as explicit returns excluded from the surviving
  path's initialization requirement; and
- no runtime default value or silent initialization inserted by the compiler.

The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:

- split and stabilize HIR/resolution/type-analysis/dataflow contracts as
  implementation evidence accumulates;
- define numeric types, defaulting, conversions, and overflow behavior;
- add user-defined records and enums only with resolved type identity and
  deterministic diagnostics;
- introduce exhaustive pattern checking after algebraic data types exist;
- define a versioned semantic-introspection schema rather than exposing debug
  HIR as a tooling protocol; and
- expand negative and adversarial tests as each rule becomes implemented.

Phase 2 is not complete until its implemented type, name, mutation, and dataflow
semantics are sufficiently specified for the executable subset and no roadmap
item is being silently approximated.

## Phase 3 — Executable language subset

**Status: planned.**

- a small verified interpreter or simple execution IR;
- functions and control flow;
- records, enums, and exhaustive pattern matching; and
- deterministic execution and semantic fixture tests.

The backend choice will follow evidence from the Phase 2 HIR rather than
precede it.

## Phase 4 — Typed errors and effects

**Status: research required.** Specify recoverable errors, propagation,
interfaces, effect inference, cancellation, and panic boundaries before adding
surface syntax.

## Phase 5 — Generics and traits

**Status: planned.** Define coherence, inference boundaries, specialization
policy, and compilation strategy with diagnostic quality as a primary metric.

## Phase 6 — Ownership, regions, and resource lifetimes

**Status: research required.** Prototype and measure the hybrid memory model.
Do not label Nova memory safe until accepted programs and unsafe boundaries are
checked by an implemented model with adversarial tests.

## Phase 7 — Stable intermediate representation and native backend

**Status: planned.** Introduce verified MIR, explicit layout and ABI rules,
optimization contracts, debug information, and an initial native backend.

## Phase 8 — Structured concurrency

**Status: research required.** Add scoped tasks, cancellation, race-freedom
rules, and executor contracts only after effects and ownership can express them.

## Phase 9 — Package and build ecosystem

**Status: planned.** One official manifest, resolver, lockfile, package tool,
formatter, linter, test runner, documentation generator, and reproducible build
protocol.

## Phase 10 — WebAssembly backend

**Status: planned.** Reuse verified shared IR; document target restrictions and
component/host interoperability without creating a Wasm-only language dialect.

## Phase 11 — C interoperability

**Status: planned.** Specify ABI, layout, ownership, error, callback, and unwind
boundaries; add narrowly classified unsafe capabilities and conformance tests.

## Phase 12 — SIMD and GPU research

**Status: research required.** Determine which shared-IR abstractions preserve
Nova semantics and which target constraints require explicit APIs or effects.

## Phase 13 — Self-hosting

**Status: long-term.** Begin only after the language subset needed by the
compiler is stable, bootstrap reproducibility is demonstrated, and builds can
compare trusted Rust-bootstrap and Nova-hosted outputs.
