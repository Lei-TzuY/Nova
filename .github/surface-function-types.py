from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    if old not in text:
        raise SystemExit(f"anchor not found in {path}: {old[:80]!r}")
    p.write_text(text.replace(old, new, 1))


# AST: type syntax becomes recursive while preserving one exact source span.
replace_once(
    "crates/nova-parser/src/ast.rs",
    '''/// A parsed type name. Type meaning is assigned only in later phases.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct TypeRef {\n    /// Unresolved type name.\n    pub name: Name,\n    /// Range occupied by the type reference.\n    pub span: Span,\n}\n''',
    '''/// A parsed type reference. Type meaning is assigned only in later phases.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct TypeRef {\n    /// Surface form of this type reference.\n    pub kind: TypeRefKind,\n    /// Range occupied by the complete type reference.\n    pub span: Span,\n}\n\n/// Implemented surface type-reference forms.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub enum TypeRefKind {\n    /// Built-in or nominal type name.\n    Named(Name),\n    /// Explicit callable signature such as `fn(Int) -> Bool`.\n    Function {\n        /// Parameter types in declaration order.\n        parameters: Vec<TypeRef>,\n        /// Explicit return type.\n        return_type: Box<TypeRef>,\n    },\n}\n''',
)

# Parser: parse recursive function signatures with a dedicated finite nesting budget.
replace_once(
    "crates/nova-parser/src/parser.rs",
    '''    MatchArm, Name, Parameter, Program, Record, RecordField, RecordLiteralField, Statement,\n    StatementKind, TypeRef, UnaryOperator,\n''',
    '''    MatchArm, Name, Parameter, Program, Record, RecordField, RecordLiteralField, Statement,\n    StatementKind, TypeRef, TypeRefKind, UnaryOperator,\n''',
)
replace_once(
    "crates/nova-parser/src/parser.rs",
    '''const MAX_EXPRESSION_DEPTH: usize = 256;\n''',
    '''const MAX_EXPRESSION_DEPTH: usize = 256;\nconst MAX_TYPE_DEPTH: usize = 128;\n''',
)
replace_once(
    "crates/nova-parser/src/parser.rs",
    '''    fn parse_type_ref(&mut self, context: &str) -> Option<TypeRef> {\n        let name = self.parse_name(context)?;\n        Some(TypeRef {\n            span: name.span,\n            name,\n        })\n    }\n''',
    '''    fn parse_type_ref(&mut self, context: &str) -> Option<TypeRef> {\n        self.parse_type_ref_with_depth(context, 0)\n    }\n\n    fn parse_type_ref_with_depth(&mut self, context: &str, depth: usize) -> Option<TypeRef> {\n        if depth >= MAX_TYPE_DEPTH {\n            let token = self.current();\n            self.diagnostics.push(\n                Diagnostic::error("N2009", "type nesting limit exceeded").with_primary(\n                    token.span,\n                    format!(\n                        "the bootstrap parser accepts at most {MAX_TYPE_DEPTH} nested type frames"\n                    ),\n                ),\n            );\n            return None;\n        }\n\n        if let Some(keyword) = self.consume(TokenKind::Fn) {\n            self.expect(TokenKind::LeftParen, "after `fn` in a function type")?;\n            let mut parameters = Vec::new();\n            if !self.at(TokenKind::RightParen) {\n                loop {\n                    parameters.push(self.parse_type_ref_with_depth(\n                        "as a function-type parameter",\n                        depth + 1,\n                    )?);\n                    if self.consume(TokenKind::Comma).is_none() {\n                        break;\n                    }\n                    if self.at(TokenKind::RightParen) {\n                        break;\n                    }\n                }\n            }\n            self.expect(TokenKind::RightParen, "after function-type parameters")?;\n            self.expect(TokenKind::Arrow, "before a function-type return type")?;\n            let return_type =\n                self.parse_type_ref_with_depth("after `->` in a function type", depth + 1)?;\n            let span = self.cover(keyword.span, return_type.span);\n            return Some(TypeRef {\n                kind: TypeRefKind::Function {\n                    parameters,\n                    return_type: Box::new(return_type),\n                },\n                span,\n            });\n        }\n\n        let name = self.parse_name(context)?;\n        Some(TypeRef {\n            span: name.span,\n            kind: TypeRefKind::Named(name),\n        })\n    }\n''',
)

# Sema: recursively resolve the new surface callable type to the already-existing HIR FunctionType.
p = Path("crates/nova-sema/src/analyzer.rs")
text = p.read_text()
start = text.index("    fn resolve_type_ref(&mut self, reference: &ast::TypeRef) -> Type {")
end = text.index("\n    fn ", start + 8)
old_fn = text[start:end]
if "reference.name.text.as_str()" not in old_fn:
    raise SystemExit("resolve_type_ref shape changed unexpectedly")
# Keep the named-type body intact and only wrap it in the new AST kind match.
named_body = old_fn.replace(
    "    fn resolve_type_ref(&mut self, reference: &ast::TypeRef) -> Type {\n        match reference.name.text.as_str() {",
    "        match name.text.as_str() {",
    1,
)
# Drop one function-closing brace; the remaining block is the named branch body.
if not named_body.endswith("\n    }"):
    raise SystemExit("resolve_type_ref closing brace not found")
named_body = named_body[: -len("\n    }")]
named_body = named_body.replace("reference.span", "reference.span")
new_fn = '''    fn resolve_type_ref(&mut self, reference: &ast::TypeRef) -> Type {\n        match &reference.kind {\n            ast::TypeRefKind::Named(name) => {\n''' + "\n".join("    " + line if line else line for line in named_body.splitlines()) + '''\n            }\n            ast::TypeRefKind::Function {\n                parameters,\n                return_type,\n            } => Type::Function(FunctionType {\n                parameters: parameters\n                    .iter()\n                    .map(|parameter| self.resolve_type_ref(parameter))\n                    .collect(),\n                return_type: Box::new(self.resolve_type_ref(return_type)),\n            }),\n        }\n    }'''
# Remove the duplicated old function signature that named_body retained if present.
new_fn = new_fn.replace(
    "            fn resolve_type_ref(&mut self, reference: &ast::TypeRef) -> Type {\n",
    "",
    1,
)
p.write_text(text[:start] + new_fn + text[end:])

# Parser integration tests for recursive syntax and fail-closed depth.
Path("crates/nova-parser/tests/function_types.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::ast::TypeRefKind;
use nova_parser::parse;
use nova_source::{SourceFile, SourceId};

fn parse_text(text: &str) -> nova_parser::ParseOutput {
    let source = SourceFile::new(SourceId::new(0), "function-types.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "lex diagnostics: {:?}", lexed.diagnostics);
    parse(&source, &lexed.tokens)
}

#[test]
fn parses_recursive_surface_function_types() {
    let parsed = parse_text(
        "fn apply(f: fn(fn(Int) -> Int, Int) -> Int, x: Int) -> fn(Int) -> Int { f }",
    );
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let function = &parsed.program.functions[0];
    let TypeRefKind::Function { parameters, return_type } = &function.parameters[0].ty.kind else {
        panic!("parameter should be a function type");
    };
    assert_eq!(parameters.len(), 2);
    assert!(matches!(parameters[0].kind, TypeRefKind::Function { .. }));
    assert!(matches!(return_type.kind, TypeRefKind::Named(_)));
    assert!(matches!(function.return_type.kind, TypeRefKind::Function { .. }));
}

#[test]
fn rejects_pathological_function_type_nesting_without_unbounded_recursion() {
    let mut ty = "Int".to_owned();
    for _ in 0..140 {
        ty = format!("fn() -> {ty}");
    }
    let parsed = parse_text(&format!("fn main() -> {ty} {{ main }}"));
    assert!(
        parsed.diagnostics.iter().any(|diagnostic| diagnostic.code == "N2009"),
        "{:?}",
        parsed.diagnostics
    );
    assert!(parsed.diagnostics.len() < 20, "{:?}", parsed.diagnostics);
}
''')

# Semantic end-to-end higher-order source contract.
Path("crates/nova-sema/tests/surface_function_types.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{FunctionType, Type};
use nova_sema::analyze;
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "surface-function-types.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "lex diagnostics: {:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "parse diagnostics: {:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn resolves_nested_surface_function_types_to_existing_hir_signatures() {
    let output = analyze_text(
        "fn higher(f: fn(Int) -> Int) -> fn(Int) -> Int { f } fn inc(x: Int) -> Int { x + 1 }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let higher = &output.program.functions[0];
    let unary = Type::Function(FunctionType {
        parameters: vec![Type::Int],
        return_type: Box::new(Type::Int),
    });
    assert_eq!(higher.parameters[0].ty, unary);
    assert_eq!(higher.return_type, unary);
}

#[test]
fn higher_order_calls_are_checked_through_surface_function_annotations() {
    let output = analyze_text(
        "fn bad(f: fn(Bool) -> Int) -> Int { f(1) }",
    );
    assert!(output.has_errors());
    assert!(output.diagnostics.iter().any(|diagnostic| diagnostic.code == "N3004"));
}
''')

# CLI executable fixture: parameter, return value, local annotation, and invocation.
Path("crates/nova-cli/tests/fixtures/valid/higher-order-functions.nv").write_text(r'''fn add_one(value: Int) -> Int {
    value + 1
}

fn identity_transform(transform: fn(Int) -> Int) -> fn(Int) -> Int {
    transform
}

fn twice(transform: fn(Int) -> Int, value: Int) -> Int {
    transform(transform(value))
}

fn main() -> Int {
    let transform: fn(Int) -> Int = identity_transform(add_one);
    twice(transform, 40)
}
''')

# Wire the fixture into check and run.
p = Path("crates/nova-cli/tests/cli.rs")
text = p.read_text()
needle = '        "valid/radix-integers.nv",\n'
if needle not in text:
    raise SystemExit("CLI positive fixture anchor not found")
text = text.replace(needle, needle + '        "valid/higher-order-functions.nv",\n', 1)
needle = '        ("valid/radix-integers.nv", "42\\n", None),\n'
if needle not in text:
    raise SystemExit("CLI run fixture anchor not found")
text = text.replace(needle, needle + '        ("valid/higher-order-functions.nv", "42\\n", None),\n', 1)
p.write_text(text)
