use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{StatementKind, Type};
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "literal-if.nv", text);
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

fn codes(output: &AnalysisOutput) -> Vec<&str> {
    output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

#[test]
fn literal_if_selects_definite_initialization_from_the_live_branch() {
    for text in [
        "fn f() -> Int { var value: Int; if true { value = 1; 0 } else { 0 }; value }",
        "fn f() -> Int { var value: Int; if false { 0 } else { value = 1; 0 }; value }",
    ] {
        let output = analyze_text(text);
        assert!(output.is_success(), "{text}: {:?}", output.diagnostics);
    }

    for text in [
        "fn f() -> Int { var value: Int; if true { 0 } else { value = 1; 0 }; value }",
        "fn f() -> Int { var value: Int; if false { value = 1; 0 } else { 0 }; value }",
    ] {
        let output = analyze_text(text);
        assert_eq!(codes(&output), vec!["N3009"], "{text}");
    }
}

#[test]
fn literal_if_dead_breaks_do_not_create_guaranteed_loop_exits() {
    for text in [
        "fn f() -> Int { while true { if false { break; } else { continue; }; } }",
        "fn f() -> Int { while true { if true { continue; } else { break; }; } }",
    ] {
        let output = analyze_text(text);
        assert!(output.is_success(), "{text}: {:?}", output.diagnostics);
        assert!(output.program.functions[0].body.ty.is_never(), "{text}");
    }
}

#[test]
fn literal_if_live_breaks_remain_guaranteed_loop_exits() {
    for text in [
        "fn f() -> Int { while true { if true { break; } else { continue; }; } }",
        "fn f() -> Int { while true { if false { continue; } else { break; }; } }",
    ] {
        let output = analyze_text(text);
        assert_eq!(codes(&output), vec!["N3007"], "{text}");
    }
}

#[test]
fn literal_if_uses_the_live_branch_for_noncontinuation() {
    for text in [
        "fn f() -> Int { if true { return 1; } else { 2 }; 3 }",
        "fn f() -> Int { if false { 2 } else { return 1; }; 3 }",
    ] {
        let output = analyze_text(text);
        assert!(output.is_success(), "{text}: {:?}", output.diagnostics);
        let StatementKind::Expression(expression) =
            &output.program.functions[0].body.statements[0].kind
        else {
            panic!("expected if expression statement: {text}");
        };
        assert_eq!(expression.ty, Type::Never, "{text}");
    }
}

#[test]
fn literal_if_dead_branches_remain_statically_checked() {
    let output = analyze_text("fn f() -> Int { if true { 1 } else { missing } }");
    assert_eq!(codes(&output), vec!["N3003"]);

    let output = analyze_text("fn f() -> Int { if true { 1 } else { false } }");
    assert_eq!(codes(&output), vec!["N3004"]);
}
