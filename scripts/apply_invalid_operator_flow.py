from pathlib import Path


def patch(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}: {old[:120]!r}")
    file.write_text(text.replace(old, new, 1))


patch(
    "crates/nova-sema/src/analyzer.rs",
    '''            ast::ExpressionKind::Unary { operator, operand } => {
                let operand = self.lower_expression(operand, return_type);
                let ty = self.check_unary(*operator, &operand, expression.span);
                (
                    ExpressionKind::Unary {
                        operator: *operator,
                        operand: Box::new(operand),
                    },
                    ty,
                )
            }
''',
    '''            ast::ExpressionKind::Unary { operator, operand } => {
                let operator_entry_state = self.capture_reachable_state();
                let operand = self.lower_expression(operand, return_type);
                let ty = self.check_unary(*operator, &operand, expression.span);
                if ty.is_error() {
                    self.restore_reachable_state(operator_entry_state);
                }
                (
                    ExpressionKind::Unary {
                        operator: *operator,
                        operand: Box::new(operand),
                    },
                    ty,
                )
            }
''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''            } => {
                let left = self.lower_expression(left, return_type);
                let left_scopes = self.scopes.clone();
''',
    '''            } => {
                let operator_entry_state = self.capture_reachable_state();
                let left = self.lower_expression(left, return_type);
                let left_scopes = self.scopes.clone();
''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''                let ty = self.check_binary(*operator, &left, &right, expression.span);
                (
                    ExpressionKind::Binary {
''',
    '''                let ty = self.check_binary(*operator, &left, &right, expression.span);
                if ty.is_error() {
                    self.restore_reachable_state(operator_entry_state);
                }
                (
                    ExpressionKind::Binary {
''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''                self.require_binary_operands(left, right, &Type::Int, span, "arithmetic operator");
                strict_binary_result_type(&left.ty, &right.ty, Type::Int)
''',
    '''                self.require_binary_operands(left, right, &Type::Int, span, "arithmetic operator");
                strict_binary_result_type(&left.ty, &right.ty, &Type::Int, Type::Int)
''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''                self.require_binary_operands(left, right, &Type::Int, span, "comparison operator");
                strict_binary_result_type(&left.ty, &right.ty, Type::Bool)
''',
    '''                self.require_binary_operands(left, right, &Type::Int, span, "comparison operator");
                strict_binary_result_type(&left.ty, &right.ty, &Type::Int, Type::Bool)
''',
)

patch(
    "crates/nova-sema/src/analyzer.rs",
    '''        if left.ty.is_error() || right.ty.is_error() {
            return Type::Error;
        }
        let left_literal = match &left.kind {
''',
    '''        if left.ty.is_error() || right.ty.is_error() {
            return Type::Error;
        }
        if !expected_type_compatible(&left.ty, &Type::Bool)
            || !expected_type_compatible(&right.ty, &Type::Bool)
        {
            return Type::Error;
        }
        let left_literal = match &left.kind {
''',
)

patch(
    "crates/nova-sema/src/type_rules.rs",
    '''pub(crate) fn strict_binary_result_type(left: &Type, right: &Type, success: Type) -> Type {
    if left.is_never() || right.is_never() {
        Type::Never
    } else if left.is_error() || right.is_error() {
        Type::Error
    } else {
        success
    }
}
''',
    '''pub(crate) fn strict_binary_result_type(
    left: &Type,
    right: &Type,
    expected: &Type,
    success: Type,
) -> Type {
    if left.is_never() || right.is_never() {
        Type::Never
    } else if left.is_error() || right.is_error() || left != expected || right != expected {
        Type::Error
    } else {
        success
    }
}
''',
)

patch(
    "crates/nova-sema/src/type_rules.rs",
    '''            strict_binary_result_type(&Type::Int, &Type::Int, Type::Bool),
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
''',
    '''            strict_binary_result_type(&Type::Int, &Type::Int, &Type::Int, Type::Bool),
            Type::Bool
        );
        assert_eq!(
            strict_binary_result_type(&Type::Bool, &Type::Int, &Type::Int, Type::Bool),
            Type::Error
        );
        assert_eq!(
            strict_binary_result_type(&Type::Error, &Type::Int, &Type::Int, Type::Bool),
            Type::Error
        );
        assert_eq!(
            strict_binary_result_type(&Type::Error, &Type::Never, &Type::Int, Type::Bool),
            Type::Never
        );
''',
)

patch(
    "README.md",
    '''Rejected field access follows the same recovery discipline. The base is evaluated
first, but a continuing non-record base or unknown field yields Error HIR and cannot
export flow facts produced only inside the rejected access. A base that is already
non-continuing keeps `!` without a secondary field-type cascade.

''',
    '''Rejected field access follows the same recovery discipline. The base is evaluated
first, but a continuing non-record base or unknown field yields Error HIR and cannot
export flow facts produced only inside the rejected access. A base that is already
non-continuing keeps `!` without a secondary field-type cascade.

Rejected unary and binary operators are also fail-closed. Concrete operand type
mismatches produce Error HIR rather than a nominal success type, and flow facts from
a continuing rejected operator are rolled back. Non-continuation from an operand that
must be evaluated keeps `!` precedence; short-circuit operators retain their existing
conditional right-hand evaluation rules.

''',
)

patch(
    "docs/language-constitution.md",
    '''A continuing field access rejected because its base is not a record or the named field
does not exist is likewise fail-closed for flow recovery. The base is still lowered
for deterministic diagnostics, but its assignments and loop-exit facts do not become
post-access facts. A base expression that is already `!` remains non-continuing and
does not acquire a secondary record-type diagnostic.

''',
    '''A continuing field access rejected because its base is not a record or the named field
does not exist is likewise fail-closed for flow recovery. The base is still lowered
for deterministic diagnostics, but its assignments and loop-exit facts do not become
post-access facts. A base expression that is already `!` remains non-continuing and
does not acquire a secondary record-type diagnostic.

A continuing unary or binary operator rejected by operand typing is fail-closed for
flow recovery as well. Concrete mismatches yield `<error>` rather than retaining the
operator's nominal result type, and assignments or loop-exit facts created only while
lowering that rejected operator are discarded. A definitely evaluated operand that
is already `!` retains non-continuation precedence; `&&` and `||` continue to model
conditional right-hand evaluation rather than treating every lowered RHS as reachable.

''',
)

roadmap = "docs/roadmap.md"
patch(
    roadmap,
    "**Status: eighteen vertical slices implemented; broader type-system work remains.**\n",
    "**Status: nineteen vertical slices implemented; broader type-system work remains.**\n",
)
marker = '''The next Phase 2 slices should address semantic depth rather than widen syntax
prematurely. In particular:
'''
addition = '''Implemented in the nineteenth Phase 2 slice:

- unary and binary lowering snapshot reachable state so a continuing rejected
  operator cannot export assignments or loop-exit facts from its operands;
- strict arithmetic and comparison result typing now validates concrete operand
  types in the pure `type_rules` contract instead of returning a nominal success
  type after N3004;
- boolean `&&`/`||` likewise become Error-typed on concrete Bool mismatches while
  retaining existing short-circuit reachability for valid Bool operands;
- definitely evaluated `!` operands keep non-continuation precedence over recovery
  errors, while optional short-circuit RHS non-continuation remains path-sensitive;
- equality already produced Error on concrete mismatches and now gains the same
  flow rollback at the shared binary lowering boundary; and
- red-to-green adversarial tests lock typing, definite-initialization, loop-exit,
  strict non-continuation, and short-circuit behavior.

'''
patch(roadmap, marker, addition + marker)
