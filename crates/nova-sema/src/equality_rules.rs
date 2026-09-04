//! Shared bootstrap equality admissibility rules over resolved HIR types.

use crate::hir::{EnumId, Type};

/// Reports whether one resolved type participates in bootstrap equality.
///
/// Primitive values, `Unit`, and function values are intrinsically comparable.
/// Enum comparability depends on declaration metadata, so callers provide the
/// payload-free predicate for the enum identity. Records, `Never`, and recovery
/// `Error` are not ordinary comparable value types.
#[must_use]
pub fn is_equality_comparable(
    ty: &Type,
    mut enum_is_payload_free: impl FnMut(EnumId) -> bool,
) -> bool {
    match ty {
        Type::Int | Type::UInt | Type::Bool | Type::String | Type::Unit | Type::Function(_) => true,
        Type::Enum(enumeration) => enum_is_payload_free(enumeration.id),
        Type::Record(_) | Type::TypeParameter(_) | Type::Never | Type::Error => false,
    }
}

/// Reports whether two ordinary resolved operand types may be compared with
/// `==` or `!=` under the bootstrap equality contract.
///
/// The operands must be exactly the same resolved type. Payload-free enum
/// eligibility is supplied by the caller because it depends on the owning
/// program's declaration table.
#[must_use]
pub fn matching_equality_types(
    left: &Type,
    right: &Type,
    enum_is_payload_free: impl FnMut(EnumId) -> bool,
) -> bool {
    left == right && is_equality_comparable(left, enum_is_payload_free)
}

#[cfg(test)]
mod tests {
    use super::{is_equality_comparable, matching_equality_types};
    use crate::hir::{EnumId, EnumType, FunctionType, RecordId, RecordType, Type};

    fn function(return_type: Type) -> Type {
        Type::Function(FunctionType {
            parameters: vec![Type::Int],
            return_type: Box::new(return_type),
        })
    }

    fn enumeration(index: usize, name: &str) -> Type {
        Type::Enum(EnumType {
            id: EnumId::new(index),
            name: name.to_owned(),
        })
    }

    #[test]
    fn intrinsic_comparability_is_explicit_and_recovery_types_are_excluded() {
        let no_enum = |_| false;
        assert!(is_equality_comparable(&Type::Int, no_enum));
        assert!(is_equality_comparable(&Type::Bool, no_enum));
        assert!(is_equality_comparable(&Type::String, no_enum));
        assert!(is_equality_comparable(&Type::Unit, no_enum));
        assert!(is_equality_comparable(&function(Type::Bool), no_enum));
        assert!(!is_equality_comparable(
            &Type::Record(RecordType {
                id: RecordId::new(0),
                name: "Pair".to_owned(),
            }),
            no_enum,
        ));
        assert!(!is_equality_comparable(&Type::Never, no_enum));
        assert!(!is_equality_comparable(&Type::Error, no_enum));
    }

    #[test]
    fn enum_comparability_is_declaration_context_dependent() {
        let first = enumeration(0, "Color");
        let second = enumeration(1, "Maybe");
        assert!(is_equality_comparable(&first, |id| id == EnumId::new(0)));
        assert!(!is_equality_comparable(&second, |id| id == EnumId::new(0)));
        assert!(matching_equality_types(&first, &first, |id| id == EnumId::new(0)));
        assert!(!matching_equality_types(&first, &second, |_| true));
    }

    #[test]
    fn function_equality_requires_the_same_fully_resolved_signature() {
        let integer = function(Type::Int);
        let boolean = function(Type::Bool);
        assert!(matching_equality_types(&integer, &integer, |_| false));
        assert!(!matching_equality_types(&integer, &boolean, |_| false));
    }
}
