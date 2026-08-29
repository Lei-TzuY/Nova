//! Shared bootstrap signed-64 `Int` arithmetic semantics for Nova.
//!
//! This leaf crate owns the executable arithmetic truth table used by both semantic
//! constant-failure preflight and the bootstrap interpreter. It deliberately knows
//! nothing about HIR, diagnostics, source spans, or runtime frames.

/// Failure classes produced by bootstrap integer arithmetic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntArithmeticError {
    /// The exact mathematical result is outside signed 64-bit `Int`.
    Overflow,
    /// Division or remainder used a zero divisor.
    ZeroDivisor,
}

/// Checked signed negation.
pub fn negate(value: i64) -> Result<i64, IntArithmeticError> {
    value.checked_neg().ok_or(IntArithmeticError::Overflow)
}

/// Checked signed addition.
pub fn add(left: i64, right: i64) -> Result<i64, IntArithmeticError> {
    left.checked_add(right).ok_or(IntArithmeticError::Overflow)
}

/// Checked signed subtraction.
pub fn subtract(left: i64, right: i64) -> Result<i64, IntArithmeticError> {
    left.checked_sub(right).ok_or(IntArithmeticError::Overflow)
}

/// Checked signed multiplication.
pub fn multiply(left: i64, right: i64) -> Result<i64, IntArithmeticError> {
    left.checked_mul(right).ok_or(IntArithmeticError::Overflow)
}

/// Signed division whose quotient truncates toward zero.
///
/// `i64::MIN / -1` is overflow because its exact quotient is `2^63`.
pub fn divide(left: i64, right: i64) -> Result<i64, IntArithmeticError> {
    classify_divisor(left, right)?;
    Ok(left / right)
}

/// Signed remainder associated with truncation-toward-zero division.
///
/// A non-zero remainder has the dividend's sign. `i64::MIN % -1` deliberately
/// shares division's overflow edge so semantic preflight and execution agree.
pub fn remainder(left: i64, right: i64) -> Result<i64, IntArithmeticError> {
    classify_divisor(left, right)?;
    Ok(left % right)
}

fn classify_divisor(left: i64, right: i64) -> Result<(), IntArithmeticError> {
    if right == 0 {
        return Err(IntArithmeticError::ZeroDivisor);
    }
    if left == i64::MIN && right == -1 {
        return Err(IntArithmeticError::Overflow);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{IntArithmeticError, add, divide, multiply, negate, remainder, subtract};

    #[test]
    fn division_truncates_toward_zero_for_every_sign_pair() {
        for (left, right, expected) in [(7, 3, 2), (-7, 3, -2), (7, -3, -2), (-7, -3, 2)] {
            assert_eq!(divide(left, right), Ok(expected));
        }
    }

    #[test]
    fn remainder_follows_the_dividend_sign_and_division_identity() {
        for (left, right, expected) in [(7, 3, 1), (-7, 3, -1), (7, -3, 1), (-7, -3, -1)] {
            let quotient = divide(left, right).expect("sample quotient is representable");
            let actual = remainder(left, right).expect("sample remainder is representable");
            assert_eq!(actual, expected);
            assert_eq!(
                i128::from(left),
                i128::from(quotient) * i128::from(right) + i128::from(actual)
            );
            assert!(actual == 0 || actual.signum() == left.signum());
            assert!(i128::from(actual).abs() < i128::from(right).abs());
        }
    }

    #[test]
    fn division_edges_have_explicit_failure_classes() {
        for left in [i64::MIN, -1, 0, 1, i64::MAX] {
            assert_eq!(divide(left, 0), Err(IntArithmeticError::ZeroDivisor));
            assert_eq!(remainder(left, 0), Err(IntArithmeticError::ZeroDivisor));
        }

        assert_eq!(divide(i64::MIN, -1), Err(IntArithmeticError::Overflow));
        assert_eq!(remainder(i64::MIN, -1), Err(IntArithmeticError::Overflow));
        assert_eq!(divide(i64::MIN, 1), Ok(i64::MIN));
        assert_eq!(remainder(i64::MIN, 1), Ok(0));
    }

    #[test]
    fn checked_non_division_arithmetic_uses_the_same_overflow_class() {
        assert_eq!(negate(i64::MIN), Err(IntArithmeticError::Overflow));
        assert_eq!(add(i64::MAX, 1), Err(IntArithmeticError::Overflow));
        assert_eq!(subtract(i64::MIN, 1), Err(IntArithmeticError::Overflow));
        assert_eq!(multiply(i64::MAX, 2), Err(IntArithmeticError::Overflow));

        assert_eq!(negate(7), Ok(-7));
        assert_eq!(add(20, 22), Ok(42));
        assert_eq!(subtract(50, 8), Ok(42));
        assert_eq!(multiply(6, 7), Ok(42));
    }
}
