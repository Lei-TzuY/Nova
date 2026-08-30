use nova_lexer::{TokenKind, lex};
use nova_source::{SourceFile, SourceId};

fn source(text: &str) -> SourceFile {
    SourceFile::new(SourceId::new(0), "radix-integers.nv", text)
}

#[test]
fn lexes_binary_octal_and_hexadecimal_magnitudes() {
    let source = source(
        "0b1010_1010 0B10101010 0o52 0O52 0x2a 0X2A 0x7fff_ffff_ffff_ffff 0x8000_0000_0000_0000",
    );
    let output = lex(&source);

    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let integers = output
        .tokens
        .iter()
        .filter_map(|token| match token.kind {
            TokenKind::Integer(value) => Some(value),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        integers,
        vec![170, 170, 42, 42, 42, 42, i64::MAX as u64, 1_u64 << 63]
    );
}

#[test]
fn rejects_invalid_digits_and_separator_placement_as_one_literal() {
    for text in [
        "0b", "0b_1", "0b102", "0o8", "0xg", "0x1__0", "0x1_", "123abc",
    ] {
        let output = lex(&source(text));
        assert_eq!(
            output.diagnostics.len(),
            1,
            "{text}: {:?}",
            output.diagnostics
        );
        assert_eq!(output.diagnostics[0].code, "N1002", "{text}");
        assert!(
            output
                .tokens
                .iter()
                .all(|token| !matches!(token.kind, TokenKind::Integer(_))),
            "{text}: malformed literal must not yield a partial integer token"
        );
    }
}

#[test]
fn applies_the_signed_int_magnitude_ceiling_in_every_radix() {
    for text in [
        "0x8000_0000_0000_0001",
        "0o1000000000000000000001",
        "0b1_0000000000000000000000000000000000000000000000000000000000000001",
    ] {
        let output = lex(&source(text));
        assert_eq!(
            output.diagnostics.len(),
            1,
            "{text}: {:?}",
            output.diagnostics
        );
        assert_eq!(output.diagnostics[0].code, "N1004", "{text}");
    }
}
