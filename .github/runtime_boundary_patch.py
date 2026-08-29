from pathlib import Path

lib = Path("crates/nova-interpreter/src/lib.rs")
text = lib.read_text()

old = """        if function.parameters.len() != arguments.len() {
            return Err(self.invariant(
                function.span,
                format!(
                    "resolved call supplied {} argument(s) to {} parameter(s)",
                    arguments.len(),
                    function.parameters.len()
                ),
            ));
        }
        if self.call_depth >= MAX_CALL_DEPTH {"""
new = """        if function.id != function_id {
            return Err(self.invariant(
                function.span,
                "function declaration index does not match its resolved identity",
            ));
        }
        if function.parameters.len() != arguments.len() {
            return Err(self.invariant(
                function.span,
                format!(
                    "resolved call supplied {} argument(s) to {} parameter(s)",
                    arguments.len(),
                    function.parameters.len()
                ),
            ));
        }
        for (index, (parameter, argument)) in
            function.parameters.iter().zip(&arguments).enumerate()
        {
            if !self.value_conforms_to_type(argument, &parameter.ty) {
                return Err(self.invariant(
                    function.span,
                    format!(
                        "argument {index} for parameter `{}` of function `{}` does not conform to declared runtime type {}",
                        parameter.name, function.name, parameter.ty
                    ),
                ));
            }
        }
        if self.call_depth >= MAX_CALL_DEPTH {"""
if old not in text:
    raise SystemExit("call boundary anchor not found")
text = text.replace(old, new, 1)

old = """        self.call_depth += 1;
        let result = self.eval_function(&function, &mut frame);
        self.call_depth -= 1;
        result
    }

    fn eval_function("""
new = """        self.call_depth += 1;
        let result = self.eval_function(&function, &mut frame);
        self.call_depth -= 1;
        let value = result?;
        if !self.value_conforms_to_type(&value, &function.return_type) {
            return Err(self.invariant(
                function.span,
                format!(
                    "function `{}` returned a runtime value that does not conform to declared type {}",
                    function.name, function.return_type
                ),
            ));
        }
        Ok(value)
    }

    fn eval_function("""
if old not in text:
    raise SystemExit("return boundary anchor not found")
text = text.replace(old, new, 1)

anchor = "    fn step(&mut self, span: nova_source::Span) -> Result<(), Diagnostic> {"
helper = """    fn value_conforms_to_type(&self, value: &Value, ty: &Type) -> bool {
        match (value, ty) {
            (Value::Int(_), Type::Int)
            | (Value::Bool(_), Type::Bool)
            | (Value::Unit, Type::Unit) => true,
            (Value::Record { record, fields }, Type::Record(expected))
                if *record == expected.id =>
            {
                let Some(definition) = self.program.records.get(record.index()) else {
                    return false;
                };
                definition.id == *record
                    && fields.len() == definition.fields.len()
                    && fields
                        .iter()
                        .zip(&definition.fields)
                        .all(|(value, field)| self.value_conforms_to_type(value, &field.ty))
            }
            (
                Value::Enum {
                    enumeration,
                    variant_index,
                    payload,
                },
                Type::Enum(expected),
            ) if *enumeration == expected.id => {
                let Some(definition) = self.program.enums.get(enumeration.index()) else {
                    return false;
                };
                if definition.id != *enumeration {
                    return false;
                }
                let Some(variant) = definition.variants.get(*variant_index) else {
                    return false;
                };
                match (&variant.payload, payload.as_deref()) {
                    (None, None) => true,
                    (Some(payload_type), Some(payload_value)) => {
                        self.value_conforms_to_type(payload_value, payload_type)
                    }
                    _ => false,
                }
            }
            (Value::Function(id), Type::Function(expected)) => {
                let Some(function) = self.program.functions.get(id.index()) else {
                    return false;
                };
                function.id == *id
                    && function.parameters.len() == expected.parameters.len()
                    && function
                        .parameters
                        .iter()
                        .zip(&expected.parameters)
                        .all(|(parameter, expected_type)| &parameter.ty == expected_type)
                    && &function.return_type == expected.return_type.as_ref()
            }
            _ => false,
        }
    }

    fn step(&mut self, span: nova_source::Span) -> Result<(), Diagnostic> {"""
if anchor not in text:
    raise SystemExit("helper anchor not found")
text = text.replace(anchor, helper, 1)
lib.write_text(text)

tests = Path("crates/nova-interpreter/tests/function_boundary_integrity.rs")
tests.write_text("""use nova_interpreter::execute;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "function-boundary.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "lex diagnostics: {:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(
        parsed.is_success(),
        "parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let analyzed = analyze(&parsed.program);
    assert!(
        analyzed.is_success(),
        "semantic diagnostics: {:?}",
        analyzed.diagnostics
    );
    analyzed
}

#[test]
fn rejects_malformed_runtime_return_type() {
    let mut analyzed = analyze_text("fn main() -> Int { 42 }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let tail = main.body.tail.as_deref_mut().expect("main tail");
    tail.kind = ExpressionKind::Boolean(true);
    tail.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("runtime return drift must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_malformed_runtime_argument_type_before_calling_the_function() {
    let mut analyzed =
        analyze_text("fn take(value: Int) -> Int { 7 } fn main() -> Int { take(42) }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let tail = main.body.tail.as_deref_mut().expect("main tail");
    let ExpressionKind::Call { arguments, .. } = &mut tail.kind else {
        panic!("expected call tail");
    };
    arguments[0].kind = ExpressionKind::Boolean(true);
    arguments[0].ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("runtime argument drift must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn recursively_validates_nominal_record_slots_at_function_boundaries() {
    let mut analyzed = analyze_text(
        "record Box { value: Int } fn make() -> Box { new Box { value: 42 } } fn main() -> Unit { make(); }",
    );
    let make = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "make")
        .expect("make function");
    let tail = make.body.tail.as_deref_mut().expect("make tail");
    let ExpressionKind::RecordLiteral { fields, .. } = &mut tail.kind else {
        panic!("expected record literal tail");
    };
    fields[0].value.kind = ExpressionKind::Boolean(true);
    fields[0].value.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("corrupt record slot must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_nominal_return_identity_drift() {
    let mut analyzed = analyze_text(
        "record Left { value: Int } record Right { value: Int } fn make() -> Left { new Left { value: 42 } } fn main() -> Unit { make(); }",
    );
    let right = analyzed.program.records[1].id;
    let make = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "make")
        .expect("make function");
    let tail = make.body.tail.as_deref_mut().expect("make tail");
    let ExpressionKind::RecordLiteral { record, .. } = &mut tail.kind else {
        panic!("expected record literal tail");
    };
    *record = right;

    let error = execute(&analyzed.program).expect_err("nominal identity drift must fail");
    assert_eq!(error.code, "N4005");
}
""")

readme = Path("README.md")
text = readme.read_text()
anchor = """If malformed HIR lets loop control escape its lexical
loop or cross a function boundary, execution fails closed with invariant
diagnostic `N4005` rather than guessing a target.
"""
replacement = anchor + """
Function calls also validate the runtime/HIR type boundary. Every argument must
recursively conform to its resolved parameter type, and every returned runtime
value must recursively conform to the function's declared type. Nominal record
and enum identities, record slots, and enum payloads are checked rather than
trusted from their outer value tag alone. Valid semantically produced HIR is
unaffected; malformed or contract-drifted HIR fails closed with `N4005`.
"""
if anchor not in text:
    raise SystemExit("README boundary anchor not found")
readme.write_text(text.replace(anchor, replacement, 1))

roadmap = Path("docs/roadmap.md")
text = roadmap.read_text()
status_old = "**Status: nine vertical slices implemented; execution surface remains small.**"
if status_old not in text:
    raise SystemExit("roadmap Phase 3 status anchor not found")
text = text.replace(
    status_old,
    "**Status: eleven vertical slices implemented; execution surface remains small.**",
    1,
)
anchor = """- the interpreter's boxed payload representation, enum layout, ownership, and ABI remain
  explicitly provisional and unaffected by this equality slice.

Next Phase 3 slices should deepen executable semantics without bypassing Phase 2
contracts:
"""
insertion = """- the interpreter's boxed payload representation, enum layout, ownership, and ABI remain
  explicitly provisional and unaffected by this equality slice.

Implemented in the tenth Phase 3 slice:

- zero-argument `main` may return `Unit` alongside the existing `Int` and `Bool`
  bootstrap entry-point types;
- `nova run` prints the existing `Value::Unit` representation as `()` rather than
  rejecting an otherwise semantically valid Unit-valued entry point;
- record-, enum-, and function-valued entry points remain outside the bootstrap
  execution contract and continue to fail with `N4001`; and
- interpreter integration tests plus CLI fixture migration lock both the newly
  accepted Unit entry point and the still-narrow aggregate boundary.

Implemented in the eleventh Phase 3 slice:

- every function call validates runtime arguments against resolved parameter
  types before binding them into a frame;
- every function return validates its runtime value against the declared return
  type before the value crosses the call boundary;
- nominal record and enum validation recursively checks declaration identity,
  record slots, selected variant, and payload shape/type instead of trusting an
  outer runtime tag alone;
- function-value validation checks resolved function identity and signature,
  while `Never` and recovery `Error` can never masquerade as runtime values; and
- malformed-HIR regressions lock argument, return, nested-record, and nominal
  identity drift to deterministic invariant diagnostic `N4005` without changing
  valid source behavior.

Next Phase 3 slices should deepen executable semantics without bypassing Phase 2
contracts:
"""
if anchor not in text:
    raise SystemExit("roadmap Phase 3 anchor not found")
roadmap.write_text(text.replace(anchor, insertion, 1))
