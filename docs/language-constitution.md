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
surface types `Int`, `Bool`, `Unit`, and declared nominal record and enum types.
The sole Unit literal is `()`, and a value-less block also produces Unit. Aggregate
identity comes from the declaration rather than shape: separately declared
types remain different even when their fields or variants have identical names
and types. The checker resolves explicitly typed function signatures, infers
initialized local binding types, and checks the implemented operators, calls,
branches, loops, loop-control legality, returns, assignments, aggregate
construction, field projection, exhaustive enum matching, and definite
initialization. A function declared to return `Unit` may complete through a body
with no tail expression; `return ();` is the explicit Unit return. Functions with
other return types still require a compatible value on every continuing path.
`Unit` is reserved alongside `Int` and `Bool` and cannot be redefined as a nominal
record or enum. Unit equality is not part of this bootstrap operator subset.

A bootstrap record declares explicitly typed, uniquely named fields.
`new Record { field: expression, ... }` must initialize every declared field
exactly once with a compatible value. Named initializers may appear in any
order, but their expressions retain written left-to-right evaluation order.
`value.field` resolves against the base value's nominal record identity. Record
equality and field mutation are not part of this slice. HIR carries stable
record identities and resolved field slots, but that is semantic identity—not a
promise about memory layout, padding, calling convention, or ABI.

A bootstrap enum declares at least one uniquely named variant. Each variant has
zero or one explicitly typed payload, and qualified construction must match that
arity and type. Recursive enum payload types are accepted. `match` currently
supports only qualified variant patterns with an optional single immutable
payload binding. Every variant of the scrutinee's nominal enum must appear
exactly once, and every continuing arm must produce a compatible type. The
scrutinee runs once and only the selected arm runs. These rules establish a
small executable algebraic-data-type core without selecting wildcard, guard,
nested-pattern, usefulness, layout, or ownership semantics prematurely.

The bootstrap frontend preserves decimal integer magnitudes through parsing and
assigns signed meaning during semantic lowering. Positive `Int` literals are
`0..=2^63-1`; the magnitude `2^63` is reserved for prefix negation, so
`-9223372036854775808` denotes the exact signed 64-bit minimum. A positive `2^63`
expression is rejected as semantic diagnostic `N3030`, and larger magnitudes are
rejected lexically as `N1004`. No literal is wrapped or truncated.

The bootstrap interpreter provisionally executes `Int` as signed 64-bit values
with checked arithmetic. Signed division truncates toward zero. The associated
remainder has the dividend's sign when non-zero, has magnitude smaller than the
divisor's magnitude, and satisfies `a = (a / b) * b + (a % b)` whenever the
operation succeeds. `Int::MIN / -1` and `Int::MIN % -1` are both overflow.
Semantic analysis preflights only reachable closed arithmetic trees composed entirely
of `Int` literals and arithmetic operators: a provable overflow is `N3031`, while a
provable zero divisor is `N3032`. Source lowered only for diagnostics on a statically
unreachable path is excluded from these execution-failure diagnostics. Semantic
reachability may also evaluate closed, side-effect-free Bool/Int literal expressions
for `if`, `while`, and `&&`/`||`, including checked arithmetic feeding comparisons.
This evaluator never propagates names, executes calls or blocks, inspects aggregates,
or folds the retained HIR. A `while` condition proven false therefore lowers its body
only for static diagnostics, while a proven true condition participates in the same
guaranteed-loop reasoning previously reserved for literal `true`. Independently of
constant evaluation, an `if`/`while` condition or `match` scrutinee already typed `!`
proves every successor branch/body/arm unreachable; those successors remain statically
checked but are lowered in diagnostic-only mode so execution-failure diagnostics and
flow mutations cannot escape a path that cannot run. Successful arithmetic trees are
deliberately not folded, and dynamic operands stop constant proofs rather than
triggering general constant propagation.
Dynamic arithmetic remains checked by the
interpreter: overflow is `N4002`, and division or remainder by zero is `N4003`.
Both layers consume the same dependency-free `nova-int-semantics` arithmetic
contract: semantic analysis owns only closed-HIR discovery and diagnostic policy,
while the interpreter owns only runtime evaluation and diagnostic mapping. The shared
leaf therefore prevents static/runtime drift without making HIR or diagnostics part of
the numeric core. This is implementation evidence for the numeric design, not yet a
stable language-wide promise about numeric widths, defaulting, conversions, or
overflow policy for future backends.

**Research.** The project must decide, with implementation evidence:

- the primitive numeric set and defaulting rules;
- whether value restriction or another rule is needed for inference;
- layout, representation, padding, and ABI guarantees for user-defined types;
- the pattern-matching model beyond qualified single-payload enum variants; and
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
Function parameters and `let` bindings are immutable. Record field projection
is read-only in the bootstrap subset; `record.field = value` is not an accepted
assignment form. Definite-initialization state is propagated through lexical
blocks and merged across `if` branches whose condition is not a direct Boolean
literal. If both branches can continue, a
binding is definitely initialized afterward only when both continuing paths
initialize it; a branch that cannot continue because it returns, breaks, or
continues does not constrain the surviving path.

Each match payload binding is immutable and scoped to one arm. A valid exhaustive
match with a dynamic scrutinee merges definite-initialization state by intersecting
every arm that can continue; non-continuing arms are excluded. If the scrutinee is
a direct, successfully resolved enum constructor, the bootstrap analyzer knows its
variant and only that selected arm contributes reachable initialization,
non-continuation, and loop-exit facts. Non-selected arms remain subject to pattern,
name, type, exhaustiveness, and arm-compatibility diagnostics, but their flow
mutations are discarded. Invalid or non-exhaustive matches establish no arm-derived
initialization facts during diagnostic recovery. This is direct-constructor
reachability, not propagation of enum values through locals, calls, or general
constant evaluation.

A continuing call rejected because its callee is not callable, its arity is wrong,
an argument has the wrong type, or an evaluated argument is erroneous is fail-closed
for flow recovery. Callee/argument diagnostics and HIR children are retained, but
assignments and loop-exit facts produced only inside that rejected call do not become
post-call facts. An actually evaluated non-continuing callee or argument keeps `!`
precedence.

A continuing field access rejected because its base is not a record or the named field
does not exist is likewise fail-closed for flow recovery. The base is still lowered
for deterministic diagnostics, but its assignments and loop-exit facts do not become
post-access facts. A base expression that is already `!` remains non-continuing and
does not acquire a secondary record-type diagnostic.

A continuing unary or binary operator rejected by operand typing is fail-closed for
flow recovery as well. Concrete mismatches yield `<error>` rather than retaining the
operator's nominal result type, and assignments or loop-exit facts created only while
lowering that rejected operator are discarded. A definitely evaluated operand that
is already `!` retains non-continuation precedence; `&&` and `||` continue to model
conditional right-hand evaluation rather than treating every lowered RHS as reachable.

A continuing `if` or `while` whose condition is not a valid `Bool` is also fail-closed
for flow recovery. The condition and nested branches/body are still lowered for
source diagnostics and lexical loop-control checking, but assignments and loop-exit
facts created only inside the rejected control construct do not become post-construct
facts. Invalid `if` conditions make the expression `<error>`-typed. A condition that
is already `!` retains non-continuation precedence rather than being flattened to a
continuing recovery error.

Continuing record or enum construction that is itself rejected by type-head,
structural, or payload/field type validation is fail-closed for flow recovery:
assignments and
loop-exit facts produced only while lowering that invalid aggregate cannot establish
state after the rejected expression. If a child expression is already `!`, its
reachable non-continuation remains dominant; this rollback rule applies only when
the invalid aggregate would otherwise continue.

The bootstrap `while` form is a pre-test statement. For an ordinary condition,
the body may execute zero times. Initialization facts established while
evaluating the mandatory first condition test may therefore flow after the
loop, while facts established only inside the body cannot by themselves prove a
binding initialized afterward. This conservative rule preserves the
zero-iteration exit.

A direct Boolean literal `while true` is a provisional special case because it
has no condition-false exit. The bootstrap analyzer records reachable `break`
exit states that target that exact loop. If at least one such exit exists, a
pre-existing binding is definitely initialized after the loop only when every
reachable break exit initializes it. If no reachable break exists, the loop is
non-continuing. A break consumed by a nested loop never becomes evidence for an
outer loop exit. This recognition is deliberately syntactic: equivalent-looking
computed or block-valued conditions do not trigger constant folding, fixed-point
iteration, or a general termination proof.

`break;` and `continue;` are provisional statement-only transfers with no value.
They are legal only in the body of an enclosing `while`; the condition
expression is outside that loop-control scope. `break;` targets the nearest
such loop and exits it. `continue;` targets the nearest such loop and re-enters
at its condition test. Both make the current path non-continuing for `if` and
exhaustive-`match` dataflow joins. Source after a transfer remains subject to
name/type diagnostics, but unreachable assignments must not alter the reachable
definite-initialization state. The same rule applies within strict left-to-right
expression evaluation: operands, call arguments, and record initializers after
an earlier non-continuing subexpression are lowered for diagnostics only and
cannot manufacture reachable scope or loop-exit facts.

Short-circuit Boolean operators are the deliberate non-strict exception.
`false && rhs` and `true || rhs` do not execute their RHS; the checker still
lowers that RHS for deterministic static diagnostics but discards its mutation,
definite-initialization, and loop-exit facts. `true && rhs` and `false || rhs`
execute the RHS normally. With a non-literal Boolean LHS, both the short-circuit
continuation and the RHS continuation remain possible, so definite-initialized
state after the expression is their intersection. An optionally executed RHS
that returns or otherwise cannot continue therefore does not make the whole
Boolean expression non-continuing, while a reachable RHS `break` remains a
possible exit from its enclosing loop.

A direct Boolean-literal `if` is another provisional reachability refinement.
For `if true`, only the then branch contributes reachable flow facts; for
`if false`, only the else branch does. The unselected branch remains fully
subject to name and type diagnostics, including branch type compatibility, but
its assignments, returns, and loop transfers cannot affect reachable continuation
state. Computed and block-valued Boolean conditions keep the ordinary two-branch
merge; this rule is not general constant folding.

Chained assignment, arbitrary lvalues, field mutation, indexing, and general
uninitialized storage remain unsupported.

**Research.** Broader flow-sensitive facts, labelled loops, value-carrying
breaks or loop expressions, nested and refutable binding forms, partial
aggregate initialization, mutable aggregate views, ownership interactions,
loop fixed-point analysis, path-sensitive Boolean reasoning beyond direct
Boolean literals, and diagnostics for more complex control-flow graphs
require implementation evidence before their rules are frozen.

## 6. Names, modules, and packages

**Decided.** Name resolution will be lexical, deterministic, independent of
filesystem enumeration order, and separate from type inference. Imports must
make dependency edges inspectable. Packages and modules must have stable
identity rules that work in reproducible builds.

**Provisional bootstrap decisions.** Top-level record and enum type identities
and function signatures are collected before function bodies are lowered. This
supports deterministic forward and recursive references to declared aggregate
types plus forward function calls without consulting filesystem or declaration
traversal order for semantic meaning. Records and enums share one type
namespace; built-in `Int` and `Bool` type names cannot be redefined.

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
roots, real-time constraints, aggregate representation, and the cost model.
The interpreter's current record-slot storage and boxed enum payloads are not
evidence of a final allocation or ownership strategy. Until a checked model
exists, Nova must not claim memory safety or zero-cost ownership.

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
calls, callbacks, unwinding, bindgen policy, and record layout interoperability
require dedicated design work.

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
The interpreter consumes typed HIR directly and supports the implemented
function, call, record construction/projection, enum construction/matching,
block, `if`, `while`, `break`, `continue`, return, binding, assignment, Unit,
Boolean, and integer subset. Unit helpers may return explicit `()` or fall through
a value-less body. Evaluation order is left-to-right; named record initializers
do not reorder their expressions when resolved to declaration slots. A match
evaluates its scrutinee once and only its selected arm. `&&` and `||`
short-circuit, and semantic dataflow models that same conditional RHS execution
rather than granting facts from code the interpreter may skip. The entry point
is a zero-argument top-level `main` returning `Int` or `Bool`.

The interpreter propagates `return`, `break`, and `continue` as structured
control flow through nested expressions and selected match arms. A `while`
consumes only the `break` or `continue` targeted lexically at its body; function
calls consume returns but may not become an implicit target for loop control.
Malformed HIR that lets loop control escape its lexical loop or function fails
closed with runtime invariant diagnostic `N4005`.

Runtime record values carry nominal identity and declaration-order field slots;
runtime enum values carry nominal identity, a declaration-order variant slot,
and an optional boxed payload. Those representations are executable semantic
oracles, not stable layouts, allocation promises, serialization formats, or
backend ABIs. Runtime failures use structured diagnostics. Recursive execution
is guarded by a finite call-depth limit, and all statement/expression evaluation
shares a finite step budget so nonterminating loops fail closed rather than
intentionally hanging the host. These choices provide an executable oracle for
the current subset; HIR interpretation is not the intended final backend ABI.

**Research.** HIR and MIR forms, verification rules, optimization contracts,
debug information, incremental compilation, monomorphization, backend
selection, stable entry-point conventions, labelled/value-producing loop-control
semantics, aggregate lowering/layout, and cross-backend execution conformance
remain open.

## 12. Diagnostics and tooling contracts

**Decided.** Diagnostics are structured compiler data first and rendered text
second. Stable diagnostic codes, exact source spans, primary and secondary
labels, notes, and machine-readable output are required directions. Recovery
diagnostics must be deterministic for identical input and compiler version.

**Provisional bootstrap decisions.** The current toolchain uses half-open UTF-8
byte spans and exposes human and JSON Lines rendering across lexical, syntactic,
semantic, and runtime diagnostics. Aggregate diagnostics distinguish duplicate,
unknown, missing, mistyped, payload-arity, nominal-mismatch, and non-exhaustive
cases while preserving source-qualified labels. `N3013` identifies a bootstrap
`break` or `continue` with no enclosing `while` body. Diagnostic code meaning is
documented by tests but codes are not yet covered by the language compatibility
promise.

The bootstrap now exposes semantic-inspection schema v1 for successfully checked
single-file programs. It projects resolved declarations, bindings, types,
nominal identities, typed blocks/statements/expressions, spans, and exhaustive
match facts into a tooling-owned JSON model. Document-local IDs and deterministic
ordering are specified independently of Rust HIR layout. Rejected source or an
inspection invariant failure produces diagnostics and no partial document.
Compiler debug text is not this protocol.

Effects, ownership facts, module graphs, transformations, and incremental keys
cannot appear until the corresponding compiler semantics exist. The v1 schema
is provisional before Nova 1.0 and is versioned independently from the language,
diagnostics, packages, and future IRs.

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
6. Nesting, call-depth, and execution-step limits fail with diagnostics before
   uncontrolled recursion or nonterminating bootstrap execution can consume the
   host indefinitely.
7. Iteration and expression-evaluation order that affects output is explicit and
   deterministic; named record fields must not reorder initializer evaluation.
8. An implemented grammar, semantic, or execution rule has positive and
   negative tests.
9. A local read cannot observe an uninitialized binding; delayed initialization
   must be proven on every reachable continuing path before the read.
10. Pre-test loop analysis must not treat body-only effects as post-loop facts
    when a loop may execute zero times. A direct literal `while true` may derive
    post-loop facts only from reachable `break` exits targeting that exact loop.
11. `break` and `continue` target only the nearest enclosing `while` body; a
    loop's condition is not inside that control-transfer scope.
12. Unreachable statements, strict-expression suffixes, statically skipped
    short-circuit operands, and unselected direct-literal `if` branches may still
    produce diagnostics but must not change definite-initialization or loop-exit
    facts observed by reachable continuation paths. Dynamic short-circuit operands
    and non-literal `if` branches contribute only facts valid on their possible
    continuing paths.
13. Nominal type identity must not silently collapse to structural field shape.
14. Resolved field slots must preserve source semantics and must not be mistaken
    for a stabilized memory-layout or ABI guarantee.
15. An accepted enum match names every variant of exactly one nominal enum once;
    its scrutinee runs once and unselected arms do not run.
16. Resolved enum variant slots and boxed interpreter payloads are not stabilized
    layout, allocation, ownership, serialization, or ABI guarantees.
17. Optimization must preserve specified behavior and later operate on verified
    IR rather than repair invalid earlier output.
18. A semantic-inspection document is emitted only for accepted, internally
    consistent HIR and must conform to an explicitly versioned tooling schema.
19. Roadmap documents distinguish implemented, provisional, and researched
    properties; benchmarks and safety claims require reproducible evidence.

## 15. Current unresolved research register

The highest-impact unresolved questions are:

- inference boundaries and public type annotation policy;
- primitive numeric semantics across all execution backends;
- richer algebraic data types, pattern usefulness, and aggregate layout
  guarantees;
- typed error and effect representation;
- the hybrid ownership/region/managed-memory model;
- data-race freedom and cancellation in structured concurrency;
- HIR/MIR contracts shared across execution modes and targets;
- stable ABI and C ownership conventions;
- deterministic, incremental, reproducible package builds; and
- evolution of semantic-introspection across modules, effects, ownership,
  transformations, and incremental compilation.

These questions intentionally have no stable surface commitment in the current
bootstrap subset.
