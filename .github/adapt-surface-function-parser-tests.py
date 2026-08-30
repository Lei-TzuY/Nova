from pathlib import Path

p = Path("crates/nova-parser/src/parser.rs")
text = p.read_text()
old = '''    use super::parse;\n    use crate::ast::{BinaryOperator, ExpressionKind, StatementKind};\n'''
new = '''    use super::parse;\n    use crate::ast::{BinaryOperator, ExpressionKind, StatementKind, TypeRef, TypeRefKind};\n'''
if old not in text:
    raise SystemExit("parser test import anchor not found")
text = text.replace(old, new, 1)
old = '''    fn parse_text(text: &str) -> (SourceFile, super::ParseOutput) {\n        let source = SourceFile::new(SourceId::new(0), "test.nv", text);\n        let lexed = lex(&source);\n        assert!(\n            lexed.diagnostics.is_empty(),\n            "parser test source must lex successfully: {:?}",\n            lexed.diagnostics\n        );\n        let parsed = parse(&source, &lexed.tokens);\n        (source, parsed)\n    }\n'''
new = old + '''\n    fn named_type_text(reference: &TypeRef) -> &str {\n        let TypeRefKind::Named(name) = &reference.kind else {\n            panic!("expected a named type reference, got {:?}", reference.kind);\n        };\n        &name.text\n    }\n'''
if old not in text:
    raise SystemExit("parser test helper anchor not found")
text = text.replace(old, new, 1)
replacements = {
    'assert_eq!(function.return_type.name.text, "Int");':
        'assert_eq!(named_type_text(&function.return_type), "Int");',
    '.map(|payload| payload.name.text.as_str()),':
        '.map(named_type_text),',
    'assert_eq!(parsed.program.functions[0].return_type.name.text, "Unit");':
        'assert_eq!(named_type_text(&parsed.program.functions[0].return_type), "Unit");',
    'if name.text == "value" && annotation.name.text == "Int"':
        'if name.text == "value" && named_type_text(annotation) == "Int"',
}
for old_text, new_text in replacements.items():
    if old_text not in text:
        raise SystemExit(f"stale parser assertion anchor not found: {old_text}")
    text = text.replace(old_text, new_text, 1)
p.write_text(text)
