from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected one anchor in {path}, found {count}: {old[:120]!r}")
    p.write_text(text.replace(old, new, 1))


replace_once(
    "crates/nova-parser/src/ast.rs",
    "    /// Explicit function return.\n    Return(Expression),\n",
    "    /// Explicit function return; `None` is the bare `return;` form.\n    Return(Option<Expression>),\n",
)

replace_once(
    "crates/nova-parser/src/parser.rs",
    '''    fn parse_return_statement(&mut self) -> Option<Statement> {\n        let keyword = self.expect(TokenKind::Return, "to start a return statement")?;\n        let expression = self.parse_expression()?;\n        let semicolon = self.expect(TokenKind::Semicolon, "after the returned expression")?;\n        Some(Statement {\n            span: self.cover(keyword.span, semicolon.span),\n            kind: StatementKind::Return(expression),\n        })\n    }\n''',
    '''    fn parse_return_statement(&mut self) -> Option<Statement> {\n        let keyword = self.expect(TokenKind::Return, "to start a return statement")?;\n        let expression = if self.at(TokenKind::Semicolon) {\n            None\n        } else {\n            Some(self.parse_expression()?)\n        };\n        let semicolon = self.expect(TokenKind::Semicolon, "after `return`")?;\n        Some(Statement {\n            span: self.cover(keyword.span, semicolon.span),\n            kind: StatementKind::Return(expression),\n        })\n    }\n''',
)

replace_once(
    "crates/nova-sema/src/hir.rs",
    "    /// Explicit function return.\n    Return(Expression),\n",
    "    /// Explicit function return; `None` preserves a source-level bare `return;`.\n    Return(Option<Expression>),\n",
)

replace_once(
    "crates/nova-sema/src/analyzer.rs",
    '''            ast::StatementKind::Return(expression) => {\n                let expression = self.lower_expression(expression, return_type);\n                self.require_type(\n                    &expression.ty,\n                    return_type,\n                    expression.span,\n                    "return expression",\n                );\n                if !expression.ty.is_never() {\n                    self.flow_advance(\n                        FlowNodeKind::Transfer(FlowTransfer::Return),\n                        Some(statement.span),\n                    );\n                }\n                (StatementKind::Return(expression), true)\n            }\n''',
    '''            ast::StatementKind::Return(expression) => {\n                let expression = expression\n                    .as_ref()\n                    .map(|expression| self.lower_expression(expression, return_type));\n                if let Some(expression) = &expression {\n                    self.require_type(\n                        &expression.ty,\n                        return_type,\n                        expression.span,\n                        "return expression",\n                    );\n                    if !expression.ty.is_never() {\n                        self.flow_advance(\n                            FlowNodeKind::Transfer(FlowTransfer::Return),\n                            Some(statement.span),\n                        );\n                    }\n                } else {\n                    self.require_type(&Type::Unit, return_type, statement.span, "bare return");\n                    self.flow_advance(\n                        FlowNodeKind::Transfer(FlowTransfer::Return),\n                        Some(statement.span),\n                    );\n                }\n                (StatementKind::Return(expression), true)\n            }\n''',
)

replace_once(
    "crates/nova-interpreter/src/lib.rs",
    '''            StatementKind::Return(expression) => match self.eval_expression(expression, frame)? {\n                Flow::Value(value) | Flow::Return(value) => Ok(Some(Flow::Return(value))),\n                Flow::Break => Ok(Some(Flow::Break)),\n                Flow::Continue => Ok(Some(Flow::Continue)),\n            },\n''',
    '''            StatementKind::Return(expression) => {\n                let Some(expression) = expression else {\n                    return Ok(Some(Flow::Return(Value::Unit)));\n                };\n                match self.eval_expression(expression, frame)? {\n                    Flow::Value(value) | Flow::Return(value) => Ok(Some(Flow::Return(value))),\n                    Flow::Break => Ok(Some(Flow::Break)),\n                    Flow::Continue => Ok(Some(Flow::Continue)),\n                }\n            }\n''',
)

replace_once(
    "crates/nova-inspect/src/lib.rs",
    '''            hir::StatementKind::Return(value) => {\n                expressions.push(self.collect_expression(value, owner)?);\n                v1::StatementKind::Return\n            }\n''',
    '''            hir::StatementKind::Return(value) => {\n                if let Some(value) = value {\n                    expressions.push(self.collect_expression(value, owner)?);\n                }\n                v1::StatementKind::Return\n            }\n''',
)

Path("crates/nova-parser/tests/bare_return.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::{ast::StatementKind, parse};
use nova_source::{SourceFile, SourceId};

#[test]
fn preserves_bare_and_value_return_forms() {
    let source = SourceFile::new(
        SourceId::new(0),
        "bare-return.nv",
        "fn bare() -> Unit { return; } fn explicit() -> Unit { return (); }",
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);

    assert!(matches!(
        parsed.program.functions[0].body.statements[0].kind,
        StatementKind::Return(None)
    ));
    assert!(matches!(
        parsed.program.functions[1].body.statements[0].kind,
        StatementKind::Return(Some(_))
    ));
}
''')

Path("crates/nova-sema/tests/bare_return.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze, hir::{StatementKind, Type}};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "bare-return.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn bare_return_is_unit_typed_noncontinuation_without_a_synthetic_expression() {
    let output = analyze_text("fn stop() -> Unit { return; }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let function = &output.program.functions[0];
    assert_eq!(function.return_type, Type::Unit);
    assert_eq!(function.body.ty, Type::Never);
    assert!(matches!(function.body.statements[0].kind, StatementKind::Return(None)));
}

#[test]
fn bare_return_reuses_ordinary_return_type_mismatch_for_non_unit_functions() {
    for source in [
        "fn main() -> Int { return; }",
        "fn main() -> Bool { return; }",
        "fn main() -> ! { return; }",
    ] {
        let output = analyze_text(source);
        assert!(
            output.diagnostics.iter().any(|diagnostic| diagnostic.code == "N3004"),
            "{source}: {:?}",
            output.diagnostics
        );
    }
}

#[test]
fn explicit_unit_return_remains_value_bearing_hir() {
    let output = analyze_text("fn stop() -> Unit { return (); }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(matches!(
        output.program.functions[0].body.statements[0].kind,
        StatementKind::Return(Some(_))
    ));
}
''')

Path("crates/nova-interpreter/tests/bare_return.rs").write_text(r'''use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::Type};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "bare-return.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn bare_unit_return_executes_as_unit_and_calls_continue_normally() {
    let analyzed = analyze_text(
        "fn stop() -> Unit { return; } fn main() -> Int { stop(); 42 }",
    );
    assert_eq!(execute(&analyzed.program).expect("program executes"), Value::Int(42));
}

#[test]
fn runtime_boundary_rejects_bare_return_retyped_to_int() {
    let mut analyzed = analyze_text("fn main() -> Unit { return; }");
    analyzed.program.functions[0].return_type = Type::Int;
    let error = execute(&analyzed.program).expect_err("Unit return must not inhabit Int");
    assert_eq!(error.code, "N4005");
}
''')

Path("crates/nova-inspect/tests/bare_return.rs").write_text(r'''use nova_inspect::{build_document, build_document_v2, build_document_v3, v1};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::analyze;
use nova_source::{SourceFile, SourceId};

#[test]
fn all_existing_schemas_preserve_bare_return_as_a_return_without_child_expression() {
    let source = SourceFile::new(SourceId::new(0), "bare-return.nv", "fn main() -> Unit { return; }");
    let lexed = lex(&source);
    let parsed = parse(&source, &lexed.tokens);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);

    let v1_document = build_document(&analyzed.program, &source).expect("v1 document");
    let v2_document = build_document_v2(&analyzed, &source).expect("v2 document");
    let v3_document = build_document_v3(&analyzed, &source).expect("v3 document");
    for program in [&v1_document.program, &v2_document.program, &v3_document.program] {
        let statement = program
            .statements
            .iter()
            .find(|statement| statement.kind == v1::StatementKind::Return)
            .expect("return statement");
        assert!(statement.expressions.is_empty());
    }
}
''')

Path("crates/nova-cli/tests/fixtures/valid/bare-unit-return.nv").write_text('''fn stop() -> Unit {\n    return;\n}\n\nfn main() -> Int {\n    stop();\n    42\n}\n''')

Path("crates/nova-cli/tests/bare_unit_return.rs").write_text(r'''use std::process::Command;

fn fixture() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/valid/bare-unit-return.nv")
}

#[test]
fn check_run_ast_and_existing_inspection_schemas_accept_bare_unit_return() {
    for command in ["check", "ast"] {
        let output = Command::new(env!("CARGO_BIN_EXE_nova"))
            .args([command, fixture()])
            .output()
            .expect("nova command executes");
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }

    let run = Command::new(env!("CARGO_BIN_EXE_nova"))
        .args(["run", fixture()])
        .output()
        .expect("nova run executes");
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42\n");

    for version in ["1", "2", "3"] {
        let output = Command::new(env!("CARGO_BIN_EXE_nova"))
            .args(["inspect", fixture(), "--format=json", "--schema-version", version])
            .output()
            .expect("nova inspect executes");
        assert!(
            output.status.success(),
            "schema {version}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
''')
