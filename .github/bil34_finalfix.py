from pathlib import Path

p = Path('crates/nova-interpreter/src/lib.rs')
text = p.read_text()
old = '''        // A generic function symbol is a valid runtime value before call-site
        // instantiation. Concrete `T` validation begins inside `call_function`.
        if let (Value::Function(id), Type::Function(expected)) = (value, ty) {
            if id.module() == self.program.module.id {
                if let Some(function) = self.program.functions.get(id.index()) {
                    let signature_matches = function.id == *id
                        && function.parameters.len() == expected.parameters.len()
                        && function
                            .parameters
                            .iter()
                            .zip(&expected.parameters)
                            .all(|(parameter, expected_type)| &parameter.ty == expected_type)
                        && &function.return_type == expected.return_type.as_ref();
                    if signature_matches {
                        return true;
                    }
                }
            }
        }
'''
new = '''        // A generic function symbol is a valid runtime value before call-site
        // instantiation. Keep this exception narrow: ordinary functions must still
        // traverse runtime-valid nominal types so forged name/identity drift fails closed.
        fn contains_type_parameter(ty: &Type) -> bool {
            match ty {
                Type::TypeParameter(_) => true,
                Type::Function(signature) => {
                    signature.parameters.iter().any(contains_type_parameter)
                        || contains_type_parameter(&signature.return_type)
                }
                _ => false,
            }
        }
        if let (Value::Function(id), Type::Function(expected)) = (value, ty) {
            let is_generic = expected.parameters.iter().any(contains_type_parameter)
                || contains_type_parameter(&expected.return_type);
            if is_generic && id.module() == self.program.module.id {
                if let Some(function) = self.program.functions.get(id.index()) {
                    let signature_matches = function.id == *id
                        && function.parameters.len() == expected.parameters.len()
                        && function
                            .parameters
                            .iter()
                            .zip(&expected.parameters)
                            .all(|(parameter, expected_type)| &parameter.ty == expected_type)
                        && &function.return_type == expected.return_type.as_ref();
                    if signature_matches {
                        return true;
                    }
                }
            }
        }
'''
assert old in text, 'generic function runtime shortcut shape changed'
p.write_text(text.replace(old, new, 1))

for helper in [
    Path('.github/workflows/conquest-bil34-finalfix.yml'),
    Path('.github/bil34-final-trigger'),
    Path('.github/bil34-test-error.txt'),
    Path('.github/bil34_finalfix.py'),
]:
    if helper.exists():
        helper.unlink()
