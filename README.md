# Nova

Nova is an early-stage programming-language project exploring whether one
coherent language can combine fast, readable application development with
predictable native performance and strong safety properties. Its intended
design space includes static typing with inference, explicit optional values,
algebraic data types, typed errors and effects, structured concurrency, and
low-level control through narrowly classified `unsafe` capabilities.

Those are design goals, not current claims. Nova is not production ready. The
current bootstrap can interpret its small checked subset, but it does not yet
implement ownership, effects, concurrency, native code generation, a standard
library, or memory-safety analysis.

## Current status

The repository contains the Phase 0 constitution, the executable Phase 1
frontend, Phase 2 semantic-core slices, and executable Phase 3 bootstrap
interpreter slices. The toolchain is written in Rust and can:

- read a Nova file while rejecting malformed UTF-8;
- lex the documented v0.1 subset with byte-exact source spans;
- parse functions, nominal records and enums, explicit aggregate construction,
  exhaustive enum matching, field projection, initialized bindings, typed
  delayed `var` initialization, narrow assignments, expressions, blocks, calls,
  `if` expressions, pre-test `while` loops, and statement-only `break`/`continue`;
- lower accepted syntax into a resolved, typed HIR with stable function,
  binding, record, and enum identities, plus a verified function-level CFG;
- resolve top-level functions and nominal types, parameters, lexical local
  bindings, record-field and enum-variant name/slot identities, and match payload bindings;
- check bootstrap `Int`, `Bool`, `Unit`, and nominal aggregate types, function
  signatures, local inference and annotations, calls, operators, block tails, branches,
  returns, loop conditions, loop-control legality, record construction/projection,
  enum construction, match exhaustiveness and arm types, assignment
  mutability/type constraints, and CFG-based definite initialization;
- execute semantically accepted programs through a deterministic bootstrap
  interpreter with function calls, recursion, Unit-valued procedures, records, enums,
  pattern matching, mutation, blocks, conditionals, bounded loops, and structured
  `break`/`continue`;
- emit structured, coded compile-time and runtime diagnostics rendered as human
  text or JSON Lines, including non-fatal unreachable-code warnings;
- print a deterministic debug representation of the parsed AST; and
- emit fail-closed semantic-inspection v1 documents with resolved declarations,
  bindings, types, spans, expression relationships, and exhaustive match facts,
  plus explicitly selected v2 documents that add the verified CFG without
  exposing debug HIR or compiler-owned graph layouts as a protocol.

`nova check` performs lexical, syntactic, name-resolution, bootstrap type, and
definite-assignment validation. `nova run` performs those same checks and then
executes zero-argument `main`. The interpreter is evidence for the executable
subset, not a claim that Nova's final runtime representation, numeric model,
aggregate layout, ABI, or backend is stable.

Semantic warnings do not reject an otherwise valid program. The bootstrap
currently reports `N3033` when the verified CFG proves that source follows an
executable `return`, `break`, or `continue`; the warning is written to standard
error while `check`, `run`, or `inspect` continues normally. Existing errors
suppress this warning pass to avoid recovery cascades.

The implemented syntax is intentionally small:

```nova
enum Result {
    Empty,
    Value(Int),
}

fn main() -> Int {
    let result = Result::Value(42);
    match result {
        Result::Empty => 0,
        Result::Value(value) => value,
    }
}
```

See [the implemented grammar](docs/grammar.md) for the normative frontend
subset, [the enum and pattern semantics](docs/enums-and-patterns.md) for that
aggregate slice's semantic contract,
[the diagnostics contract](docs/diagnostics.md) for error/warning and exit-status
behavior,
[the semantic-introspection v1 contract](docs/semantic-introspection.md) and
[v2 CFG extension](docs/semantic-introspection-v2.md) for the machine-readable
tooling boundary,
[the bootstrap control-flow contract](docs/control-flow.md) for CFG verification
and definite-initialization dataflow, and
[the language constitution](docs/language-constitution.md) for decisions that
extend beyond them.

## Current semantic rules

The Phase 2 bootstrap checker predeclares function signatures and nominal record
and enum identities, so forward calls, recursion, forward aggregate type
references, and recursive enum payload types resolve deterministically. A local
initializer is checked before its new binding enters scope, preventing
accidental self-reference. Duplicate names in the same lexical scope are
rejected; nested lexical blocks may shadow outer bindings in this slice.
Function parameters and a function body's outermost bindings share one scope.

Rejected calls are fail-closed for continuing flow recovery. Callees and arguments
are still lowered left-to-right for deterministic diagnostics, but a non-callable
callee, wrong arity, argument type mismatch, or erroneous argument yields Error HIR
and cannot export assignments or loop-exit facts. An actually evaluated child that
is already non-continuing keeps its `!` flow.

Rejected field access follows the same recovery discipline. The base is evaluated
first, but a continuing non-record base or unknown field yields Error HIR and cannot
export flow facts produced only inside the rejected access. A base that is already
non-continuing keeps `!` without a secondary field-type cascade.

Rejected unary and binary operators are also fail-closed. Concrete operand type
mismatches produce Error HIR rather than a nominal success type, and flow facts from
a continuing rejected operator are rolled back. Non-continuation from an operand that
must be evaluated keeps `!` precedence; short-circuit operators retain their existing
conditional right-hand evaluation rules.

Matching `Int`, `Bool`, and `Unit` values support `==` and `!=`. `Unit` has a
single runtime value, so Unit equality is always true and Unit inequality is always false
once both operands have evaluated normally. A nominal enum also supports equality when
every declared variant is payload-free; operands must have the same enum identity and
comparison uses the resolved variant slot. Function values are comparable only at the
same fully resolved signature and compare top-level declaration identity. Direct top-level
function-reference HIR retains the source-resolved spelling alongside `FunctionId`; runtime
and semantic-inspection consumers recheck that name/id/signature contract, so malformed HIR
cannot silently retarget a reference to a same-signature sibling declaration. Validated local
aliases still carry only runtime declaration identity rather than source spelling. Enums with
any payload variant and records remain non-comparable. Closed-condition analysis can prove
literal Unit, direct payload-free enum-constructor, and direct function-reference
comparisons, while locals and calls remain dynamic and are still evaluated at runtime.

Invalid continuing control conditions are fail-closed too. A non-Bool or erroneous
`if` condition makes the expression Error-typed and discards condition/branch flow
facts; a rejected `while` condition likewise cannot export pre-test initialization or
loop exits. A condition that is already `!` keeps its non-continuation, while valid
Bool conditions retain the established pre-test and branch dataflow rules.

`record Name { field: Type, ... }` declares a nominal type: two separately
declared records are distinct even if their fields have the same shape. Field
names must be unique. `new Name { field: expression, ... }` must initialize every
declared field exactly once with a value of the declared type. Named
initializers may be written in any order, but their expressions evaluate left
to right in written source order. HIR retains each resolved field spelling alongside
the nominal record identity and declaration-order slot without reordering evaluation.
The interpreter and semantic-inspection boundary independently recheck that name/slot
pair, so malformed HIR cannot silently retarget one field to a same-typed sibling while
the stable inspection schema continues to expose its existing declaration field ID.
`value.field` is read-only field projection in this slice. Record equality,
field assignment, layout, and ABI guarantees are not implemented.

Rejected aggregate construction is fail-closed for continuing flow recovery. A
record with an unresolved/non-record target or invalid field shape/type, or an enum
constructor with invalid target/payload shape/type, may still emit deterministic
child diagnostics, but assignments or loop
exits created only inside that rejected continuing expression do not become
post-expression facts. A child that is already non-continuing keeps its `!` flow.

`enum Name { Empty, Value(Type) }` declares a nominal sum type whose variants
carry zero or one payload in this slice. Construction is explicitly qualified as
`Name::Empty` or `Name::Value(expression)`. A `match` scrutinee must have an enum
type, every pattern must name a variant of that same nominal enum, and every
variant must occur exactly once. Payload bindings are immutable and scoped to
one arm. HIR retains the source-resolved variant spelling alongside the nominal enum
identity and declaration-order slot for both constructors and match arms. Runtime and
semantic-inspection consumers recheck that name/slot pair, so malformed HIR cannot
silently retarget a constructor or pattern to a same-shaped sibling variant. Wildcards,
guards, nested patterns, multi-payload variants, equality for payload-bearing enums,
layout, and ABI guarantees are not implemented.

`let` bindings and function parameters are immutable. `var` bindings may be
assigned with the narrow statement form `name = expression;`. The target must
resolve to a lexical `var`; functions, unknown names, `let` bindings, and
parameters are rejected as assignment targets. The replacement value must keep
the binding's established type. Resolved local/parameter reads and assignment targets
retain the selected declaration's `BindingId`, source spelling, and declaration-name
span in HIR. The declaration span is part of the integrity pair because nested scopes
may legally shadow a binding with the same spelling and type; name/type alone cannot
distinguish those declarations. This metadata does not alter lexical resolution, CFG
binding IDs, or the semantic-inspection schema. Assignment is not an expression and
therefore cannot be chained or embedded in another expression.

A mutable local may also be declared as `var name: Type;` and initialized by a
later assignment. The explicit type is required. Reading such a binding before
it is definitely initialized is diagnostic `N3009`. For `if` expressions with a non-literal condition,
analysis evaluates the branch states independently and keeps a binding
initialized afterward only when every branch that can continue has initialized
it. For a valid exhaustive match with a dynamic scrutinee, the same intersection
rule applies across every arm that can continue. When the scrutinee is a direct,
successfully resolved enum constructor, its variant is already known: only the
selected arm may contribute definite-initialization, non-continuation, or loop-exit
facts. Non-selected arms are still fully checked for pattern validity, static
diagnostics, exhaustiveness, and arm type compatibility. A branch or reachable arm
that returns, breaks, or continues does not constrain a surviving continuation.
Unreachable code is still analyzed for deterministic diagnostics, but its
assignments cannot manufacture reachable definite-initialization facts.
For strict left-to-right expression forms, once an earlier subexpression cannot
continue, later operands, call arguments, or record initializers are likewise
lowered only for diagnostics and cannot create reachable scope or loop-exit
facts.

`&&` and `||` preserve static checking of both operands while modeling their
runtime reachability explicitly. A direct `false && rhs` or `true || rhs` lowers
`rhs` only for diagnostics, so skipped assignments and loop transfers contribute
no reachable flow facts. A direct `true && rhs` or `false || rhs` treats `rhs` as
mandatory. With a dynamic Boolean left operand, `rhs` is optional: post-expression
definite initialization is the intersection of the short-circuit continuation
and every continuing RHS path. An RHS that returns, breaks, or continues therefore
does not make the whole Boolean expression non-continuing when the left operand
can bypass it, although a reachable RHS `break` still remains a possible exit
from its enclosing loop.

A direct `if true` or `if false` refines control-flow reachability without
turning the checker into a general constant folder. Only the selected branch may
contribute definite-initialization, non-continuation, or loop-exit facts; the
unselected branch is still lowered for deterministic static diagnostics and still
participates in branch type compatibility. Block-valued or computed Boolean
conditions keep the ordinary conservative two-branch merge.

`while condition { body }` is a pre-test statement. The condition must be
`Bool`. For an ordinary condition, the body may execute zero times, so
definite-assignment facts established while evaluating the mandatory first
condition test may survive the loop while facts established only inside the body
do not. This preserves the zero-iteration exit rather than manufacturing
initialization evidence.

When the closed-condition evaluator proves a `while` condition true (for example
`true` or the statement-free wrapper `{ true }`), the loop has no condition-false
exit. The checker treats it as guaranteed-entry and records only reachable `break`
transfers that target that exact loop. If there are such exits, a pre-existing
binding is definitely initialized afterward only when it is initialized at every
reachable break exit. If there is no reachable break, the loop is non-continuing.
A `break` consumed by a nested loop does not count as an exit from an outer loop.
This proof changes flow analysis only; the retained HIR is never constant-folded.

`break;` and `continue;` are legal only inside an enclosing `while` body. The
condition expression is intentionally outside that loop-control scope. `break;`
exits the nearest enclosing loop; `continue;` skips the rest of the current
iteration and re-evaluates that same loop's condition. Neither carries a value
or acts as an expression. Labelled loops and value-carrying breaks are not part
of the bootstrap subset.

`Int`, `Bool`, `Unit`, and declared nominal records and enums are recognized
surface types today. `()` is the sole Unit literal, and a block with no tail also
produces Unit. A function declared `-> Unit` may fall through such a body or use
the explicit `return ();` form; non-Unit functions still need a compatible tail or
an explicit return on every continuing path. Arithmetic and ordered comparisons
require `Int`; boolean operators require `Bool`; equality accepts matching `Int`, `Bool`, `Unit`, the same function signature, or
the same nominal payload-free enum type; function equality compares declaration
identity rather than addresses or code layout, and calls require matching arity and
argument types.
`if` conditions require `Bool`, and continuing branches or match arms must remain
type-compatible. The internal `!` bottom type still has no surface spelling.

These rules are bootstrap semantics, not a promise that Nova's broader type,
mutation, control-flow, aggregate, and shadowing policies are frozen.

## Bootstrap execution rules

`nova run` requires one top-level `main` with no parameters and an `Int`, `Bool`,
or `Unit` return type. A Unit-valued `main` prints `()` like any other returned
bootstrap value. Execution evaluates expressions left to
right. Record initializer
expressions follow the same rule even when named fields are written out of
declaration order. `&&` and `||` are short-circuiting, so a skipped right operand
performs no mutation, call, return, or loop transfer. Semantic dataflow follows
that same reachability while still type-checking the skipped source. A match
evaluates its scrutinee exactly once and then only its selected arm.

The interpreter propagates structured control flow through nested blocks,
conditionals, aggregate initializers, call arguments, and selected match arms.
`return` reaches the current function call. `break` and `continue` travel only
to the nearest enclosing `while`; that loop consumes them by exiting or starting
the next condition test. If malformed HIR lets loop control escape its lexical
loop or cross a function boundary, execution fails closed with invariant
diagnostic `N4005` rather than guessing a target.

Function calls also validate the runtime/HIR type boundary. Every argument must
recursively conform to its resolved parameter type, and every returned runtime
value must recursively conform to the function's declared type. A direct top-level
function reference additionally revalidates its retained source spelling against the
referenced declaration's `FunctionId` before producing `Value::Function`; the ordinary
expression postcondition then independently checks the declaration signature against
the reference HIR type. Local aliases receive only that already-validated runtime
identity. Nominal record and enum identities, record slots, and enum payloads are
checked rather than trusted from their outer value tag alone. Valid semantically
produced HIR is unaffected; malformed or contract-drifted HIR fails closed with `N4005`.

Aggregate construction enforces the same invariant locally: each evaluated record
field must conform to its declaration slot type, and each enum payload must conform
to its selected variant payload type before the aggregate value is created. Record
construction and projection also revalidate the HIR-resolved field spelling against
its declaration-order slot, closing same-typed member-retargeting drift that a type
postcondition alone cannot observe. Enum construction and matching apply the analogous
variant spelling/slot check after payload or scrutinee evaluation has produced an ordinary
value, preserving structured return/break/continue propagation before value-only invariant
validation. These checks catch malformed HIR even when the aggregate never crosses a
function boundary.

Runtime frames preserve the resolved binding contract too. Each slot records its
resolved type, mutability, initialization state, declaration spelling, and declaration
span. Parameters, local bindings, delayed `var` declarations, and match payload bindings
reject non-conforming initial values or incompatible reuse of one binding identity;
repeated execution of the same lexical binding may refresh its slot only with identical
type/mutability/name/span metadata. Binding reads revalidate the retained HIR reference
against that slot before checking expression type and stored runtime-value conformance,
so even same-name, same-type shadow retargeting fails closed. Assignment evaluates its
RHS first; only an ordinary produced value triggers target identity, mutability, and
replacement-type validation, preserving structured `return`/`break`/`continue` precedence.
Any such interpreter/HIR drift fails closed with `N4005`.

Every expression that completes with an ordinary runtime value also has a final
interpreter postcondition: the value must recursively conform to that expression's
typed-HIR result type. Runtime conformance first validates the resolved type itself:
nominal record/enum names must still match their declaration IDs, and function
signatures recursively apply the same rule to parameter and return types. This closes
a malformed-HIR gap where a record or enum value with the correct nominal ID could
previously satisfy a drifted `Type::Record`/`Type::Enum` spelling. The same entry gate
therefore protects local or discarded literals, projections, operators, blocks,
conditionals, matches, call boundaries, frame storage, and aggregate nesting without
changing the compact runtime value representation. Equality adds an operator-level
precondition on ordinary value-producing paths as well: when both operands can complete
normally, their resolved types must satisfy the same shared semantic comparability rule,
including the declaration-wide payload-free requirement for enums. Malformed HIR therefore
cannot compare a payload-free variant of an enum whose other variants carry payloads.
A `Never` operand still evaluates normally for structured `return`, `break`, or `continue`
propagation and never reaches the comparison itself. Any interpreter/HIR contract drift
on a value-producing equality path fails closed with `N4005`.

For deterministic execution while the numeric design remains provisional, the
bootstrap frontend now covers the complete signed 64-bit literal endpoints: positive
literals end at `9223372036854775807`, while `-9223372036854775808` is normalized
during semantic lowering to the exact minimum `Int`. Positive `9223372036854775808`
is `N3030`; any larger decimal magnitude is lexical `N1004`. The interpreter
represents `Int` as signed 64-bit at runtime and uses checked arithmetic.
Signed division truncates the quotient toward zero; a non-zero remainder has the
same sign as the dividend and satisfies `a = (a / b) * b + (a % b)`. Both
`i64::MIN / -1` and `i64::MIN % -1` are classified as integer overflow. Before
execution, semantic analysis also preflights reachable closed arithmetic trees made
entirely from `Int` literals and arithmetic operators: statically certain overflow is
`N3031` and a statically certain zero divisor is `N3032`. Source lowered only for
diagnostics because control flow proves it unreachable does not manufacture these
execution-failure diagnostics. The same side-effect-free closed evaluator may
determine `if`, `while`, and short-circuit reachability from Bool/Int/Unit values,
direct payload-free enum constructors, direct top-level function references,
statement-free block wrappers, comparisons, and Boolean operations; this changes
flow analysis only and never folds the retained HIR. Local bindings, calls,
statement-bearing blocks, other aggregates, and other dynamic operands stop the proof.
More generally, when an `if`/`while` condition or `match` scrutinee is already
non-continuing (`!`), its successor branches/body/arms are lowered only for static
diagnostics: execution-only constant failures and flow mutations cannot come from a
path runtime control never reaches. Successful constant arithmetic is not folded, and
any expression with a dynamic operand remains runtime checked. Such
dynamic overflow produces `N4002`; dynamic division or remainder by zero produces
`N4003`. The arithmetic truth table itself lives once in the dependency-free
`nova-int-semantics` leaf crate; semantic preflight supplies only closed-HIR traversal
and the interpreter supplies only runtime diagnostic mapping. This keeps both layers
on one checked signed-64 contract rather than duplicating host-edge-case policy.
Recursive execution is guarded by a finite active-call budget
and reports `N4004`. All statement/expression evaluation also shares a finite
execution-step budget; a nonterminating loop therefore reports `N4006` instead
of hanging indefinitely. Missing or invalid `main` is `N4001`. Record values
currently use declaration-order slots; enum values use a variant slot and an
optional boxed payload. Those interpreter-owned nominal representations are not
stabilized source layouts, allocation promises, ownership rules, or ABI
contracts.

## Build and use

Nova declares Rust 1.85 as its bootstrap minimum and also tracks current stable
Rust in CI. With Rust and Cargo installed:

```console
cargo build --workspace
cargo run -p nova-cli -- check examples/basics.nv
cargo run -p nova-cli -- run examples/basics.nv
cargo run -p nova-cli -- ast examples/basics.nv
cargo run -p nova-cli -- inspect examples/enums.nv --format json
cargo run -p nova-cli -- inspect examples/enums.nv --format json --schema-version 2
```

The `run` command prints the returned value from `main`.

Machine-readable diagnostics are available without changing the compiler's
internal diagnostic model:

```console
cargo run -p nova-cli -- run examples/broken.nv --message-format json
```

The installed binary is named `nova`:

```text
nova check <file> [--message-format human|json]
nova run <file> [--message-format human|json]
nova ast <file> [--message-format human|json]
nova inspect <file> --format json [--schema-version 1|2] [--message-format human|json]
```

Exit status `0` means the requested operation succeeded, `1` means the source or
execution was rejected, and `2` means the command line was invalid. `nova ast`
intentionally stops after parsing, so it can inspect a syntactically valid AST
even when `nova check` or `nova run` would reject that program later.
`nova inspect` instead requires the complete semantic pipeline to succeed and
writes no partial document when source diagnostics or an inspection invariant
failure occurs. Non-fatal warnings are written to standard error without changing
status `0`, runtime output, or a successful inspection document. Schema v1 remains
the default; v2 must be requested explicitly.

## Bootstrap architecture

```text
source bytes
  -> nova-source        source identity, UTF-8 text, spans, locations
  -> nova-lexer         tokens and lexical diagnostics
  -> nova-parser        AST and syntactic diagnostics
  -> nova-sema          typed HIR, verified CFG, resolution, typing, dataflow
      -> nova-inspect       versioned facts and fail-closed JSON projection
      -> nova-interpreter   deterministic checked, bounded HIR execution

nova-int-semantics      dependency-free checked signed-64 arithmetic truth table
nova-cli                check/run/ast/inspect orchestration and presentation

nova-diagnostics        shared structured diagnostic model and renderers
```

Crate boundaries follow semantic responsibilities rather than intended future
compiler passes. Later work can deepen HIR, inference, effects, MIR, layout, and
backends without making the AST, interpreter, or CLI the owner of unfinished
language semantics.

## Engineering policy

- Unsupported constructs are errors; the compiler does not approximate them.
- Every implemented semantic, syntactic, or execution rule requires
  deterministic tests.
- Source positions are UTF-8 byte ranges internally and one-based line/column
  locations when rendered.
- Runtime arithmetic is checked; host build mode never decides Nova results.
- Observable evaluation order is explicit; named record fields do not reorder
  their initializer expressions, and a match evaluates only its selected arm.
- Short-circuit reachability in semantic flow must agree with runtime `&&`/`||`
  execution while skipped source remains statically checked.
- Non-continuing control-flow paths cannot contribute definite-assignment or loop-
  exit facts to code they cannot reach.
- Non-fatal unreachable warnings are derived from verified CFG edges, not from a
  parallel lexical reachability flag, and never turn accepted HIR into a rejection.
- Function CFGs are verified before publication; `N3009` is produced by their
  fixed-point must analysis rather than ad-hoc diagnostic emission during name lookup.
- Definite initialization has no parallel lexical Boolean: binding HIR preserves the
  declared type while CFG read/initialize events exclusively own flow validity.
- Machine-readable semantics cross a separately versioned schema boundary;
  debug AST/HIR output is never silently promoted into a tooling contract.
- Potentially nonterminating bootstrap execution is bounded and fails with a
  structured diagnostic rather than intentionally hanging the host.
- CI checks Rust 1.85 compatibility, rejects formatting and Clippy warnings on
  current stable, and runs all tests, builds, and rustdoc.
- Roadmap status is evidence-based; planned properties are not reported as
  implemented guarantees.

The staged implementation plan is in [docs/roadmap.md](docs/roadmap.md).

## Contributing

Keep changes focused and pair grammar, semantic, or runtime changes with
specification updates, positive tests, and negative tests. Prefer a small
end-to-end slice over disconnected scaffolding for several future phases.

No project license has been selected yet. Until a license file is added, the
repository remains under the rights granted by applicable copyright law and
GitHub's terms; do not infer an open-source license from public visibility.
