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

`--message-format human|json` changes diagnostic presentation only. It does not
change acceptance, exit status, runtime values, or semantic-inspection JSON.

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

Nova has no warning configuration, lint groups, source attributes, command-line
allow/deny switches, warnings-as-errors mode, cap-lints policy, or cross-package
diagnostic aggregation yet. `N3033` and `N3034` are narrow implemented proofs, not a
claim that CFG reachability or pattern usefulness is a general-purpose linter or that the
current warning set is complete.
