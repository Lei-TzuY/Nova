# Bootstrap Diagnostics Contract

This document specifies Nova's implemented diagnostic severity and command-line
behavior. It is a bootstrap tooling contract, not a promise that every current
diagnostic code or warning policy is stable through Nova 1.0.

## Structured model

Every diagnostic carries:

- severity `error` or `warning`;
- a compiler-version-scoped code;
- a summary;
- ordered primary and secondary labels with source-qualified UTF-8 byte spans;
  and
- zero or more notes.

Human rendering and JSON Lines are presentations of the same data. JSON output
uses the lowercase severity spelling and emits one complete object per line.
Diagnostics are ordered by primary source span for deterministic output.

An `error` rejects the requested compile operation. A `warning` describes an
accepted program and does not by itself prevent HIR inspection or execution.
`AnalysisOutput::has_errors()` and `AnalysisOutput::is_success()` therefore test
severity rather than whether the diagnostic vector is empty.

## CLI behavior

Semantic diagnostics are written to standard error before command-specific
output. Their effect is:

| Result | Exit status | `check` | `run` | `inspect` |
| --- | ---: | --- | --- | --- |
| no diagnostics | `0` | no output | value on stdout | document on stdout |
| warnings only | `0` | warnings on stderr | warnings on stderr, value on stdout | warnings on stderr, document on stdout |
| any error | `1` | diagnostics on stderr | no execution | no document |
| invalid command line | `2` | usage error | usage error | usage error |

Every source command accepts exactly one filesystem path or `-`. The latter consumes
standard input to EOF and uses `<stdin>` as the source name in human and JSON diagnostics.
A standard-input read failure is `N0002`; malformed bytes remain `N0001`, with the first
invalid byte offset reported against `<stdin>`. Input transport does not otherwise change
diagnostic ordering, severity, command output, or exit policy.

`--message-format human|json` changes diagnostic presentation only. It does not
change acceptance, exit status, runtime values, or semantic-inspection JSON.

`check`, `run`, and `inspect` also accept `--fail-on-warnings` for strict CI
policy. If otherwise-successful semantic analysis emits a warning, the option
changes the exit status to `1`; `run` does not execute the program and `inspect`
does not emit a document. The diagnostics retain severity `warning` in both
renderers rather than being promoted to errors. Programs with no warnings and
programs already rejected by errors behave unchanged. `ast` rejects this option
because it deliberately stops before semantic analysis.

## `N3031` / `N3032` closed arithmetic failures

Semantic analysis reports an execution failure early when the side-effect-free
closed-HIR proof establishes that a reachable arithmetic expression must fail:

- `N3031` reports signed-64 `Int` overflow;
- `N3032` reports division or remainder by zero;
- the primary label covers the exact arithmetic expression whose checked
  operation fails, even when the proof reaches it through immutable block-local
  bindings, a selected `if`/`match`, a payload binding, or a record projection;
- re-proving that same source failure through enclosing closed composites does
  not emit duplicate diagnostics, while distinct failing source spans remain
  distinct diagnostics;
- one reachable closed block may therefore emit multiple `N3031`/`N3032`
  diagnostics when it contains multiple independent deterministic failures; and
- successful proofs validate semantics without folding or replacing the retained
  HIR.

This is an execution-certainty boundary, not general constant propagation. A
call, mutable value, or other genuinely dynamic arithmetic operand stops the proof,
leaving runtime arithmetic checks (`N4002` overflow and `N4003` zero divisor) responsible
for failures that cannot be known statically. A dynamic selector does not erase closed
facts already established outside it: if a valid continuing `if` condition or `match`
scrutinee is unknown, every potentially executable branch or arm is still inspected for
deterministic arithmetic failures that depend only on those outer closed facts. The
selector and any dynamic payload binding remain dynamic. Likewise, source lowered only for
diagnostics because control flow proves it cannot execute does not emit
`N3031`/`N3032`. Static name, type, pattern, and exhaustiveness diagnostics still
run on such diagnostic-only source under their ordinary policies.

A closed-true `while` has a reachable successor for this preflight only when its body has a
reachable `break` targeting that loop. A nested loop consumes its own breaks, an unselected
branch contributes no exit, and a rejected expression cannot reconnect its recovery-only
loop-exit facts.

## `N3033` unreachable code

The first implemented warning is deliberately narrow. After a function CFG has
passed verification and the complete semantic analysis contains no errors, the
warning pass computes nodes reachable from the entry without crossing a
`Diagnostic` edge. For each executable `return`, `break`, or `continue` transfer
that has diagnostic-only successors, it reports the earliest successor span:

- primary label: the first source region that cannot execute;
- secondary label: the controlling transfer and why it prevents continuation;
- at most one warning per transfer; and
- exact duplicate primary spans are emitted once.

Source after a transfer remains fully lowered and statically checked. If that
source produces an error, the program is rejected and the warning pass is
suppressed, preventing an unreachable warning from obscuring the real error.

`N3033` itself does not report constant-selected `if` or `match` branches,
short-circuit operands, a statically skipped `while` body, or code after a proven
nonterminating loop. Direct-constructor match usefulness has its own `N3034` policy
below rather than being inferred generically from diagnostic CFG edges.

## `N3034` statically unreachable match arm

When a successfully resolved `match` scrutinee is a direct enum constructor, semantic
analysis already knows its exact declaration-order variant before arm execution. For each
otherwise-valid concrete arm that names a different variant, Nova reports `N3034`:

- primary label: the arm pattern that can never be selected;
- secondary label: the direct constructor that proves the selected variant;
- one warning for each non-selected valid arm; and
- no warning when the scrutinee reaches the match through a local, parameter, call, or
  other dynamic expression.

A warned arm is still fully lowered and name/type checked. Its CFG path remains
diagnostic-only and cannot contribute definite-initialization, non-continuation, or loop
transfer facts. `N3034` therefore exposes an existing reachability proof; it does not
change HIR, CFG shape, runtime dispatch, or semantic-inspection schemas. Warning candidates
are deferred until semantic analysis is otherwise error-free, so an error inside a
non-selected arm suppresses `N3034` and remains the actionable diagnostic.

This is the first narrow usefulness diagnostic, not a general pattern-usefulness matrix.
There is still no catch-all arm, guard usefulness, nested-pattern coverage, or warning for
dynamic enum matches.

## Deliberate limits

Apart from the whole-program `--fail-on-warnings` exit policy, Nova has no warning
selection, lint groups, source attributes, command-line allow/deny switches,
severity-promoting warnings-as-errors mode, cap-lints policy, or cross-package diagnostic
aggregation yet. `N3033` and `N3034` are narrow implemented proofs, not a claim that CFG
reachability or pattern usefulness is a general-purpose linter or that the current warning
set is complete.
