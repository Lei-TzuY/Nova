use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "closed-arithmetic-diagnostic-contract.nv",
        text,
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

fn only_diagnostic<'a>(output: &'a AnalysisOutput, code: &str) -> &'a nova_diagnostics::Diagnostic {
    let matching = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1, "{:?}", output.diagnostics);
    matching[0]
}

fn assert_primary_exact(text: &str, diagnostic: &nova_diagnostics::Diagnostic, needle: &str) {
    let start = text.find(needle).expect("expected source fragment");
    let primary = diagnostic.labels.first().expect("expected primary label");
    assert_eq!(primary.span.start(), start, "{diagnostic:?}");
    assert_eq!(primary.span.end(), start + needle.len(), "{diagnostic:?}");
    assert!(
        primary.message.contains("closed arithmetic expression"),
        "{diagnostic:?}"
    );
}

#[test]
fn block_local_zero_divisor_points_at_the_failing_expression() {
    let text = r#"
        fn main() -> Int {
            let value = {
                let zero = 0;
                let bad = 1 / zero;
                7
            };
            value
        }
    "#;
    let output = analyze_text(text);
    let diagnostic = only_diagnostic(&output, "N3032");

    assert_eq!(diagnostic.message, "constant zero divisor");
    assert_primary_exact(text, diagnostic, "1 / zero");
}

#[test]
fn selected_nested_overflow_points_at_the_failing_expression() {
    let text = r#"
        fn main() -> Int {
            let value = if true {
                {
                    let max = 9223372036854775807;
                    let bad = max + 1;
                    7
                }
            } else {
                0
            };
            value
        }
    "#;
    let output = analyze_text(text);
    let diagnostic = only_diagnostic(&output, "N3031");

    assert_eq!(diagnostic.message, "constant Int arithmetic overflow");
    assert_primary_exact(text, diagnostic, "max + 1");
}

#[test]
fn direct_literal_failure_keeps_the_outer_operation_span() {
    let text = "fn main() -> Int { 10 / (3 - 3) }";
    let output = analyze_text(text);
    let diagnostic = only_diagnostic(&output, "N3032");

    assert_primary_exact(text, diagnostic, "10 / (3 - 3)");
}

#[test]
fn distinct_failures_keep_distinct_primary_spans() {
    let text = r#"
        fn main() -> Int {
            {
                let zero = 0;
                let first = 1 / zero;
                ()
            };
            {
                let max = 9223372036854775807;
                let second = max + 1;
                ()
            };
            0
        }
    "#;
    let output = analyze_text(text);
    let zero_divisor = only_diagnostic(&output, "N3032");
    let overflow = only_diagnostic(&output, "N3031");

    assert_primary_exact(text, zero_divisor, "1 / zero");
    assert_primary_exact(text, overflow, "max + 1");
    assert_ne!(zero_divisor.labels[0].span, overflow.labels[0].span);
}
