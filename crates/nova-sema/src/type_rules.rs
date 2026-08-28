use crate::hir::Type;

/// Recovery-aware compatibility used whenever a value is checked against an expected type.
///
/// `Never` is compatible with every expected type because the value is never produced, while
/// `Error` suppresses cascaded diagnostics after an earlier type failure.
pub(crate) fn expected_type_compatible(actual: &Type, expected: &Type) -> bool {
    actual.is_error() || expected.is_error() || actual.is_never() || actual == expected
}

/// Computes the result of a strict left-to-right binary expression after operand checking.
///
/// A non-continuing operand dominates recovery errors because runtime control cannot reach a
/// result value. Otherwise an error-recovery operand makes the expression erroneous.
pub(crate) fn strict_binary_result_type(left: &Type, right: &Type, success: Type) -> Type {
    if left.is_never() || right.is_never() {
        Type::Never
    } else if left.is_error() || right.is_error() {
        Type::Error
    } else {
        success
    }
}

/// One observation made while joining alternative control-flow result types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum JoinObservation {
    /// A non-continuing path contributes no produced value type.
    Never,
    /// An already-diagnosed path contributes no concrete anchor type.
    Error,
    /// The first concrete continuing path establishes the join anchor.
    Anchor(Type),
    /// A concrete continuing path agrees with the existing anchor.
    Compatible,
    /// A concrete continuing path disagrees with the existing anchor.
    Mismatch { expected: Type, found: Type },
}

/// Recovery-aware join state shared by `if` and exhaustive `match` typing.
///
/// The first concrete continuing type is the anchor. `Never` is ignored, `Error` is neutral when
/// another concrete type exists, and any concrete mismatch makes the final join erroneous.
#[derive(Clone, Debug, Default)]
pub(crate) struct TypeJoin {
    anchor: Option<Type>,
    saw_error: bool,
    mismatch: bool,
}

impl TypeJoin {
    /// Observes one alternative path type without emitting diagnostics.
    pub(crate) fn observe(&mut self, ty: &Type) -> JoinObservation {
        if ty.is_never() {
            return JoinObservation::Never;
        }
        if ty.is_error() {
            self.saw_error = true;
            return JoinObservation::Error;
        }
        if let Some(anchor) = &self.anchor {
            if anchor == ty {
                JoinObservation::Compatible
            } else {
                self.mismatch = true;
                JoinObservation::Mismatch {
                    expected: anchor.clone(),
                    found: ty.clone(),
                }
            }
        } else {
            self.anchor = Some(ty.clone());
            JoinObservation::Anchor(ty.clone())
        }
    }

    /// Finishes the join after all alternative paths have been observed.
    pub(crate) fn finish(self) -> Type {
        if self.mismatch || (self.anchor.is_none() && self.saw_error) {
            Type::Error
        } else {
            self.anchor.unwrap_or(Type::Never)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{JoinObservation, TypeJoin, expected_type_compatible, strict_binary_result_type};
    use crate::hir::{RecordId, RecordType, Type};

    fn record(index: usize, name: &str) -> Type {
        Type::Record(RecordType {
            id: RecordId::new(index),
            name: name.to_owned(),
        })
    }

    #[test]
    fn expected_type_compatibility_is_recovery_aware_and_nominal() {
        assert!(expected_type_compatible(&Type::Int, &Type::Int));
        assert!(expected_type_compatible(&Type::Unit, &Type::Unit));
        assert!(expected_type_compatible(&Type::Never, &Type::Bool));
        assert!(expected_type_compatible(&Type::Error, &Type::Unit));
        assert!(expected_type_compatible(&Type::Int, &Type::Error));
        assert!(!expected_type_compatible(&Type::Int, &Type::Bool));
        assert!(!expected_type_compatible(&Type::Unit, &Type::Int));
        assert!(expected_type_compatible(&record(0, "A"), &record(0, "A")));
        assert!(!expected_type_compatible(&record(0, "A"), &record(1, "B")));
    }

    #[test]
    fn strict_binary_result_gives_noncontinuation_precedence_over_recovery_error() {
        assert_eq!(
            strict_binary_result_type(&Type::Int, &Type::Int, Type::Bool),
            Type::Bool
        );
        assert_eq!(
            strict_binary_result_type(&Type::Error, &Type::Int, Type::Bool),
            Type::Error
        );
        assert_eq!(
            strict_binary_result_type(&Type::Error, &Type::Never, Type::Bool),
            Type::Never
        );
    }

    #[test]
    fn type_join_treats_never_as_bottom_and_error_as_recovery_neutral() {
        let mut join = TypeJoin::default();
        assert_eq!(join.observe(&Type::Never), JoinObservation::Never);
        assert_eq!(join.observe(&Type::Error), JoinObservation::Error);
        assert_eq!(
            join.observe(&Type::Unit),
            JoinObservation::Anchor(Type::Unit)
        );
        assert_eq!(join.observe(&Type::Unit), JoinObservation::Compatible);
        assert_eq!(join.finish(), Type::Unit);

        let mut all_never = TypeJoin::default();
        all_never.observe(&Type::Never);
        all_never.observe(&Type::Never);
        assert_eq!(all_never.finish(), Type::Never);

        let mut only_error = TypeJoin::default();
        only_error.observe(&Type::Never);
        only_error.observe(&Type::Error);
        assert_eq!(only_error.finish(), Type::Error);
    }

    #[test]
    fn type_join_keeps_first_concrete_anchor_after_mismatches() {
        let mut join = TypeJoin::default();
        assert_eq!(join.observe(&Type::Int), JoinObservation::Anchor(Type::Int));
        assert_eq!(
            join.observe(&Type::Bool),
            JoinObservation::Mismatch {
                expected: Type::Int,
                found: Type::Bool,
            }
        );
        assert_eq!(
            join.observe(&Type::Unit),
            JoinObservation::Mismatch {
                expected: Type::Int,
                found: Type::Unit,
            }
        );
        assert_eq!(join.finish(), Type::Error);
    }
}
