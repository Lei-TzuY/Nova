from pathlib import Path

# Normative grammar.
path = Path("docs/grammar.md")
text = path.read_text()
old = '''letter          = "A" … "Z" | "a" … "z" ;
digit           = "0" … "9" ;
identifier      = (letter | "_") , { letter | digit | "_" } ;
integer         = digit , { [ "_" ] , digit } ;
'''
new = '''letter          = "A" … "Z" | "a" … "z" ;
digit           = "0" … "9" ;
binary_digit    = "0" | "1" ;
octal_digit     = "0" … "7" ;
hex_digit       = digit | "A" … "F" | "a" … "f" ;
identifier      = (letter | "_") , { letter | digit | "_" } ;
decimal_integer = digit , { [ "_" ] , digit } ;
binary_integer  = ("0b" | "0B") , binary_digit , { [ "_" ] , binary_digit } ;
octal_integer   = ("0o" | "0O") , octal_digit , { [ "_" ] , octal_digit } ;
hex_integer     = ("0x" | "0X") , hex_digit , { [ "_" ] , hex_digit } ;
integer         = decimal_integer | binary_integer | octal_integer | hex_integer ;
'''
if old not in text:
    raise SystemExit("grammar EBNF anchor not found")
text = text.replace(old, new, 1)
old = '''Integer separators cannot lead, trail, or repeat. Lexing preserves a checked decimal
magnitude and rejects magnitudes above `9223372036854775808`; it never wraps or
truncates. Semantic lowering interprets `Int` as signed for this bootstrap slice:
positive literals end at `9223372036854775807`, while the otherwise-reserved
`9223372036854775808` magnitude is accepted only under prefix `-`, giving the exact
minimum value `-9223372036854775808`.
'''
new = '''Integer separators cannot lead, trail, repeat, or immediately follow a radix prefix.
Decimal literals have no prefix; binary, octal, and hexadecimal literals use
`0b`/`0B`, `0o`/`0O`, and `0x`/`0X`. A prefixed literal must use digits valid for
that radix. Lexing erases source radix after decoding and preserves one checked
magnitude, rejecting values above `2^63` without wrapping or truncation. Semantic
lowering interprets that magnitude as signed `Int`: positive literals end at
`2^63 - 1`, while magnitude `2^63` in any supported radix is accepted only under
prefix `-`, giving the exact minimum value `-9223372036854775808`.
'''
if old not in text:
    raise SystemExit("grammar integer policy anchor not found")
path.write_text(text.replace(old, new, 1))

# Constitution: expand the provisional lexical evidence without freezing future widths.
path = Path("docs/language-constitution.md")
text = path.read_text()
text = text.replace(
    "ASCII identifiers, decimal integer literals, `//` line comments, and nested\n",
    "ASCII identifiers, decimal plus base-prefixed binary/octal/hexadecimal integer\n"
    "literals, `//` line comments, and nested\n",
    1,
)
old = '''The bootstrap frontend preserves decimal integer magnitudes through parsing and
assigns signed meaning during semantic lowering. Positive `Int` literals are
`0..=2^63-1`; the magnitude `2^63` is reserved for prefix negation, so
`-9223372036854775808` denotes the exact signed 64-bit minimum. A positive `2^63`
expression is rejected as semantic diagnostic `N3030`, and larger magnitudes are
rejected lexically as `N1004`. No literal is wrapped or truncated.
'''
new = '''The bootstrap frontend accepts decimal integer literals plus binary (`0b`/`0B`),
octal (`0o`/`0O`), and hexadecimal (`0x`/`0X`) forms. Single `_` separators may
appear only between digits. Lexing validates digits in the selected radix, decodes the
source spelling to one checked magnitude, and deliberately erases radix before parsing
and HIR lowering. Positive `Int` literals are `0..=2^63-1`; magnitude `2^63` in any
supported radix is reserved for prefix negation, so both `-9223372036854775808` and
`-0x8000_0000_0000_0000` denote the exact signed 64-bit minimum. A positive `2^63`
expression is rejected as semantic diagnostic `N3030`, and larger magnitudes are
rejected lexically as `N1004`. No literal is wrapped or truncated.
'''
if old not in text:
    raise SystemExit("constitution numeric anchor not found")
path.write_text(text.replace(old, new, 1))

# README user-facing status.
path = Path("README.md")
text = path.read_text()
old = '''The bootstrap frontend preserves decimal integer magnitudes through parsing and
assigns signed meaning during semantic lowering. Positive `Int` literals are
`0..=2^63-1`; the magnitude `2^63` is reserved for prefix negation, so
`-9223372036854775808` denotes the exact signed 64-bit minimum. A positive `2^63`
expression is rejected as semantic diagnostic `N3030`, and larger magnitudes are
rejected lexically as `N1004`. No literal is wrapped or truncated.
'''
new = '''Integer literals may be decimal or use binary (`0b`/`0B`), octal (`0o`/`0O`),
or hexadecimal (`0x`/`0X`) prefixes, with single `_` separators between digits.
The lexer validates the selected radix and decodes every accepted spelling to the same
checked magnitude before parsing, so arithmetic and type semantics do not depend on how
a value was written. Positive `Int` literals end at `2^63 - 1`; magnitude `2^63` in
any supported radix is reserved for prefix negation, making both decimal
`-9223372036854775808` and hexadecimal `-0x8000_0000_0000_0000` exact `Int::MIN`.
Larger magnitudes are rejected lexically and no literal is wrapped or truncated.
'''
if old not in text:
    raise SystemExit("README numeric anchor not found")
path.write_text(text.replace(old, new, 1))

# Roadmap: this is a frontend slice, not a fake semantic/runtime count increase.
path = Path("docs/roadmap.md")
text = path.read_text()
status = "**Status: first vertical slice implemented; broader grammar work remains.**"
if status not in text:
    raise SystemExit("Phase 1 status anchor not found")
text = text.replace(
    status,
    "**Status: two vertical slices implemented; broader grammar work remains.**",
    1,
)
anchor = '''- Rust 1.85 MSRV checking plus current-stable formatting, Clippy, test, build,
  and rustdoc CI.

Next Phase 1 refinements should be driven by the needs of later semantic work,
'''
insert = '''- Rust 1.85 MSRV checking plus current-stable formatting, Clippy, test, build,
  and rustdoc CI.

Implemented in the second Phase 1 slice:

- integer literals now accept decimal plus `0b`/`0B`, `0o`/`0O`, and `0x`/`0X`
  binary, octal, and hexadecimal spellings without introducing new numeric types;
- single `_` separators remain legal only between digits, including after a radix
  prefix only once at least one valid digit has been written;
- invalid radix digits and malformed separator placement remain one fail-closed
  lexical literal (`N1002`) rather than being split into misleading partial tokens;
- every radix decodes to the same checked magnitude contract capped at `2^63`, so
  prefix negation can represent `Int::MIN` equally as decimal or hexadecimal while
  positive `2^63` and larger magnitudes retain the established semantic/lexical errors;
- parser, HIR, arithmetic, runtime value, and inspection representations remain
  unchanged because source radix is intentionally erased at the lexer boundary; and
- focused lexer regressions plus a CLI check/run fixture cover all prefixes, separator
  policy, invalid digits, range failure, and exact hexadecimal `Int::MIN` execution.

Next Phase 1 refinements should be driven by the needs of later semantic work,
'''
if anchor not in text:
    raise SystemExit("Phase 1 insertion anchor not found")
path.write_text(text.replace(anchor, insert, 1))
