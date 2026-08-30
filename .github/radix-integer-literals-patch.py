from pathlib import Path
import re

lexer = Path("crates/nova-lexer/src/lib.rs")
text = lexer.read_text()
text = text.replace(
    "    /// A checked decimal integer magnitude. Signed meaning is assigned later.\n",
    "    /// A checked integer magnitude. Source radix is erased after lexing.\n",
)
pattern = re.compile(
    r"    fn lex_integer\(&mut self, start: usize\) \{.*?\n    \}\n\n    fn current_byte",
    re.S,
)
replacement = '''    fn lex_integer(&mut self, start: usize) {
        while matches!(
            self.current_byte(),
            Some(byte) if byte.is_ascii_alphanumeric() || byte == b'_'
        ) {
            self.offset += 1;
        }

        let text = self.source.text().get(start..self.offset).unwrap_or("");
        let (radix, digits, digit_hint) = if let Some(digits) = text
            .strip_prefix("0b")
            .or_else(|| text.strip_prefix("0B"))
        {
            (2_u32, digits, "binary digits")
        } else if let Some(digits) = text
            .strip_prefix("0o")
            .or_else(|| text.strip_prefix("0O"))
        {
            (8_u32, digits, "octal digits")
        } else if let Some(digits) = text
            .strip_prefix("0x")
            .or_else(|| text.strip_prefix("0X"))
        {
            (16_u32, digits, "hexadecimal digits")
        } else {
            (10_u32, text, "decimal digits")
        };

        let malformed = digits.is_empty()
            || digits.starts_with('_')
            || digits.ends_with('_')
            || digits.contains("__")
            || digits
                .chars()
                .filter(|character| *character != '_')
                .any(|character| character.to_digit(radix).is_none());
        if malformed {
            self.diagnostics.push(
                Diagnostic::error("N1002", "malformed integer literal").with_primary(
                    self.span(start, self.offset),
                    format!("use {digit_hint} with single separators between digits"),
                ),
            );
            return;
        }

        const MAX_SIGNED_INT_MAGNITUDE: u64 = 1_u64 << 63;
        let value = digits
            .chars()
            .filter(|character| *character != '_')
            .try_fold(0_u64, |value, character| {
                let digit = u64::from(character.to_digit(radix)?);
                value.checked_mul(u64::from(radix))?.checked_add(digit)
            })
            .filter(|value| *value <= MAX_SIGNED_INT_MAGNITUDE);

        if let Some(value) = value {
            self.tokens.push(Token {
                kind: TokenKind::Integer(value),
                span: self.span(start, self.offset),
            });
        } else {
            self.diagnostics.push(
                Diagnostic::error("N1004", "integer literal magnitude is out of range")
                    .with_primary(
                        self.span(start, self.offset),
                        "the bootstrap frontend accepts integer magnitudes up to 2^63",
                    )
                    .with_note(
                        "the largest accepted magnitude is reserved for the signed Int minimum under prefix `-`",
                    ),
            );
        }
    }

    fn current_byte'''
text, count = pattern.subn(replacement, text, count=1)
if count != 1:
    raise SystemExit("could not replace lex_integer")
lexer.write_text(text)

cli = Path("crates/nova-cli/tests/cli.rs")
text = cli.read_text()
needle = '        "valid/int-division.nv",\n'
if '"valid/radix-integers.nv"' not in text:
    if needle not in text:
        raise SystemExit("could not find positive fixture anchor")
    text = text.replace(needle, needle + '        "valid/radix-integers.nv",\n', 1)
run_needle = '        ("valid/int-division.nv", "-21\\n", None),\n'
if '("valid/radix-integers.nv", "42\\n", None)' not in text:
    if run_needle not in text:
        raise SystemExit("could not find run fixture anchor")
    text = text.replace(
        run_needle,
        run_needle + '        ("valid/radix-integers.nv", "42\\n", None),\n',
        1,
    )
cli.write_text(text)
