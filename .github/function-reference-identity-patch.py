from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one anchor, found {count}")
    file.write_text(text.replace(old, new, 1))

# HIR: retain source-resolved top-level function spelling beside stable FunctionId.
replace_once(
    "crates/nova-sema/src/hir.rs",
    "    /// Reference to a top-level function.\n    Function(FunctionId),",
    "    /// Reference to a top-level function.\n    Function {\n        /// Stable source-order function identity.\n        function: FunctionId,\n        /// Source-resolved function spelling paired with the stable identity.\n        function_name: String,\n    },",
    "function reference HIR",
)

# Sema producer: preserve exactly the spelling that resolved to the function symbol.
replace_once(
    "crates/nova-sema/src/analyzer.rs",
    "        if let Some(symbol) = self.functions.get(&name.text) {\n            return (\n                ExpressionKind::Function(symbol.id),\n                Type::Function(symbol.signature.clone()),\n            );\n        }",
    "        if let Some(symbol) = self.functions.get(&name.text) {\n            return (\n                ExpressionKind::Function {\n                    function: symbol.id,\n                    function_name: name.text.clone(),\n                },\n                Type::Function(symbol.signature.clone()),\n            );\n        }",
    "function reference lowering",
)

# Closed-condition proof is an analyzer-owned consumer; identity proof remains FunctionId-based.
replace_once(
    "crates/nova-sema/src/constant_condition.rs",
    "        ExpressionKind::Function(function) => Some(*function),",
    "        ExpressionKind::Function { function, .. } => Some(*function),",
    "closed function identity proof",
)

# Runtime trusted boundary: revalidate source-resolved name against FunctionId before creating Value::Function.
replace_once(
    "crates/nova-interpreter/src/lib.rs",
    "            ExpressionKind::Function(function) => Ok(Flow::Value(Value::Function(*function))),",
    "            ExpressionKind::Function {\n                function,\n                function_name,\n            } => {\n                self.resolved_function_reference(*function, function_name, expression.span)?;\n                Ok(Flow::Value(Value::Function(*function)))\n            }",
    "runtime function reference evaluation",
)
replace_once(
    "crates/nova-interpreter/src/lib.rs",
    "    fn resolved_enum_variant(\n",
    "    fn resolved_function_reference(\n        &self,\n        function: FunctionId,\n        function_name: &str,\n        span: nova_source::Span,\n    ) -> Result<&Function, Diagnostic> {\n        let Some(definition) = self.program.functions.get(function.index()) else {\n            return Err(self.invariant(\n                span,\n                format!(\"resolved function id {} is outside the program\", function.index()),\n            ));\n        };\n        if definition.id != function {\n            return Err(self.invariant(\n                span,\n                \"function declaration index does not match its resolved identity\",\n            ));\n        }\n        if definition.name != function_name {\n            return Err(self.invariant(\n                span,\n                format!(\n                    \"resolved function `{function_name}` does not match declaration id {} (`{}`)\",\n                    function.index(), definition.name\n                ),\n            ));\n        }\n        Ok(definition)\n    }\n\n    fn resolved_enum_variant(\n",
    "runtime resolved function helper",
)

# Inspector trusted boundary: name/id/signature must agree before publishing existing stable function target ID.
replace_once(
    "crates/nova-inspect/src/lib.rs",
    "            hir::ExpressionKind::Function(resolved) => {\n                self.require_function(*resolved)?;\n                target = Some(function_id(resolved.index()));\n                v1::ExpressionKind::FunctionReference\n            }",
    "            hir::ExpressionKind::Function {\n                function,\n                function_name,\n            } => {\n                let declaration = self.require_function(*function)?;\n                if declaration.name != *function_name {\n                    return Err(InspectionError::invalid(format!(\n                        \"function reference `{function_name}` does not match declaration id {} (`{}`)\",\n                        function.index(), declaration.name\n                    )));\n                }\n                let expected_type = function_type(declaration);\n                if expression.ty != expected_type {\n                    return Err(InspectionError::invalid(format!(\n                        \"function reference `{function_name}` type {} does not match declaration signature {}\",\n                        expression.ty, expected_type\n                    )));\n                }\n                target = Some(function_id(function.index()));\n                v1::ExpressionKind::FunctionReference\n            }",
    "inspection function reference identity",
)

# Existing malformed signature-drift fixture must remain internally coherent on name/id before changing type.
replace_once(
    "crates/nova-interpreter/tests/function_equality.rs",
    "    right.kind = ExpressionKind::Function(flag);",
    "    right.kind = ExpressionKind::Function {\n        function: flag,\n        function_name: \"flag\".to_owned(),\n    };",
    "function equality malformed fixture",
)

# Sema producer regression.
Path("crates/nova-sema/tests/function_reference_identity.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

#[test]
fn direct_function_reference_retains_resolved_name_and_identity() {
    let source = SourceFile::new(
        SourceId::new(0),
        "function-reference-identity.nv",
        "fn first() -> Int { 1 } fn second() -> Int { 2 } fn main() -> Int { first() }",
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);

    let tail = analyzed.program.functions[2]
        .body
        .tail
        .as_deref()
        .expect("main tail");
    let ExpressionKind::Call { callee, .. } = &tail.kind else {
        panic!("expected call HIR");
    };
    let ExpressionKind::Function {
        function,
        function_name,
    } = &callee.kind
    else {
        panic!("expected direct function reference");
    };
    assert_eq!(*function, analyzed.program.functions[0].id);
    assert_eq!(function_name, "first");
}
''')

# Runtime same-signature sibling retargeting regression plus valid alias control.
Path("crates/nova-interpreter/tests/function_reference_identity.rs").write_text(r'''use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "function-reference-identity.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn rejects_same_signature_direct_function_retargeting() {
    let mut analyzed = analyze_text(
        "fn first() -> Int { 1 } fn second() -> Int { 2 } fn main() -> Int { first() }",
    );
    let second = analyzed.program.functions[1].id;
    let tail = analyzed.program.functions[2]
        .body
        .tail
        .as_deref_mut()
        .expect("main tail");
    let ExpressionKind::Call { callee, .. } = &mut tail.kind else {
        panic!("call HIR");
    };
    let ExpressionKind::Function { function, function_name } = &mut callee.kind else {
        panic!("function reference HIR");
    };
    assert_eq!(function_name, "first");
    *function = second;

    let error = execute(&analyzed.program).expect_err("retargeted function reference must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn validated_function_alias_keeps_runtime_declaration_identity() {
    let analyzed = analyze_text(
        "fn first() -> Int { 7 } fn main() -> Int { let alias = first; alias() }",
    );
    let value = execute(&analyzed.program).expect("validated alias should execute");
    assert_eq!(value, Value::Int(7));
}
''')

# Inspector corruption regressions: same-signature ID drift and signature metadata drift.
Path("crates/nova-inspect/tests/function_reference_identity.rs").write_text(r'''use nova_inspect::build_document;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::{ExpressionKind, FunctionType, Type}};
use nova_source::{SourceFile, SourceId};

fn checked(text: &str) -> (SourceFile, nova_sema::hir::Program) {
    let source = SourceFile::new(SourceId::new(0), "function-reference-inspect.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed.program)
}

fn callee_mut(program: &mut nova_sema::hir::Program) -> &mut nova_sema::hir::Expression {
    let tail = program.functions[2]
        .body
        .tail
        .as_deref_mut()
        .expect("main tail");
    let ExpressionKind::Call { callee, .. } = &mut tail.kind else {
        panic!("call HIR");
    };
    callee
}

#[test]
fn rejects_same_signature_function_name_id_drift() {
    let (source, mut program) = checked(
        "fn first() -> Int { 1 } fn second() -> Int { 2 } fn main() -> Int { first() }",
    );
    let second = program.functions[1].id;
    let callee = callee_mut(&mut program);
    let ExpressionKind::Function { function, function_name } = &mut callee.kind else {
        panic!("function reference HIR");
    };
    assert_eq!(function_name, "first");
    *function = second;

    let error = build_document(&program, &source).expect_err("function identity drift must fail");
    assert!(error.message().contains("function reference `first`"));
}

#[test]
fn rejects_function_reference_signature_drift() {
    let (source, mut program) = checked(
        "fn first() -> Int { 1 } fn second() -> Int { 2 } fn main() -> Int { first() }",
    );
    let callee = callee_mut(&mut program);
    callee.ty = Type::Function(FunctionType {
        parameters: Vec::new(),
        return_type: Box::new(Type::Bool),
    });

    let error = build_document(&program, &source).expect_err("function type drift must fail");
    assert!(error.message().contains("does not match declaration signature"));
}
''')
