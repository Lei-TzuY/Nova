from pathlib import Path

p = Path('.github/surface-function-types.py')
text = p.read_text()
start = text.index('# Sema: recursively resolve the new surface callable type')
end = text.index('# Parser integration tests for recursive syntax', start)
replacement = r'''# Sema: recursively resolve the new surface callable type to the already-existing HIR FunctionType.
replace_once(
    "crates/nova-sema/src/analyzer.rs",
    ''' + "'''" + '''    fn resolve_type_ref(&mut self, reference: &ast::TypeRef) -> Type {\n        match reference.name.text.as_str() {\n            \"Int\" => Type::Int,\n            \"Bool\" => Type::Bool,\n            \"Unit\" => Type::Unit,\n            unknown => {\n                if let Some(symbol) = self.types.get(unknown).copied() {\n                    return match symbol.definition {\n                        TypeDefinition::Record(id) => Type::Record(RecordType {\n                            id,\n                            name: unknown.to_owned(),\n                        }),\n                        TypeDefinition::Enum(id) => Type::Enum(EnumType {\n                            id,\n                            name: unknown.to_owned(),\n                        }),\n                    };\n                }\n                self.diagnostics.push(\n                    Diagnostic::error(\"N3001\", \"unknown type\")\n                        .with_primary(reference.span, format!(\"unknown type `{unknown}`\"))\n                        .with_note(\n                            \"the bootstrap semantic core recognizes Int, Bool, Unit, and declared record or enum names\",\n                        ),\n                );\n                Type::Error\n            }\n        }\n    }\n''' + "'''" + ''',
    ''' + "'''" + '''    fn resolve_type_ref(&mut self, reference: &ast::TypeRef) -> Type {\n        match &reference.kind {\n            ast::TypeRefKind::Named(name) => match name.text.as_str() {\n                \"Int\" => Type::Int,\n                \"Bool\" => Type::Bool,\n                \"Unit\" => Type::Unit,\n                unknown => {\n                    if let Some(symbol) = self.types.get(unknown).copied() {\n                        return match symbol.definition {\n                            TypeDefinition::Record(id) => Type::Record(RecordType {\n                                id,\n                                name: unknown.to_owned(),\n                            }),\n                            TypeDefinition::Enum(id) => Type::Enum(EnumType {\n                                id,\n                                name: unknown.to_owned(),\n                            }),\n                        };\n                    }\n                    self.diagnostics.push(\n                        Diagnostic::error(\"N3001\", \"unknown type\")\n                            .with_primary(reference.span, format!(\"unknown type `{unknown}`\"))\n                            .with_note(\n                                \"the bootstrap semantic core recognizes Int, Bool, Unit, declared record or enum names, and explicit function types\",\n                            ),\n                    );\n                    Type::Error\n                }\n            },\n            ast::TypeRefKind::Function {\n                parameters,\n                return_type,\n            } => Type::Function(FunctionType {\n                parameters: parameters\n                    .iter()\n                    .map(|parameter| self.resolve_type_ref(parameter))\n                    .collect(),\n                return_type: Box::new(self.resolve_type_ref(return_type)),\n            }),\n        }\n    }\n''' + "'''" + ''',
)

'''
text = text[:start] + replacement + text[end:]
needle = '# Parser integration tests for recursive syntax and fail-closed depth.\nPath("crates/nova-parser/tests/function_types.rs").write_text'
if needle not in text:
    raise SystemExit('parser integration test anchor not found')
text = text.replace(
    needle,
    '# Parser integration tests for recursive syntax and fail-closed depth.\nPath("crates/nova-parser/tests").mkdir(parents=True, exist_ok=True)\nPath("crates/nova-parser/tests/function_types.rs").write_text',
    1,
)
p.write_text(text)
