//! Tokenization for the implemented Nova v0.1 frontend grammar.

use nova_diagnostics::Diagnostic;
use nova_source::{SourceFile, Span};
use std::fmt;

/// Kind and decoded value of one lexical token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenKind {
    /// An ASCII identifier.
    Identifier,
    /// A checked integer magnitude. Source radix is erased after lexing.
    Integer(u64),
    /// A validated UTF-8 string literal. The decoded value is recovered from its span.
    String,
    /// `fn`.
    Fn,
    /// `record`.
    Record,
    /// `enum`.
    Enum,
    /// `match`.
    Match,
    /// `new`.
    New,
    /// `let`.
    Let,
    /// `var`.
    Var,
    /// `if`.
    If,
    /// `else`.
    Else,
    /// `while`.
    While,
    /// `break`.
    Break,
    /// `continue`.
    Continue,
    /// `return`.
    Return,
    /// `true`.
    True,
    /// `false`.
    False,
    /// `(`.
    LeftParen,
    /// `)`.
    RightParen,
    /// `{`.
    LeftBrace,
    /// `}`.
    RightBrace,
    /// `,`.
    Comma,
    /// `:`.
    Colon,
    /// `::`.
    ColonColon,
    /// `;`.
    Semicolon,
    /// `.`.
    Dot,
    /// `->`.
    Arrow,
    /// `+`.
    Plus,
    /// `-`.
    Minus,
    /// `*`.
    Star,
    /// `/`.
    Slash,
    /// `%`.
    Percent,
    /// `=`.
    Equal,
    /// `==`.
    EqualEqual,
    /// `=>`.
    FatArrow,
    /// `!`.
    Bang,
    /// `!=`.
    BangEqual,
    /// `<`.
    Less,
    /// `<=`.
    LessEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterEqual,
    /// `&&`.
    AndAnd,
    /// `||`.
    OrOr,
    /// Synthetic end-of-file marker.
    Eof,
}

impl TokenKind {
    /// Returns a stable description for diagnostics.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Identifier => "identifier",
            Self::Integer(_) => "integer literal",
            Self::String => "string literal",
            Self::Fn => "`fn`",
            Self::Record => "`record`",
            Self::Enum => "`enum`",
            Self::Match => "`match`",
            Self::New => "`new`",
            Self::Let => "`let`",
            Self::Var => "`var`",
            Self::If => "`if`",
            Self::Else => "`else`",
            Self::While => "`while`",
            Self::Break => "`break`",
            Self::Continue => "`continue`",
            Self::Return => "`return`",
            Self::True => "`true`",
            Self::False => "`false`",
            Self::LeftParen => "`(`",
            Self::RightParen => "`)`",
            Self::LeftBrace => "`{`",
            Self::RightBrace => "`}`",
            Self::Comma => "`,`",
            Self::Colon => "`:`",
            Self::ColonColon => "`::`",
            Self::Semicolon => "`;`",
            Self::Dot => "`.`",
            Self::Arrow => "`->`",
            Self::Plus => "`+`",
            Self::Minus => "`-`",
            Self::Star => "`*`",
            Self::Slash => "`/`",
            Self::Percent => "`%`",
            Self::Equal => "`=`",
            Self::EqualEqual => "`==`",
            Self::FatArrow => "`=>`",
            Self::Bang => "`!`",
            Self::BangEqual => "`!=`",
            Self::Less => "`<`",
            Self::LessEqual => "`<=`",
            Self::Greater => "`>`",
            Self::GreaterEqual => "`>=`",
            Self::AndAnd => "`&&`",
            Self::OrOr => "`||`",
            Self::Eof => "end of file",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.description())
    }
}

/// One token with an exact source span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Token {
    /// Token category and decoded literal value, if any.
    pub kind: TokenKind,
    /// Exact source bytes occupied by the token.
    pub span: Span,
}

/// Complete deterministic result of lexing one source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexOutput {
    /// Tokens, always terminated by exactly one `Eof` token.
    pub tokens: Vec<Token>,
    /// Lexical errors in source order.
    pub diagnostics: Vec<Diagnostic>,
}

impl LexOutput {
    /// Reports whether lexing found any errors.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Lexes a validated UTF-8 source without panicking on source contents.
#[must_use]
pub fn lex(source: &SourceFile) -> LexOutput {
    Lexer::new(source).run()
}

/// Decodes a string-literal token after lexical validation.
///
/// The accepted escape set is deliberately small and deterministic: `\\`, `\"`,
/// `\n`, `\r`, `\t`, and `\0`. Unescaped UTF-8 scalar values are preserved.
/// Invalid spans, delimiters, escapes, and unescaped control characters fail
/// closed instead of manufacturing a value for a malformed token stream.
#[must_use]
pub fn decode_string_literal(source: &SourceFile, span: Span) -> Option<String> {
    let text = source.slice(span)?;
    let contents = text.strip_prefix('"')?.strip_suffix('"')?;
    let mut decoded = String::with_capacity(contents.len());
    let mut characters = contents.chars();

    while let Some(character) = characters.next() {
        if character == '\\' {
            let escaped = match characters.next()? {
                '\\' => '\\',
                '"' => '"',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '0' => '\0',
                _ => return None,
            };
            decoded.push(escaped);
        } else if character == '"' || character.is_control() {
            return None;
        } else {
            decoded.push(character);
        }
    }

    Some(decoded)
}

struct Lexer<'source> {
    source: &'source SourceFile,
    offset: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl<'source> Lexer<'source> {
    fn new(source: &'source SourceFile) -> Self {
        Self {
            source,
            offset: 0,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn run(mut self) -> LexOutput {
        while self.offset < self.source.len() {
            if self.skip_trivia() {
                continue;
            }
            if self.offset >= self.source.len() {
                break;
            }
            self.lex_token();
        }

        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: self.source.eof_span(),
        });
        LexOutput {
            tokens: self.tokens,
            diagnostics: self.diagnostics,
        }
    }

    fn skip_trivia(&mut self) -> bool {
        let start = self.offset;
        while matches!(self.current_byte(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.offset += 1;
        }

        if self.starts_with("//") {
            self.offset += 2;
            while !matches!(self.current_byte(), None | Some(b'\n')) {
                self.offset += 1;
            }
        } else if self.starts_with("/*") {
            self.skip_block_comment();
        }

        self.offset != start
    }

    fn skip_block_comment(&mut self) {
        let opening = self.offset;
        self.offset += 2;
        let mut depth = 1_usize;

        while self.offset < self.source.len() {
            if self.starts_with("/*") {
                depth += 1;
                self.offset += 2;
            } else if self.starts_with("*/") {
                depth -= 1;
                self.offset += 2;
                if depth == 0 {
                    return;
                }
            } else {
                self.offset += 1;
            }
        }

        let span = self.span(opening, (opening + 2).min(self.source.len()));
        self.diagnostics.push(
            Diagnostic::error("N1003", "unterminated block comment")
                .with_primary(span, "this comment is never closed")
                .with_note("block comments may be nested, and every `/*` needs a matching `*/`"),
        );
    }

    fn lex_token(&mut self) {
        let start = self.offset;
        let Some(character) = self.remaining().chars().next() else {
            return;
        };

        if character.is_ascii_alphabetic() || character == '_' {
            self.lex_identifier(start);
            return;
        }
        if character.is_ascii_digit() {
            self.lex_integer(start);
            return;
        }
        if character == '"' {
            self.lex_string(start);
            return;
        }

        self.offset += character.len_utf8();
        let kind = match character {
            '(' => Some(TokenKind::LeftParen),
            ')' => Some(TokenKind::RightParen),
            '{' => Some(TokenKind::LeftBrace),
            '}' => Some(TokenKind::RightBrace),
            ',' => Some(TokenKind::Comma),
            ':' if self.consume_if(b':') => Some(TokenKind::ColonColon),
            ':' => Some(TokenKind::Colon),
            ';' => Some(TokenKind::Semicolon),
            '.' => Some(TokenKind::Dot),
            '+' => Some(TokenKind::Plus),
            '*' => Some(TokenKind::Star),
            '/' => Some(TokenKind::Slash),
            '%' => Some(TokenKind::Percent),
            '-' if self.consume_if(b'>') => Some(TokenKind::Arrow),
            '-' => Some(TokenKind::Minus),
            '=' if self.consume_if(b'=') => Some(TokenKind::EqualEqual),
            '=' if self.consume_if(b'>') => Some(TokenKind::FatArrow),
            '=' => Some(TokenKind::Equal),
            '!' if self.consume_if(b'=') => Some(TokenKind::BangEqual),
            '!' => Some(TokenKind::Bang),
            '<' if self.consume_if(b'=') => Some(TokenKind::LessEqual),
            '<' => Some(TokenKind::Less),
            '>' if self.consume_if(b'=') => Some(TokenKind::GreaterEqual),
            '>' => Some(TokenKind::Greater),
            '&' if self.consume_if(b'&') => Some(TokenKind::AndAnd),
            '|' if self.consume_if(b'|') => Some(TokenKind::OrOr),
            _ => None,
        };

        if let Some(kind) = kind {
            self.tokens.push(Token {
                kind,
                span: self.span(start, self.offset),
            });
        } else {
            let span = self.span(start, self.offset);
            self.diagnostics.push(
                Diagnostic::error("N1001", format!("unexpected character {character:?}"))
                    .with_primary(span, "this character is not part of the Nova v0.1 grammar"),
            );
        }
    }

    fn lex_identifier(&mut self, start: usize) {
        while matches!(
            self.current_byte(),
            Some(byte) if byte.is_ascii_alphanumeric() || byte == b'_'
        ) {
            self.offset += 1;
        }

        let text = self.source.text().get(start..self.offset).unwrap_or("");
        let kind = match text {
            "fn" => TokenKind::Fn,
            "record" => TokenKind::Record,
            "enum" => TokenKind::Enum,
            "match" => TokenKind::Match,
            "new" => TokenKind::New,
            "let" => TokenKind::Let,
            "var" => TokenKind::Var,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            _ => TokenKind::Identifier,
        };
        self.tokens.push(Token {
            kind,
            span: self.span(start, self.offset),
        });
    }

    fn lex_integer(&mut self, start: usize) {
        while matches!(
            self.current_byte(),
            Some(byte) if byte.is_ascii_alphanumeric() || byte == b'_'
        ) {
            self.offset += 1;
        }

        let text = self.source.text().get(start..self.offset).unwrap_or("");
        let (radix, digits, digit_hint) = if let Some(digits) =
            text.strip_prefix("0b").or_else(|| text.strip_prefix("0B"))
        {
            (2_u32, digits, "binary digits")
        } else if let Some(digits) = text.strip_prefix("0o").or_else(|| text.strip_prefix("0O")) {
            (8_u32, digits, "octal digits")
        } else if let Some(digits) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
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

    fn lex_string(&mut self, start: usize) {
        debug_assert_eq!(self.current_byte(), Some(b'"'));
        self.offset += 1;

        while self.offset < self.source.len() {
            let character_start = self.offset;
            let Some(character) = self.remaining().chars().next() else {
                break;
            };
            match character {
                '"' => {
                    self.offset += 1;
                    self.tokens.push(Token {
                        kind: TokenKind::String,
                        span: self.span(start, self.offset),
                    });
                    return;
                }
                '\n' | '\r' => break,
                '\\' => {
                    self.offset += 1;
                    let Some(escaped) = self.remaining().chars().next() else {
                        break;
                    };
                    if matches!(escaped, '\n' | '\r') {
                        break;
                    }
                    self.offset += escaped.len_utf8();
                    if !matches!(escaped, '\\' | '"' | 'n' | 'r' | 't' | '0') {
                        self.diagnostics.push(
                            Diagnostic::error("N1006", "invalid string escape")
                                .with_primary(
                                    self.span(character_start, self.offset),
                                    "supported escapes are `\\\\`, `\\\"`, `\\n`, `\\r`, `\\t`, and `\\0`",
                                ),
                        );
                    }
                }
                character if character.is_control() => {
                    self.offset += character.len_utf8();
                    self.diagnostics.push(
                        Diagnostic::error("N1006", "unescaped control character in string literal")
                            .with_primary(
                                self.span(character_start, self.offset),
                                "write this control character with a supported escape",
                            ),
                    );
                }
                _ => self.offset += character.len_utf8(),
            }
        }

        self.diagnostics.push(
            Diagnostic::error("N1005", "unterminated string literal")
                .with_primary(
                    self.span(start, (start + 1).min(self.source.len())),
                    "this string literal is never closed",
                )
                .with_note("string literals must close before the end of the source line"),
        );
    }

    fn current_byte(&self) -> Option<u8> {
        self.source.text().as_bytes().get(self.offset).copied()
    }

    fn consume_if(&mut self, expected: u8) -> bool {
        if self.current_byte() == Some(expected) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn starts_with(&self, expected: &str) -> bool {
        self.remaining().starts_with(expected)
    }

    fn remaining(&self) -> &str {
        self.source.text().get(self.offset..).unwrap_or("")
    }

    fn span(&self, start: usize, end: usize) -> Span {
        self.source
            .span(start, end)
            .unwrap_or(self.source.eof_span())
    }
}

#[cfg(test)]
mod tests {
    use super::{TokenKind, decode_string_literal, lex};
    use nova_source::{SourceFile, SourceId};

    fn source(text: &str) -> SourceFile {
        SourceFile::new(SourceId::new(0), "test.nv", text)
    }

    #[test]
    fn lexes_keywords_operators_and_exact_spans() {
        let source = source(
            "record Pair { left: Int, right: Int } enum Maybe { None, Some(Int) } fn yes(x: Maybe) -> Bool { let p = new Pair { left: 1, right: 2 }; while p.left >= 1 && true { continue; break; } return match x { Maybe::None => false, Maybe::Some(value) => value > 0, }; } true }",
        );
        let output = lex(&source);
        let kinds = output
            .tokens
            .iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>();

        assert!(output.diagnostics.is_empty());
        assert_eq!(kinds[0], TokenKind::Record);
        assert!(kinds.contains(&TokenKind::Enum));
        assert!(kinds.contains(&TokenKind::Match));
        assert!(kinds.contains(&TokenKind::ColonColon));
        assert!(kinds.contains(&TokenKind::FatArrow));
        assert!(kinds.contains(&TokenKind::New));
        assert!(kinds.contains(&TokenKind::Fn));
        assert!(kinds.contains(&TokenKind::While));
        assert!(kinds.contains(&TokenKind::Break));
        assert!(kinds.contains(&TokenKind::Continue));
        assert!(kinds.contains(&TokenKind::Dot));
        assert!(kinds.contains(&TokenKind::GreaterEqual));
        assert!(kinds.contains(&TokenKind::AndAnd));
        assert!(kinds.contains(&TokenKind::Return));
        assert_eq!(kinds.last(), Some(&TokenKind::Eof));
    }

    #[test]
    fn skips_line_and_nested_block_comments() {
        let source = source("1 /* outer /* inner */ done */ + // line\n 2");
        let output = lex(&source);
        let kinds = output
            .tokens
            .iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>();

        assert!(output.diagnostics.is_empty());
        assert_eq!(
            kinds,
            vec![
                TokenKind::Integer(1),
                TokenKind::Plus,
                TokenKind::Integer(2),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn diagnoses_unterminated_nested_comment_at_opening() {
        let source = source("let x = 1; /* outer /* inner */");
        let output = lex(&source);

        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(output.diagnostics[0].code, "N1003");
        assert_eq!(
            source.slice(output.diagnostics[0].labels[0].span),
            Some("/*")
        );
    }

    #[test]
    fn checks_integer_shape_and_overflow() {
        for text in ["1_", "1__2", "123abc"] {
            let output = lex(&source(text));
            assert_eq!(output.diagnostics[0].code, "N1002", "source: {text}");
        }

        let max = lex(&source("9_223_372_036_854_775_807"));
        assert_eq!(max.tokens[0].kind, TokenKind::Integer(i64::MAX as u64));
        assert!(max.diagnostics.is_empty());

        let min_magnitude = lex(&source("9_223_372_036_854_775_808"));
        assert_eq!(
            min_magnitude.tokens[0].kind,
            TokenKind::Integer(1_u64 << 63)
        );
        assert!(min_magnitude.diagnostics.is_empty());

        let overflow_source = source("9223372036854775809");
        let overflow = lex(&overflow_source);
        assert_eq!(overflow.diagnostics[0].code, "N1004");
        assert_eq!(
            overflow.tokens,
            vec![super::Token {
                kind: TokenKind::Eof,
                span: overflow_source.eof_span(),
            }]
        );
    }

    #[test]
    fn lexes_and_decodes_utf8_strings_with_the_closed_escape_set() {
        let source = source(r#""Nova 🦀\n\"quote\"\\tab\tzero\0" true"#);
        let output = lex(&source);

        assert!(output.is_success(), "{:?}", output.diagnostics);
        assert_eq!(output.tokens[0].kind, TokenKind::String);
        assert_eq!(
            source.slice(output.tokens[0].span),
            Some(r#""Nova 🦀\n\"quote\"\\tab\tzero\0""#)
        );
        assert_eq!(
            decode_string_literal(&source, output.tokens[0].span),
            Some("Nova 🦀\n\"quote\"\\tab\tzero\0".to_owned())
        );
        assert_eq!(output.tokens[1].kind, TokenKind::True);
    }

    #[test]
    fn rejects_invalid_escapes_and_unescaped_control_characters_exactly() {
        let invalid_escape_source = source(r#""bad\q""#);
        let invalid_escape = lex(&invalid_escape_source);
        assert_eq!(invalid_escape.diagnostics.len(), 1);
        assert_eq!(invalid_escape.diagnostics[0].code, "N1006");
        assert_eq!(
            invalid_escape_source.slice(invalid_escape.diagnostics[0].labels[0].span),
            Some(r"\q")
        );
        assert_eq!(invalid_escape.tokens[0].kind, TokenKind::String);
        assert_eq!(
            decode_string_literal(&invalid_escape_source, invalid_escape.tokens[0].span),
            None
        );

        let control_source = source("\"raw\ttab\"");
        let control = lex(&control_source);
        assert_eq!(control.diagnostics.len(), 1);
        assert_eq!(control.diagnostics[0].code, "N1006");
        assert_eq!(
            control_source.slice(control.diagnostics[0].labels[0].span),
            Some("\t")
        );

        let synthetic_source = source(r#""first"second""#);
        assert_eq!(
            decode_string_literal(
                &synthetic_source,
                synthetic_source
                    .span(0, synthetic_source.len())
                    .expect("whole-source span"),
            ),
            None
        );
    }

    #[test]
    fn rejects_unterminated_strings_at_the_opening_quote_without_eating_the_next_line() {
        for text in [
            "\"end of file",
            "\"first line\ntrue",
            "\"continued\\\nfalse",
        ] {
            let source = source(text);
            let output = lex(&source);
            assert_eq!(output.diagnostics.len(), 1, "source: {text:?}");
            assert_eq!(output.diagnostics[0].code, "N1005");
            assert_eq!(
                source.slice(output.diagnostics[0].labels[0].span),
                Some("\"")
            );
            assert!(
                !output
                    .tokens
                    .iter()
                    .any(|token| token.kind == TokenKind::String)
            );
        }
    }

    #[test]
    fn rejects_non_ascii_identifiers_with_character_exact_spans() {
        let source = source("let β = 1;");
        let output = lex(&source);

        assert_eq!(output.diagnostics[0].code, "N1001");
        assert_eq!(
            source.slice(output.diagnostics[0].labels[0].span),
            Some("β")
        );
    }

    #[test]
    fn handles_arbitrary_valid_utf8_without_panicking_or_losing_eof() {
        for text in ["", "\0", "🦀", "/*/**/*/", "&&&|||", "9__x", "}\n{;"] {
            let output = lex(&source(text));
            assert_eq!(
                output.tokens.last().map(|token| token.kind),
                Some(TokenKind::Eof)
            );
        }
    }

    #[test]
    fn handles_every_ascii_code_point_as_source_input() {
        for byte in 0_u8..=127 {
            let text = char::from(byte).to_string();
            let output = lex(&source(&text));
            assert_eq!(
                output.tokens.last().map(|token| token.kind),
                Some(TokenKind::Eof),
                "ASCII byte {byte} lost the EOF token"
            );
        }
    }
}
