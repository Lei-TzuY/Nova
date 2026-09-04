# Nova numeric semantics

Status: **normative for the implemented bootstrap `Int` and `UInt` behavior**

Nova currently has two numeric types: signed 64-bit `Int`, with the closed range
`-9223372036854775808` through `9223372036854775807`, and unsigned 64-bit `UInt`,
with the closed range `0` through `18446744073709551615`. This document promotes
their executable behavior from an interpreter accident to the language-level contract
for the current two-family subset; it does not reserve additional numeric families,
literal suffixes, implicit conversions, or backend ABIs.

## Literals and boundaries

Decimal, binary, octal, and hexadecimal unsuffixed integer literals decode to one
radix-independent magnitude and default to `Int`. Positive literals are limited to
`Int::MAX`. Magnitude `2^63` is accepted only
as the operand of prefix negation, producing `Int::MIN`; larger magnitudes are rejected.

Two payload-free associated constants expose the exact language boundaries:

- `Int::MIN` is `-9223372036854775808`;
- `Int::MAX` is `9223372036854775807`.

`Int` is a reserved primitive type, so these names cannot collide with a user-defined
nominal type. Other payload-free `Int::member` spellings and payload-bearing boundary
forms such as `Int::MAX(1)` are not numeric constants and remain rejected by ordinary
semantic resolution. The parser keeps qualified syntax generic; semantic analysis
canonicalizes only the implemented built-in spellings before ordinary name/type
resolution.

## Arithmetic

Unary `-` and binary `+`, `-`, `*`, `/`, and `%` operate on `Int`. Arithmetic is checked:
a mathematically unrepresentable result is an error rather than wrapping, saturating, or
silently widening. Division truncates toward zero. A non-zero remainder has the dividend's
sign and satisfies `a == (a / b) * b + (a % b)` for representable, non-zero-divisor
operations.

Division or remainder by zero is an error. `Int::MIN / -1` and `Int::MIN % -1` are
overflow because the exact quotient is outside the `Int` range. Semantic constant
preflight and runtime execution share the same `nova-int-semantics` truth table so the
same operation cannot be accepted at compile time and fail differently at runtime.

`Int::abs(n)` returns `n` when `n >= 0` and otherwise returns checked unary `-n`.
Consequently `Int::abs(Int::MIN)` is an overflow rather than wrapping or saturating.
The operand is evaluated exactly once. Semantic canonicalization lowers the operation
through an ordinary local binding, comparison, conditional, and unary negation HIR, so
the existing checked arithmetic and control-flow semantics remain authoritative rather
than introducing a parallel interpreter primitive. Missing payloads and non-`Int`
payloads are rejected by normal semantic diagnostics.

## Comparison, classification, and conversions

`Int` supports `==`, `!=`, `<`, `<=`, `>`, and `>=`.

Three explicit sign predicates classify an `Int` relative to zero:

- `Int::is_negative(n)` is equivalent to `n < 0`;
- `Int::is_zero(n)` is equivalent to `n == 0`;
- `Int::is_positive(n)` is equivalent to `n > 0`.

Two explicit parity predicates classify the remainder modulo two:

- `Int::is_even(n)` is equivalent to `n % 2 == 0`;
- `Int::is_odd(n)` is equivalent to `n % 2 != 0`.

These predicates follow Nova's signed remainder contract, so negative odd values remain
odd and both `Int::MIN` and zero are even. Each predicate evaluates its operand exactly
once. Semantic canonicalization lowers the spellings to ordinary typed arithmetic and
comparison HIR, so arithmetic checking, comparison typing, control-flow, diagnostics,
and runtime behavior remain the single source of truth. A missing payload or a payload
whose type is not `Int` is rejected rather than coerced.

The bootstrap language also has explicit conversions between `Bool` and `Int`:

- `Int::from(false)` evaluates to `0`;
- `Int::from(true)` evaluates to `1`;
- `Bool::from(0)` evaluates to `false`;
- `Bool::from(n)` evaluates to `true` for every non-zero `Int`, including `Int::MIN`.

Each conversion operand is evaluated exactly once and retains the ordinary control-flow
and side-effect semantics of any other expression. Semantic canonicalization lowers
`Int::from(Bool)` to ordinary typed conditional HIR and `Bool::from(Int)` to an ordinary
`!= 0` comparison, so the interpreter does not carry parallel conversion opcodes or a
second source of truth. Missing payloads and operands of the wrong type are rejected by
normal semantic diagnostics. There are no implicit conversions.

Nova now has two integer families: signed 64-bit `Int` and unsigned 64-bit `UInt`.
Unsuffixed decimal literals continue to default to `Int`; there is no implicit conversion
between the families. `UInt::MIN` is `0`, `UInt::MAX` is `2^64 - 1`, and same-family
`UInt` arithmetic and ordering are checked unsigned 64-bit operations. Overflow and
underflow report runtime `N4002`; zero divisors report `N4003`. Unary negation remains
`Int`-only.

`UInt::from(Int)` is the explicit widening-domain conversion and rejects negative values
with runtime `N4007`. `Int::from_uint(UInt)` is the explicit narrowing conversion and
rejects values above `Int::MAX` with `N4007`. Both evaluate their operand exactly once,
never wrap or saturate, and mixed-family arithmetic remains a type error.

## Tooling representation

Semantic-inspection schema v7 is the first version that represents `UInt`. Its type
table adds `uint`; expressions add `unsigned_integer` for canonical boundary constants
and `numeric_conversion` with operator `int_to_uint` or `uint_to_int`. Schemas v1-v6
remain frozen and reject any accepted HIR containing `UInt` with `N5001`; callers must
explicitly select v7. Inspection never invents an implicit conversion or literal suffix.

Literal suffixes, floating-point semantics, additional numeric families, richer defaulting,
and backend representation beyond these 64-bit contracts remain future BIL-5 work.
