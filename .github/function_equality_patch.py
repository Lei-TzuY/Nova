from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    if old not in text:
        raise SystemExit(f"anchor not found in {path}: {old[:120]!r}")
    file.write_text(text.replace(old, new, 1))


# Semantic equality contract.
path = "crates/nova-sema/src/analyzer.rs"
text = Path(path).read_text()
text = text.replace(
    "equality requires Int, Bool, Unit, or payload-free enum operands, found {} and {}",
    "equality requires matching comparable operands (Int, Bool, Unit, function, or payload-free enum), found {} and {}",
)
text = text.replace(
    "equality requires matching Int, Bool, Unit, or payload-free enum operands, found {} and {}",
    "equality requires matching comparable operands (Int, Bool, Unit, function, or payload-free enum), found {} and {}",
)
old = """        match ty {\n            Type::Int | Type::Bool | Type::Unit => true,\n            Type::Enum(enumeration) => self\n"""
new = """        match ty {\n            Type::Int | Type::Bool | Type::Unit | Type::Function(_) => true,\n            Type::Enum(enumeration) => self\n"""
if old not in text:
    raise SystemExit("equality-comparable anchor not found")
Path(path).write_text(text.replace(old, new, 1))

# Closed condition proof for direct function references only.
path = "crates/nova-sema/src/constant_condition.rs"
text = Path(path).read_text()
text = text.replace(
    "use crate::hir::{Expression, ExpressionKind, Type};",
    "use crate::hir::{Expression, ExpressionKind, FunctionId, Type};",
    1,
)
old = """                (Type::Unit, Type::Unit) => {\n                    unit_value(left)?;\n                    unit_value(right)?;\n                    true\n                }\n                (Type::Enum(left_enum), Type::Enum(right_enum))\n"""
new = """                (Type::Unit, Type::Unit) => {\n                    unit_value(left)?;\n                    unit_value(right)?;\n                    true\n                }\n                (Type::Function(left_function), Type::Function(right_function))\n                    if left_function == right_function =>\n                {\n                    function_id(left)? == function_id(right)?\n                }\n                (Type::Enum(left_enum), Type::Enum(right_enum))\n"""
if old not in text:
    raise SystemExit("constant equality anchor not found")
text = text.replace(old, new, 1)
text += """

fn function_id(expression: &Expression) -> Option<FunctionId> {
    match &expression.kind {
        ExpressionKind::Function(function) => Some(*function),
        ExpressionKind::Block(block) if block.statements.is_empty() => {
            function_id(block.tail.as_deref()?)
        }
        _ => None,
    }
}
"""
Path(path).write_text(text)

# Runtime function identity equality with signature fail-closed validation.
path = "crates/nova-interpreter/src/lib.rs"
text = Path(path).read_text()
old = """            (BinaryOperator::Equal, Value::Unit, Value::Unit) => Ok(Value::Bool(true)),\n            (BinaryOperator::NotEqual, Value::Unit, Value::Unit) => Ok(Value::Bool(false)),\n            (\n                BinaryOperator::Equal,\n                Value::Enum {\n"""
new = """            (BinaryOperator::Equal, Value::Unit, Value::Unit) => Ok(Value::Bool(true)),\n            (BinaryOperator::NotEqual, Value::Unit, Value::Unit) => Ok(Value::Bool(false)),\n            (BinaryOperator::Equal, Value::Function(left), Value::Function(right)) => {\n                if !self.function_signatures_match(left, right) {\n                    return Err(self.invariant(\n                        expression.span,\n                        "function equality received runtime functions with different signatures",\n                    ));\n                }\n                Ok(Value::Bool(left == right))\n            }\n            (BinaryOperator::NotEqual, Value::Function(left), Value::Function(right)) => {\n                if !self.function_signatures_match(left, right) {\n                    return Err(self.invariant(\n                        expression.span,\n                        "function equality received runtime functions with different signatures",\n                    ));\n                }\n                Ok(Value::Bool(left != right))\n            }\n            (\n                BinaryOperator::Equal,\n                Value::Enum {\n"""
if old not in text:
    raise SystemExit("runtime equality anchor not found")
text = text.replace(old, new, 1)
old = """    fn value_conforms_to_type(&self, value: &Value, ty: &Type) -> bool {\n"""
new = """    fn function_signatures_match(&self, left: FunctionId, right: FunctionId) -> bool {\n        let Some(left_function) = self.program.functions.get(left.index()) else {\n            return false;\n        };\n        let Some(right_function) = self.program.functions.get(right.index()) else {\n            return false;\n        };\n        left_function.id == left\n            && right_function.id == right\n            && left_function.parameters.len() == right_function.parameters.len()\n            && left_function\n                .parameters\n                .iter()\n                .zip(&right_function.parameters)\n                .all(|(left_parameter, right_parameter)| left_parameter.ty == right_parameter.ty)\n            && left_function.return_type == right_function.return_type\n    }\n\n    fn value_conforms_to_type(&self, value: &Value, ty: &Type) -> bool {\n"""
if old not in text:
    raise SystemExit("runtime helper anchor not found")
Path(path).write_text(text.replace(old, new, 1))

# Semantic regressions.
Path("crates/nova-sema/tests/function_equality.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze, hir::Type};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "function-equality.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "lex diagnostics: {:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "parse diagnostics: {:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

fn codes(output: &AnalysisOutput) -> Vec<&str> {
    output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

#[test]
fn matching_function_signatures_are_equality_comparable() {
    let output = analyze_text(
        "fn first() -> Int { 1 }\n\
         fn second() -> Int { 2 }\n\
         fn main() -> Bool { first == first && first != second }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(output.program.functions[2].body.ty, Type::Bool);
}

#[test]
fn different_function_signatures_remain_non_comparable() {
    let output = analyze_text(
        "fn integer() -> Int { 1 }\n\
         fn boolean() -> Bool { true }\n\
         fn main() -> Bool { integer == boolean }",
    );
    assert_eq!(codes(&output), vec!["N3004"]);
}

#[test]
fn direct_function_identity_refines_reachability_through_empty_blocks() {
    let output = analyze_text(
        "fn target() -> Int { 1 }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             if { target } == target { value = 42; () } else { () };\n\
             value\n\
         }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn local_function_alias_equality_remains_dynamic_for_flow() {
    let output = analyze_text(
        "fn target() -> Int { 1 }\n\
         fn main() -> Int {\n\
             let alias = target;\n\
             var value: Int;\n\
             if alias == target { value = 42; () } else { () };\n\
             value\n\
         }",
    );
    assert_eq!(codes(&output), vec!["N3009"]);
}
''')

# Interpreter regressions including malformed-HIR signature drift.
Path("crates/nova-interpreter/tests/function_equality.rs").write_text(r'''use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, FunctionType, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "function-equality.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "lex diagnostics: {:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "parse diagnostics: {:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "semantic diagnostics: {:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn runtime_function_aliases_compare_by_declaration_identity() {
    let analyzed = analyze_text(
        "fn first() -> Int { 1 }\n\
         fn second() -> Int { 2 }\n\
         fn main() -> Bool {\n\
             let left = first;\n\
             let right = second;\n\
             left == first && left != right\n\
         }",
    );
    let value = execute(&analyzed.program).expect("function identity equality should execute");
    assert_eq!(value, Value::Bool(true));
}

#[test]
fn malformed_function_equality_signature_drift_fails_closed() {
    let mut analyzed = analyze_text(
        "fn first() -> Int { 1 }\n\
         fn second() -> Int { 2 }\n\
         fn flag() -> Bool { true }\n\
         fn main() -> Bool { first == second }",
    );
    let flag = analyzed
        .program
        .functions
        .iter()
        .find(|function| function.name == "flag")
        .expect("flag function")
        .id;
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let expression = main.body.tail.as_deref_mut().expect("main tail expression");
    let ExpressionKind::Binary { right, .. } = &mut expression.kind else {
        panic!("expected equality expression");
    };
    right.kind = ExpressionKind::Function(flag);
    right.ty = Type::Function(FunctionType {
        parameters: Vec::new(),
        return_type: Box::new(Type::Bool),
    });

    let error = execute(&analyzed.program).expect_err("signature drift must fail closed");
    assert_eq!(error.code, "N4005");
}
''')

# CLI fixture and coverage.
Path("crates/nova-cli/tests/fixtures/valid/function-equality.nv").write_text('''fn first() -> Int { 1 }\nfn second() -> Int { 2 }\nfn main() -> Bool {\n    let alias = first;\n    alias == first && first != second\n}\n''')
path = "crates/nova-cli/tests/cli.rs"
text = Path(path).read_text()
text = text.replace(
    '        "valid/payload-free-enum-equality.nv",\n',
    '        "valid/payload-free-enum-equality.nv",\n        "valid/function-equality.nv",\n',
    1,
)
text = text.replace(
    '        ("valid/payload-free-enum-equality.nv", "true\\n", None),\n',
    '        ("valid/payload-free-enum-equality.nv", "true\\n", None),\n        ("valid/function-equality.nv", "true\\n", None),\n',
    1,
)
Path(path).write_text(text)

# README semantics and proof documentation, including pre-existing block-proof corrections.
path = "README.md"
text = Path(path).read_text()
old = """A direct literal `while true { body }` is the one bootstrap exception because it\nhas no condition-false exit. The checker treats it as guaranteed-entry and\nrecords only reachable `break` transfers that target that exact loop. If there\nare such exits, a pre-existing binding is definitely initialized afterward only\nwhen it is initialized at every reachable break exit. If there is no reachable\nbreak, the loop is non-continuing. A `break` consumed by a nested loop does not\ncount as an exit from an outer loop. This is deliberately not constant folding:\n`while { true }`, computed booleans, and other equivalent-looking conditions keep\nthe ordinary conservative rule.\n"""
new = """When the closed-condition evaluator proves a `while` condition true (for example\n`true` or the statement-free wrapper `{ true }`), the loop has no condition-false\nexit. The checker treats it as guaranteed-entry and records only reachable `break`\ntransfers that target that exact loop. If there are such exits, a pre-existing\nbinding is definitely initialized afterward only when it is initialized at every\nreachable break exit. If there is no reachable break, the loop is non-continuing.\nA `break` consumed by a nested loop does not count as an exit from an outer loop.\nThis proof changes flow analysis only; the retained HIR is never constant-folded.\n"""
if old not in text:
    raise SystemExit("README while proof anchor not found")
text = text.replace(old, new, 1)
text = text.replace(
    "equality accepts matching `Int`, `Bool`,\n`Unit`, or the same nominal payload-free enum type; calls require matching arity and\nargument types.",
    "equality accepts matching `Int`, `Bool`, `Unit`, the same function signature, or\nthe same nominal payload-free enum type; function equality compares declaration\nidentity rather than addresses or code layout, and calls require matching arity and\nargument types.",
    1,
)
old = """execution-failure diagnostics. The same side-effect-free literal Bool/Int evaluator\nmay determine `if`, `while`, and short-circuit reachability from closed comparisons\nand Boolean operations; this changes flow analysis only and never folds the retained\nHIR. Names, calls, blocks, aggregates, and other dynamic operands stop the proof.\n"""
new = """execution-failure diagnostics. The same side-effect-free closed evaluator may\ndetermine `if`, `while`, and short-circuit reachability from Bool/Int/Unit values,\ndirect payload-free enum constructors, direct top-level function references,\nstatement-free block wrappers, comparisons, and Boolean operations; this changes\nflow analysis only and never folds the retained HIR. Local bindings, calls,\nstatement-bearing blocks, other aggregates, and other dynamic operands stop the proof.\n"""
if old not in text:
    raise SystemExit("README constant proof anchor not found")
Path(path).write_text(text.replace(old, new, 1))

# Constitution: identity semantics are language-level, not ABI identity.
path = "docs/language-constitution.md"
text = Path(path).read_text()
old = """Runtime record values carry nominal identity and declaration-order field slots;\nruntime enum values carry nominal identity, a declaration-order variant slot,\nand an optional boxed payload. Those representations are executable semantic\noracles, not stable layouts, allocation promises, serialization formats, or\nbackend ABIs. Runtime failures use structured diagnostics. Recursive execution\n"""
new = """Runtime record values carry nominal identity and declaration-order field slots;\nruntime enum values carry nominal identity, a declaration-order variant slot,\nand an optional boxed payload. Top-level function values carry declaration identity.\nEquality is defined only between matching function signatures and compares that\ndeclaration identity; it does not expose code addresses, pointer equality, layout, or\nABI identity. Those representations are executable semantic oracles, not stable\nlayouts, allocation promises, serialization formats, or backend ABIs. Runtime failures\nuse structured diagnostics. Recursive execution\n"""
if old not in text:
    raise SystemExit("constitution runtime identity anchor not found")
Path(path).write_text(text.replace(old, new, 1))

# Roadmap Phase 2 + Phase 3 accounting.
path = "docs/roadmap.md"
text = Path(path).read_text()
text = text.replace(
    "**Status: thirty-nine vertical slices implemented; broader type-system work remains.**",
    "**Status: forty vertical slices implemented; broader type-system work remains.**",
    1,
)
phase2 = """
Implemented in the fortieth Phase 2 slice:

- equality and inequality accept function values only when both operands have the same
  fully resolved `FunctionType`; different parameter or return types remain `N3004`;
- equality denotes top-level declaration identity rather than code-address, layout, ABI,
  or structural body equality, keeping the contract independent from future backends;
- the closed-condition evaluator can prove equality/inequality of direct function
  references, including statement-free block wrappers, while local aliases and call
  results remain dynamic and cannot manufacture definite-assignment reachability; and
- semantic regressions lock same-signature acceptance, cross-signature rejection,
  direct-reference flow refinement, and alias conservatism.

"""
anchor = "The next Phase 2 slices should address semantic depth rather than widen syntax\nprematurely. In particular:\n"
if anchor not in text:
    raise SystemExit("Phase 2 insertion anchor not found")
text = text.replace(anchor, phase2 + anchor, 1)
text = text.replace(
    "**Status: fourteen vertical slices implemented; execution surface remains small.**",
    "**Status: fifteen vertical slices implemented; execution surface remains small.**",
    1,
)
phase3 = """
Implemented in the fifteenth Phase 3 slice:

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

"""
anchor = "Next Phase 3 slices should deepen executable semantics without bypassing Phase 2\ncontracts:\n"
if anchor not in text:
    raise SystemExit("Phase 3 insertion anchor not found")
Path(path).write_text(text.replace(anchor, phase3 + anchor, 1))
