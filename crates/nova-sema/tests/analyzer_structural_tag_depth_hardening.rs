use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "analyzer-structural-tag-depth-hardening.nv",
        text,
    );
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

fn code_count(output: &AnalysisOutput, code: &str) -> usize {
    output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .count()
}

fn nested_enum_definitions(depth: usize) -> String {
    let mut source = String::new();
    for index in 0..depth {
        let payload = if index + 1 == depth {
            "Leaf".to_string()
        } else {
            format!("E{}", index + 1)
        };
        source.push_str(&format!("enum E{index} {{ Empty, Value({payload}) }} "));
    }
    source.push_str("enum Leaf { A, B } ");
    source
}

fn nested_constructor(depth: usize, leaf: &str) -> String {
    let mut value = leaf.to_string();
    for index in (0..depth).rev() {
        value = format!("E{index}::Value({value})");
    }
    value
}

fn nested_match_chain(depth: usize) -> String {
    let mut value = format!("match v{depth} {{ Leaf::A => 1, Leaf::B => 2, }}");
    for index in (0..depth).rev() {
        let scrutinee = if index == 0 {
            "root".to_string()
        } else {
            format!("v{index}")
        };
        value = format!(
            "match {scrutinee} {{ E{index}::Empty => 0, E{index}::Value(v{}) => {value}, }}",
            index + 1
        );
    }
    value
}

#[test]
fn long_immutable_alias_chain_preserves_recursive_summary() {
    const ALIASES: usize = 128;

    let mut source = String::from(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main() -> Int { let v0 = Wrap::Value(new Holder { choice: Choice::A });",
    );
    for index in 1..=ALIASES {
        source.push_str(&format!(" let v{index} = v{};", index - 1));
    }
    source.push_str(&format!(
        " match v{ALIASES} {{ Wrap::Empty => 0, Wrap::Value(holder) => match holder.choice {{ Choice::A => 1, Choice::B => 2, }}, }} }}"
    ));

    let output = analyze_text(&source);
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn deep_recursive_enum_payload_chain_reaches_the_leaf() {
    const DEPTH: usize = 32;

    let mut source = nested_enum_definitions(DEPTH);
    source.push_str("fn main() -> Int { let root = ");
    source.push_str(&nested_constructor(DEPTH, "Leaf::A"));
    source.push_str("; ");
    source.push_str(&nested_match_chain(DEPTH));
    source.push_str(" }");

    let output = analyze_text(&source);
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), DEPTH + 1);
}

#[test]
fn deep_dynamic_leaf_cuts_off_only_the_unknown_suffix() {
    const DEPTH: usize = 24;

    let mut source = nested_enum_definitions(DEPTH);
    source.push_str("fn main(flag: Bool) -> Int { let root = ");
    source.push_str(&nested_constructor(
        DEPTH,
        "if flag { Leaf::A } else { Leaf::B }",
    ));
    source.push_str("; ");
    source.push_str(&nested_match_chain(DEPTH));
    source.push_str(" }");

    let output = analyze_text(&source);
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), DEPTH);
}

#[test]
fn shadow_after_long_alias_chain_does_not_reuse_stale_summary() {
    const ALIASES: usize = 96;

    let mut source = String::from(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main(flag: Bool) -> Int { let v0 = Wrap::Value(new Holder { choice: Choice::A });",
    );
    for index in 1..=ALIASES {
        source.push_str(&format!(" let v{index} = v{};", index - 1));
    }
    source.push_str(&format!(
        " match v{ALIASES} {{ Wrap::Empty => 0, Wrap::Value(holder) => {{ let holder = if flag {{ new Holder {{ choice: Choice::A }} }} else {{ new Holder {{ choice: Choice::B }} }}; match holder.choice {{ Choice::A => 1, Choice::B => 2, }} }}, }} }}"
    ));

    let output = analyze_text(&source);
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn nominally_distinct_isomorphic_structures_keep_separate_tags() {
    let output = analyze_text(
        "enum LeftChoice { A, B } enum RightChoice { A, B } record LeftHolder { choice: LeftChoice } record RightHolder { choice: RightChoice } enum LeftWrap { Empty, Value(LeftHolder) } enum RightWrap { Empty, Value(RightHolder) } fn main() -> Int { let left = LeftWrap::Value(new LeftHolder { choice: LeftChoice::A }); let right = RightWrap::Value(new RightHolder { choice: RightChoice::B }); match left { LeftWrap::Empty => 0, LeftWrap::Value(holder) => match holder.choice { LeftChoice::A => 1, LeftChoice::B => 2, }, }; match right { RightWrap::Empty => 0, RightWrap::Value(holder) => match holder.choice { RightChoice::A => 3, RightChoice::B => 4, }, } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 4);
}
