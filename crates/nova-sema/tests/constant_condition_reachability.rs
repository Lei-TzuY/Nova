use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "constant-condition.nv", text);
    let lexed = lex(&source);
    assert!(
        lexed.is_success(),
        "lex diagnostics: {:?}",
        lexed.diagnostics
    );
    let parsed = parse(&source, &lexed.tokens);
    assert!(
        parsed.is_success(),
        "parse diagnostics: {:?}",
        parsed.diagnostics
    );
    analyze(&parsed.program)
}

#[test]
fn literal_false_while_body_is_diagnostic_only_for_execution_failures() {
    let output = analyze_text("fn main() -> Int { while false { 1 / 0; } 0 }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn closed_integer_comparison_selects_if_flow_without_folding_hir() {
    let output = analyze_text(
        "fn main() -> Int { var value: Int; if 1 + 1 == 2 { value = 42; () } else { () }; value }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);

    let condition = output.program.functions[0]
        .body
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            nova_sema::hir::StatementKind::Expression(expression) => match &expression.kind {
                ExpressionKind::If { condition, .. } => Some(condition.as_ref()),
                _ => None,
            },
            _ => None,
        })
        .expect("main should retain the if condition in HIR");
    assert!(matches!(condition.kind, ExpressionKind::Binary { .. }));
}

#[test]
fn known_false_comparison_makes_dead_if_branch_diagnostic_only() {
    let output = analyze_text("fn main() -> Int { if 2 < 1 { 1 / 0 } else { 0 } }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn known_true_comparison_makes_loop_noncontinuing() {
    let output = analyze_text("fn main() -> Int { while 3 > 2 { } }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn derived_short_circuit_truth_controls_flow_and_dead_rhs_diagnostics() {
    let forced =
        analyze_text("fn main() -> Int { var value: Int; (1 > 2) || { value = 42; true }; value }");
    assert!(forced.is_success(), "{:?}", forced.diagnostics);

    let skipped = analyze_text("fn main() -> Bool { (1 < 2) || (1 / 0 == 0) }");
    assert!(skipped.is_success(), "{:?}", skipped.diagnostics);
}

#[test]
fn pure_unit_blocks_participate_in_closed_equality_reachability() {
    for text in [
        "fn main() -> Int { var value: Int; if ({}) == ({ () }) { value = 42; () } else { () }; value }",
        "fn main() -> Int { var value: Int; if ({ { () } }) != () { () } else { value = 42; () }; value }",
    ] {
        let output = analyze_text(text);
        assert!(
            output.is_success(),
            "source: {text}; diagnostics: {:?}",
            output.diagnostics
        );
    }
}

#[test]
fn names_and_calls_remain_dynamic_conditions() {
    for text in [
        "fn truth() -> Bool { true } fn main() -> Int { var value: Int; if truth() { value = 1; () } else { () }; value }",
        "fn main() -> Int { var flag = true; var value: Int; if flag { value = 1; () } else { () }; value }",
        "fn unit() -> Unit {} fn main() -> Int { var value: Int; if ({ unit(); }) == () { value = 1; () } else { () }; value }",
    ] {
        let output = analyze_text(text);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "N3009"),
            "source: {text}; diagnostics: {:?}",
            output.diagnostics
        );
    }
}

#[test]
fn closed_discarded_statement_block_can_drive_reachability() {
    let text =
        "fn main() -> Int { var value: Int; if ({ (); }) == () { value = 1; () } else { () }; value }";
    let output = analyze_text(text);
    assert!(
        output.is_success(),
        "source: {text}; diagnostics: {:?}",
        output.diagnostics
    );
}
