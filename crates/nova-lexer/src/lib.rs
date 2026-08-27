//! Tokenization for the implemented Nova v0.1 frontend grammar.

use nova_diagnostics::Diagnostic;
use nova_source::{SourceFile, Span};
use std::fmt;

/// Kind and decoded value of one lexical token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenKind {
    /// An ASCII identifier.
    Identifier,
    /// A checked decimal integer literal.
    Integer(i64),
    /// `fn`.
    Fn,
    /// `let`.
    Let,
    /// `var`.
    Var,
    /// `if`.
    If,
    /// `else`.
    Else,
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
    /// `;`.
    Semicolon,
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
            Self::Fn => "`fn`",
            Self::Let => "`let`",
            Self::Var => "`var`",
            Self::If => "`if`",
            Self::Else => "`else`",
            Self::Return => "`return`",
            Self::True => "`true`",
            Self::False => "`false`",
            Self::LeftParen => "`(`",
            Self::RightParen => "`)`",
            Self::LeftBrace => "`{`",
            Self::RightBrace => "`}`",
            Self::Comma => "`,`",
            Self::Colon => "`:`",
            Self::Semicolon => "`;`",
            Self::Arrow => "`->`",
            Self::Plus => "`+`",
            Self::Minus => "`-`",
            Self::Star => "`*`",
            Self::Slash => "`/`",
            Self::Percent => "`%`",
            Self::Equal => "`=`",
            Self::EqualEqual => "`==`",
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
    pub const fn is_success(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Lexes a validated UTF-8 source without panicking on source contents.
#[must_use]
pub fn lex(source: &SourceFile) -> LexOutput {
    Lexer::new(source).run()
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

        self.offset += character.len_utf8();
        let kind = match character {
            '(' => Some(TokenKind::LeftParen),
            ')' => Some(TokenKind::RightParen),
            '{' => Some(TokenKind::LeftBrace),
            '}' => Some(TokenKind::RightBrace),
            ',' => Some(TokenKind::Comma),
            ':' => Some(TokenKind::Colon),
            ';' => Some(TokenKind::Semicolon),
            '+' => Some(TokenKind::Plus),
            '*' => Some(TokenKind::Star),
            '/' => Some(TokenKind::Slash),
            '%' => Some(TokenKind::Percent),
            '-' if self.consume_if(b'>') => Some(TokenKind::Arrow),
            '-' => Some(TokenKind::Minus),
            '=' if self.consume_if(b'=') => Some(TokenKind::EqualEqual),
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
            "let" => TokenKind::Let,
            "var" => TokenKind::Var,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
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
        let malformed = text.ends_with('_')
            || text.contains("__")
            || text.bytes().any(|byte| byte.is_ascii_alphabetic());
        if malformed {
            self.diagnostics.push(
                Diagnostic::error("N1002", "malformed integer literal").with_primary(
                    self.span(start, self.offset),
                    "use decimal digits with single separators between digits",
                ),
            );
            return;
        }

        let value = text.bytes().filter(|byte| *byte != b'_').try_fold(
            0_i64,
            |value, byte| {
                value
                    .checked_mul(10)?
                    .checked_add(i64::from(byte - b'0'))
            },
        );

        if let Some(value) = value {
            self.tokens.push(Token {
                kind: TokenKind::Integer(value),
                span: self.span(start, self.offset),
            });
        } else {
            self.diagnostics.push(
                Diagnostic::error("N1004", "integer literal is out of range")
                    .with_primary(
                        self.span(start, self.offset),
                        "the current frontend accepts magnitudes up to 9223372036854775807",
                    )
                    .with_note("integer widths and overflow semantics are provisional in Nova v0.1"),
            );
        }
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
    use super::{TokenKind, lex};
    use nova_source::{SourceFile, SourceId};

    fn source(text: &str) -> SourceFile {
        SourceFile::new(SourceId::new(0), "test.nv", text)
    }

    #[test]
    fn lexes_keywords_operators_and_exact_spans() {
        let source = source("fn yes(x: Int) -> Bool { x >= 1 && true }");
        let output = lex(&source);
        let kinds = output
            .tokens
            .iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>();

        assert!(output.diagnostics.is_empty());
        assert_eq!(kinds[0], TokenKind::Fn);
        assert_eq!(kinds[1], TokenKind::Identifier);
        assert_eq!(kinds[10], TokenKind::Identifier);
        assert!(kinds.contains(&TokenKind::GreaterEqual));
        assert!(kinds.contains(&TokenKind::AndAnd));
        assert_eq!(kinds.last(), Some(&TokenKind::Eof));
        assert_eq!(source.slice(output.tokens[1].span), Some("yes"));
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
        assert_eq!(source.slice(output.diagnostics[0].labels[0].span), Some("/*"));
    }

    #[test]
    fn checks_integer_shape_and_overflow() {
        for text in ["1_", "1__2", "123abc"] {
            let output = lex(&source(text));
            assert_eq!(output.diagnostics[0].code, "N1002", "source: {text}");
        }

        let valid = lex(&source("9_223_372_036_854_775_807"));
        assert_eq!(valid.tokens[0].kind, TokenKind::Integer(i64::MAX));
        assert!(valid.diagnostics.is_empty());

        let overflow = lex(&source("9223372036854775808"));
        assert_eq!(overflow.diagnostics[0].code, "N1004");
        assert_eq!(overflow.tokens, vec![super::Token {
            kind: TokenKind::Eof,
            span: source("9223372036854775808").eof_span(),
        }]);
    }

    #[test]
    fn rejects_non_ascii_identifiers_with_character_exact_spans() {
        let source = source("let β = 1;");
        let output = lex(&source);

        assert_eq!(output.diagnostics[0].code, "N1001");
        assert_eq!(source.slice(output.diagnostics[0].labels[0].span), Some("β"));
    }

    #[test]
    fn handles_arbitrary_valid_utf8_without_panicking_or_losing_eof() {
        for text in ["", "\0", "🦀", "/*/**/*/", "&&&|||", "9__x", "}\n{;"] {
            let output = lex(&source(text));
            assert_eq!(output.tokens.last().map(|token| token.kind), Some(TokenKind::Eof));
        }
    }
}
