from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected one anchor in {path}, found {count}: {old[:100]!r}")
    p.write_text(text.replace(old, new, 1))


replace_once(
    "crates/nova-parser/src/ast.rs",
    '''    /// Built-in or nominal type name.\n    Named(Name),\n    /// Explicit callable signature such as `fn(Int) -> Bool`.\n''',
    '''    /// Built-in or nominal type name.\n    Named(Name),\n    /// Uninhabited bottom type written `!`.\n    Never,\n    /// Explicit callable signature such as `fn(Int) -> Bool`.\n''',
)

replace_once(
    "crates/nova-parser/src/parser.rs",
    '''        if let Some(keyword) = self.consume(TokenKind::Fn) {\n''',
    '''        if let Some(bang) = self.consume(TokenKind::Bang) {\n            return Some(TypeRef {\n                kind: TypeRefKind::Never,\n                span: bang.span,\n            });\n        }\n\n        if let Some(keyword) = self.consume(TokenKind::Fn) {\n''',
)

replace_once(
    "crates/nova-sema/src/analyzer.rs",
    '''        match &reference.kind {\n            ast::TypeRefKind::Named(name) => match name.text.as_str() {\n''',
    '''        match &reference.kind {\n            ast::TypeRefKind::Never => Type::Never,\n            ast::TypeRefKind::Named(name) => match name.text.as_str() {\n''',
)

replace_once(
    "crates/nova-sema/src/analyzer.rs",
    '''                                "the bootstrap semantic core recognizes Int, Bool, Unit, declared record or enum names, and explicit function types",\n''',
    '''                                "the bootstrap semantic core recognizes Int, Bool, Unit, !, declared record or enum names, and explicit function types",\n''',
)

Path("crates/nova-parser/tests").mkdir(parents=True, exist_ok=True)
Path("crates/nova-parser/tests/surface_never_type.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::{ast::TypeRefKind, parse};
use nova_source::{SourceFile, SourceId};

#[test]
fn parses_never_in_direct_and_nested_type_positions() {
    let source = SourceFile::new(
        SourceId::new(0),
        "never.nv",
        "fn sink(value: !) -> ! { while true {} }\nfn higher(f: fn() -> !) -> fn(!) -> ! { higher }",
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);

    assert!(matches!(parsed.program.functions[0].parameters[0].ty.kind, TypeRefKind::Never));
    assert!(matches!(parsed.program.functions[0].return_type.kind, TypeRefKind::Never));

    let TypeRefKind::Function { return_type, .. } = &parsed.program.functions[1].parameters[0].ty.kind else {
        panic!("expected function parameter type");
    };
    assert!(matches!(return_type.kind, TypeRefKind::Never));

    let TypeRefKind::Function { parameters, return_type } = &parsed.program.functions[1].return_type.kind else {
        panic!("expected function return type");
    };
    assert!(matches!(parameters[0].kind, TypeRefKind::Never));
    assert!(matches!(return_type.kind, TypeRefKind::Never));
}
''')

Path("crates/nova-sema/tests/surface_never_type.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::Type};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "never.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn surface_never_reuses_existing_bottom_type_and_branch_join() {
    let analyzed = analyze_text(
        "fn forever() -> ! { while true {} }\n\
         fn choose(flag: Bool) -> Int { if flag { 42 } else { forever() } }\n\
         fn main() -> Int { choose(true) }",
    );
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    assert_eq!(analyzed.program.functions[0].return_type, Type::Never);
    assert_eq!(analyzed.program.functions[0].body.ty, Type::Never);
    assert_eq!(analyzed.program.functions[1].return_type, Type::Int);
    assert_eq!(analyzed.program.functions[1].body.ty, Type::Int);
}

#[test]
fn never_function_rejects_continuing_fallthrough_and_tail_values() {
    let fallthrough = analyze_text("fn bad() -> ! {}");
    assert!(fallthrough.diagnostics.iter().any(|diagnostic| diagnostic.code == "N3007"));

    let tail = analyze_text("fn bad() -> ! { () }");
    assert!(tail.diagnostics.iter().any(|diagnostic| diagnostic.code == "N3004"));
}
''')

Path("crates/nova-cli/tests/fixtures/valid/surface-never-type.nv").write_text(r'''fn forever() -> ! {
    while true {}
}

fn choose(flag: Bool) -> Int {
    if flag { 42 } else { forever() }
}

fn main() -> Int {
    choose(true)
}
''')

Path("crates/nova-cli/tests/surface_never_type.rs").write_text(r'''use std::path::Path;
use std::process::Command;

fn fixture() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/valid/surface-never-type.nv")
}

#[test]
fn check_run_and_all_inspection_versions_accept_surface_never() {
    let checked = Command::new(env!("CARGO_BIN_EXE_nova"))
        .args(["check", fixture()])
        .output()
        .expect("nova check should execute");
    assert!(checked.status.success(), "{}", String::from_utf8_lossy(&checked.stderr));

    let run = Command::new(env!("CARGO_BIN_EXE_nova"))
        .args(["run", fixture()])
        .output()
        .expect("nova run should execute");
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42\n");

    for version in ["1", "2", "3"] {
        let inspected = Command::new(env!("CARGO_BIN_EXE_nova"))
            .args(["inspect", fixture(), "--format=json", "--schema-version", version])
            .output()
            .expect("nova inspect should execute");
        assert!(inspected.status.success(), "schema {version}: {}", String::from_utf8_lossy(&inspected.stderr));
        let json = String::from_utf8(inspected.stdout).expect("inspection output is UTF-8");
        assert!(json.contains("\"kind\": \"never\""), "schema {version}: {json}");
        assert!(json.contains("\"display\": \"!\""), "schema {version}: {json}");
    }

    assert!(Path::new(fixture()).exists());
}
''')

Path("crates/nova-inspect/tests/surface_never_type.rs").write_text(r'''use nova_inspect::{build_document, build_document_v2, build_document_v3, v1};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::analyze;
use nova_source::{SourceFile, SourceId};

#[test]
fn every_supported_schema_projects_surface_never_without_a_version_bump() {
    let source = SourceFile::new(
        SourceId::new(0),
        "never.nv",
        "fn forever() -> ! { while true {} } fn main() -> Int { 42 }",
    );
    let lexed = lex(&source);
    let parsed = parse(&source, &lexed.tokens);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);

    let v1_document = build_document(&analyzed.program, &source).expect("v1 accepts Never");
    let v2_document = build_document_v2(&analyzed, &source).expect("v2 accepts Never");
    let v3_document = build_document_v3(&analyzed, &source).expect("v3 accepts Never");

    for program in [&v1_document.program, &v2_document.program, &v3_document.program] {
        assert!(program.types.iter().any(|ty| ty.kind == v1::TypeKind::Never && ty.display == "!"));
        assert_eq!(program.functions[0].return_type, program.types.iter().find(|ty| ty.kind == v1::TypeKind::Never).unwrap().id);
    }
}
''')
