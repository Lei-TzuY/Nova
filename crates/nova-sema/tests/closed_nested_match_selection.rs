use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "closed_nested_match.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn selected_if_enum_scrutinee_can_drive_a_closed_int_match() {
    let analyzed = analyze_text(
        "enum Choice { A, B }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while ((match (if true { Choice::B } else { Choice::A }) {\n\
                 Choice::A => 0,\n\
                 Choice::B => 42,\n\
             }) == 42) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn selected_match_enum_scrutinee_can_drive_another_closed_match() {
    let analyzed = analyze_text(
        "enum Outer { First, Second }\n\
         enum Choice { A, B }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while ((match (match Outer::Second {\n\
                 Outer::First => Choice::A,\n\
                 Outer::Second => Choice::B,\n\
             }) {\n\
                 Choice::A => 0,\n\
                 Choice::B => 42,\n\
             }) == 42) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn dynamic_if_condition_keeps_nested_match_runtime_only() {
    let analyzed = analyze_text(
        "enum Choice { A, B }\n\
         fn main(flag: Bool) -> Int {\n\
             var value: Int;\n\
             while ((match (if flag { Choice::B } else { Choice::A }) {\n\
                 Choice::A => 0,\n\
                 Choice::B => 42,\n\
             }) == 42) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(!analyzed.is_success());
    assert!(
        analyzed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3009"),
        "{:?}",
        analyzed.diagnostics
    );
}

#[test]
fn selected_immutable_enum_binding_scrutinee_branch_can_drive_nested_match() {
    let analyzed = analyze_text(
        "enum Choice { A, B }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while ((match (if true {\n\
                 let choice = Choice::B;\n\
                 choice\n\
             } else { Choice::A }) {\n\
                 Choice::A => 0,\n\
                 Choice::B => 42,\n\
             }) == 42) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}

#[test]
fn selected_mutable_enum_binding_scrutinee_branch_stops_nested_match_proof() {
    let analyzed = analyze_text(
        "enum Choice { A, B }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while ((match (if true {\n\
                 var choice = Choice::B;\n\
                 choice\n\
             } else { Choice::A }) {\n\
                 Choice::A => 0,\n\
                 Choice::B => 42,\n\
             }) == 42) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(!analyzed.is_success());
    assert!(
        analyzed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "N3009"),
        "{:?}",
        analyzed.diagnostics
    );
}

#[test]
fn statement_bearing_unselected_scrutinee_branch_does_not_block_nested_match_proof() {
    let analyzed = analyze_text(
        "enum Choice { A, B }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             while ((match (if true { Choice::B } else {\n\
                 let choice = Choice::A;\n\
                 choice\n\
             }) {\n\
                 Choice::A => 0,\n\
                 Choice::B => 42,\n\
             }) == 42) {\n\
                 value = 42;\n\
                 break;\n\
             }\n\
             value\n\
         }",
    );

    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
}
