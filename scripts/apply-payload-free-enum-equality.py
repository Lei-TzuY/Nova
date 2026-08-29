from pathlib import Path

# Semantic comparability.
path = Path("crates/nova-sema/src/analyzer.rs")
text = path.read_text()
text = text.replace(
    '            if other.is_never() || matches!(other, Type::Int | Type::Bool | Type::Unit) {\n',
    '            if other.is_never() || self.is_equality_comparable(other) {\n',
    1,
)
text = text.replace(
    '                        "equality requires Int, Bool, or Unit operands, found {} and {}",\n',
    '                        "equality requires Int, Bool, Unit, or payload-free enum operands, found {} and {}",\n',
    1,
)
text = text.replace(
    '        let comparable = matches!(left.ty, Type::Int | Type::Bool | Type::Unit);\n',
    '        let comparable = self.is_equality_comparable(&left.ty);\n',
    1,
)
text = text.replace(
    '                        "equality requires matching Int, Bool, or Unit operands, found {} and {}",\n',
    '                        "equality requires matching Int, Bool, Unit, or payload-free enum operands, found {} and {}",\n',
    1,
)
anchor = '''    fn check_call(
'''
helper = '''    fn is_equality_comparable(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Bool | Type::Unit => true,
            Type::Enum(enumeration) => self
                .enum_definitions
                .get(enumeration.id.index())
                .is_some_and(|definition| {
                    definition.id == enumeration.id
                        && definition
                            .variants
                            .iter()
                            .all(|variant| variant.payload.is_none())
                }),
            _ => false,
        }
    }

    fn check_call(
'''
if text.count(anchor) != 1:
    raise SystemExit("expected one check_call anchor")
if 'fn is_equality_comparable' in text:
    raise SystemExit("equality comparability helper already exists")
text = text.replace(anchor, helper, 1)
path.write_text(text)

# Closed-condition proof for direct payload-free constructors. Type checking is the
# authority that restricts Bool-typed enum equality to payload-free enum declarations.
path = Path("crates/nova-sema/src/constant_condition.rs")
text = path.read_text()
old = '''                (Type::Unit, Type::Unit) => {
                    unit_value(left)?;
                    unit_value(right)?;
                    true
                }
                _ => return None,
'''
new = '''                (Type::Unit, Type::Unit) => {
                    unit_value(left)?;
                    unit_value(right)?;
                    true
                }
                (Type::Enum(left_enum), Type::Enum(right_enum))
                    if left_enum.id == right_enum.id =>
                {
                    enum_tag(left)? == enum_tag(right)?
                }
                _ => return None,
'''
if text.count(old) != 1:
    raise SystemExit("constant-condition equality anchor not found exactly once")
text = text.replace(old, new)
text += '''
fn enum_tag(expression: &Expression) -> Option<(crate::hir::EnumId, usize)> {
    match &expression.kind {
        ExpressionKind::EnumConstructor {
            enumeration,
            variant_index,
            payload,
        } if payload.is_none() => Some((*enumeration, *variant_index)),
        _ => None,
    }
}
'''
path.write_text(text)

# Runtime equality for semantically accepted payload-free enum values.
path = Path("crates/nova-interpreter/src/lib.rs")
text = path.read_text()
anchor = '''            (BinaryOperator::Equal, Value::Unit, Value::Unit) => Ok(Value::Bool(true)),
            (BinaryOperator::NotEqual, Value::Unit, Value::Unit) => Ok(Value::Bool(false)),
'''
addition = anchor + '''            (
                BinaryOperator::Equal,
                Value::Enum {
                    enumeration: left_enum,
                    variant_index: left_variant,
                    payload: None,
                },
                Value::Enum {
                    enumeration: right_enum,
                    variant_index: right_variant,
                    payload: None,
                },
            ) if left_enum == right_enum => Ok(Value::Bool(left_variant == right_variant)),
            (
                BinaryOperator::NotEqual,
                Value::Enum {
                    enumeration: left_enum,
                    variant_index: left_variant,
                    payload: None,
                },
                Value::Enum {
                    enumeration: right_enum,
                    variant_index: right_variant,
                    payload: None,
                },
            ) if left_enum == right_enum => Ok(Value::Bool(left_variant != right_variant)),
'''
if text.count(anchor) != 1:
    raise SystemExit("interpreter Unit equality anchor not found exactly once")
text = text.replace(anchor, addition)
path.write_text(text)
