from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if text.count(old) != 1:
        raise SystemExit(f"{label}: expected exactly one anchor, found {text.count(old)}")
    return text.replace(old, new, 1)


# Reuse the shared equality type classifier from semantic analysis.
path = Path("crates/nova-sema/src/analyzer.rs")
text = path.read_text()
text = replace_once(
    text,
    "use crate::constant_int::{self, ConstantIntError};\n",
    "use crate::constant_int::{self, ConstantIntError};\n"
    "use crate::equality_rules::is_equality_comparable as type_is_equality_comparable;\n",
    "analyzer import",
)
pattern = re.compile(
    r"    fn is_equality_comparable\(&self, ty: &Type\) -> bool \{\n"
    r"        match ty \{\n"
    r"            Type::Int \| Type::Bool \| Type::Unit \| Type::Function\(_\) => true,\n"
    r"            Type::Enum\(enumeration\) => self\n"
    r"                \.enum_definitions\n"
    r"                \.get\(enumeration\.id\.index\(\)\)\n"
    r"                \.is_some_and\(\|definition\| \{\n"
    r"                    definition\.id == enumeration\.id\n"
    r"                        && definition\n"
    r"                            \.variants\n"
    r"                            \.iter\(\)\n"
    r"                            \.all\(\|variant\| variant\.payload\.is_none\(\)\)\n"
    r"                \}\),\n"
    r"            _ => false,\n"
    r"        \}\n"
    r"    \}\n"
)
replacement = '''    fn is_equality_comparable(&self, ty: &Type) -> bool {
        type_is_equality_comparable(ty, |enum_id| {
            self.enum_definitions
                .get(enum_id.index())
                .is_some_and(|definition| {
                    definition.id == enum_id
                        && definition
                            .variants
                            .iter()
                            .all(|variant| variant.payload.is_none())
                })
        })
    }
'''
text, count = pattern.subn(replacement, text, count=1)
if count != 1:
    raise SystemExit(f"analyzer equality helper: expected one match, found {count}")
path.write_text(text)

# Validate equality operator admissibility against typed HIR before evaluating operands.
path = Path("crates/nova-interpreter/src/lib.rs")
text = path.read_text()
text = replace_once(
    text,
    "use nova_parser::ast::{BinaryOperator, UnaryOperator};\nuse nova_sema::hir::{\n",
    "use nova_parser::ast::{BinaryOperator, UnaryOperator};\n"
    "use nova_sema::equality_rules::matching_equality_types;\n"
    "use nova_sema::hir::{\n",
    "interpreter equality import",
)
needle = '''    ) -> Result<Flow, Diagnostic> {
        let left = match self.eval_expression(left, frame)? {
'''
replacement = '''    ) -> Result<Flow, Diagnostic> {
        if matches!(operator, BinaryOperator::Equal | BinaryOperator::NotEqual)
            && !self.equality_types_match(&left.ty, &right.ty)
        {
            return Err(self.invariant(
                expression.span,
                format!(
                    "equality operator received HIR operand types outside the semantic equality contract: {} and {}",
                    left.ty, right.ty
                ),
            ));
        }

        let left = match self.eval_expression(left, frame)? {
'''
text = replace_once(text, needle, replacement, "eval_binary precondition")
needle = '''    fn function_signatures_match(&self, left: FunctionId, right: FunctionId) -> bool {
'''
replacement = '''    fn equality_types_match(&self, left: &Type, right: &Type) -> bool {
        matching_equality_types(left, right, |enum_id| {
            self.program
                .enums
                .get(enum_id.index())
                .is_some_and(|definition| {
                    definition.id == enum_id
                        && definition
                            .variants
                            .iter()
                            .all(|variant| variant.payload.is_none())
                })
        })
    }

    fn function_signatures_match(&self, left: FunctionId, right: FunctionId) -> bool {
'''
text = replace_once(text, needle, replacement, "runtime equality helper")
path.write_text(text)

# Correct stale current-status prose and document the runtime operator boundary.
path = Path("README.md")
text = path.read_text()
old = '''Matching `Int`, `Bool`, and `Unit` values support `==` and `!=`. `Unit` has a
single runtime value, so Unit equality is always true and Unit inequality is always false
once both operands have evaluated normally. A nominal enum also supports equality when
every declared variant is payload-free; operands must have the same enum identity and
comparison uses the resolved variant slot. Enums with any payload variant, records, and
functions remain non-comparable. Closed-condition analysis can prove literal Unit and
direct payload-free enum-constructor comparisons, while locals and calls remain dynamic
and are still evaluated at runtime.
'''
new = '''Matching `Int`, `Bool`, and `Unit` values support `==` and `!=`. `Unit` has a
single runtime value, so Unit equality is always true and Unit inequality is always false
once both operands have evaluated normally. A nominal enum also supports equality when
every declared variant is payload-free; operands must have the same enum identity and
comparison uses the resolved variant slot. Function values are comparable only at the
same fully resolved signature and compare top-level declaration identity. Enums with any
payload variant and records remain non-comparable. Closed-condition analysis can prove
literal Unit, direct payload-free enum-constructor, and direct function-reference
comparisons, while locals and calls remain dynamic and are still evaluated at runtime.
'''
text = replace_once(text, old, new, "README equality contract")
old = '''Every expression that completes with an ordinary runtime value also has a final
interpreter postcondition: the value must recursively conform to that expression's
typed-HIR result type. This closes gaps for local or discarded literals, projections,
operators, blocks, conditionals, matches, and other values that may never cross a
function, aggregate, or frame-storage boundary. Structured `return`, `break`, and
`continue` propagation is not a runtime value and therefore remains outside this
postcondition. A mismatched value fails closed with `N4005` at the expression span.
'''
new = '''Every expression that completes with an ordinary runtime value also has a final
interpreter postcondition: the value must recursively conform to that expression's
typed-HIR result type. This closes gaps for local or discarded literals, projections,
operators, blocks, conditionals, matches, and other values that may never cross a
function, aggregate, or frame-storage boundary. Equality adds an operator-level
precondition as well: the two resolved operand types must satisfy the same shared
semantic comparability rule, including the declaration-wide payload-free requirement
for enums. Malformed HIR therefore cannot compare a payload-free variant of an enum
whose other variants carry payloads. Structured `return`, `break`, and `continue`
propagation is not a runtime value and therefore remains outside the value postcondition.
Any such interpreter/HIR contract drift fails closed with `N4005`.
'''
text = replace_once(text, old, new, "README runtime equality boundary")
path.write_text(text)

# Record the semantic and executable slices.
path = Path("docs/roadmap.md")
text = path.read_text()
text = replace_once(
    text,
    "**Status: forty vertical slices implemented; broader type-system work remains.**",
    "**Status: forty-one vertical slices implemented; broader type-system work remains.**",
    "Phase 2 count",
)
anchor = '''Implemented in the fortieth Phase 2 slice:

- equality and inequality accept function values only when both operands have the same
  fully resolved `FunctionType`; different parameter or return types remain `N3004`;
- equality denotes top-level declaration identity rather than code-address, layout, ABI,
  or structural body equality, keeping the contract independent from future backends;
- the closed-condition evaluator can prove equality/inequality of direct function
  references, including statement-free block wrappers, while local aliases and call
  results remain dynamic and cannot manufacture definite-assignment reachability; and
- semantic regressions lock same-signature acceptance, cross-signature rejection,
  direct-reference flow refinement, and alias conservatism.

'''
addition = anchor + '''Implemented in the forty-first Phase 2 slice:

- bootstrap equality type admissibility is factored into a small public semantic rule
  over resolved HIR types instead of leaving primitive/function/enum classification
  embedded only in analyzer implementation code;
- the shared rule keeps exact-type matching explicit and delegates payload-free enum
  eligibility to declaration context, so consumers cannot infer comparability from one
  runtime variant shape alone;
- semantic analysis continues to own source diagnostics and enum declaration lookup,
  preserving accepted/rejected source behavior while making the equality contract
  reusable at later trusted boundaries; and
- focused truth-table tests lock primitive, Unit, function, record, Never/Error, nominal
  enum, and cross-signature behavior without changing syntax, HIR, CFG, or inspection
  schema shape.

'''
text = replace_once(text, anchor, addition, "Phase 2 slice 41")
text = replace_once(
    text,
    "**Status: fifteen vertical slices implemented; execution surface remains small.**",
    "**Status: sixteen vertical slices implemented; execution surface remains small.**",
    "Phase 3 count",
)
anchor = '''Implemented in the fifteenth Phase 3 slice:

- the interpreter executes equality and inequality for first-class top-level function
  values by resolved `FunctionId` declaration identity after ordinary left-to-right
  operand evaluation;
- runtime comparison independently verifies that both referenced declarations exist and
  have identical parameter and return types before comparing identity, so malformed HIR
  with cross-signature function operands fails closed as `N4005`;
- inferred local function aliases use the same runtime identity semantics as direct
  references, while no code address, layout, closure environment, or ABI contract is
  introduced; and
- interpreter malformed-HIR tests plus a CLI check/run fixture lock dynamic alias
  equality, inequality, and signature-drift rejection end to end.

'''
addition = anchor + '''Implemented in the sixteenth Phase 3 slice:

- equality execution now validates the resolved HIR operand types against the shared
  semantic equality-admissibility rule before evaluating either operand;
- enum equality rechecks declaration-wide payload freedom from the runtime program table,
  closing a malformed-HIR gap where a currently payload-free variant of a payload-bearing
  enum could previously reach the variant-slot comparison path;
- ordinary expression result conformance still validates each produced value, while the
  new operator precondition protects a distinct invariant: whether those types are legal
  operands for equality at all;
- function equality retains its independent declaration/signature validation as
  defense-in-depth after the shared type gate; and
- malformed payload-bearing-enum regression coverage plus a valid payload-free enum
  control prove the boundary fails closed with `N4005` without changing valid execution.

'''
text = replace_once(text, anchor, addition, "Phase 3 slice 16")
path.write_text(text)
