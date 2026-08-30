from pathlib import Path
import re


def replace_exact(path, old, new, count=1):
    p = Path(path)
    text = p.read_text()
    actual = text.count(old)
    if actual != count:
        raise SystemExit(f"{path}: expected {count} occurrences, found {actual}: {old[:80]!r}")
    p.write_text(text.replace(old, new, count))

# HIR: retain a source-resolved binding contract independent of the numeric id.
hir = "crates/nova-sema/src/hir.rs"
replace_exact(
    hir,
    """/// A typed lexical block.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct Block {\n""",
    """/// A resolved local/parameter reference paired with declaration metadata.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct BindingReference {\n    /// Stable analysis-order binding identity.\n    pub binding: BindingId,\n    /// Source-resolved declaration spelling.\n    pub binding_name: String,\n    /// Span of the declaration name that resolution selected.\n    pub declaration_span: Span,\n}\n\n/// A typed lexical block.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct Block {\n""",
)
replace_exact(
    hir,
    """    Assignment {\n        /// Resolved assignment target, or `None` for an already-diagnosed target.\n        target: Option<BindingId>,\n        /// Typed replacement value.\n""",
    """    Assignment {\n        /// Resolved assignment target, or `None` for an already-diagnosed target.\n        target: Option<BindingReference>,\n        /// Typed replacement value.\n""",
)
replace_exact(
    hir,
    """    /// Reference to a local binding or parameter.\n    Binding(BindingId),\n""",
    """    /// Reference to a local binding or parameter.\n    Binding(BindingReference),\n""",
)

# Analyzer: producer must retain the exact declaration chosen by lexical resolution.
analyzer = "crates/nova-sema/src/analyzer.rs"
replace_exact(
    analyzer,
    """                    Some(symbol.id)\n""",
    """                    Some(hir::BindingReference {\n                        binding: symbol.id,\n                        binding_name: target.text.clone(),\n                        declaration_span: symbol.span,\n                    })\n""",
)
replace_exact(
    analyzer,
    """            return (ExpressionKind::Binding(symbol.id), symbol.ty);\n""",
    """            return (\n                ExpressionKind::Binding(hir::BindingReference {\n                    binding: symbol.id,\n                    binding_name: name.text.clone(),\n                    declaration_span: symbol.span,\n                }),\n                symbol.ty,\n            );\n""",
)

# Interpreter: runtime slots retain declaration identity, reads/writes cross-check it.
interp = "crates/nova-interpreter/src/lib.rs"
replace_exact(
    interp,
    """    BindingId, Block, EnumId, Expression, ExpressionKind, Function, FunctionId, Program, RecordId,\n    Statement, StatementKind, Type,\n""",
    """    BindingId, BindingReference, Block, EnumId, Expression, ExpressionKind, Function, FunctionId,\n    Program, RecordId, Statement, StatementKind, Type,\n""",
)
replace_exact(
    interp,
    """struct RuntimeSlot {\n    ty: Type,\n    mutable: bool,\n    value: Option<Value>,\n}\n""",
    """struct RuntimeSlot {\n    ty: Type,\n    mutable: bool,\n    binding_name: String,\n    declaration_span: nova_source::Span,\n    value: Option<Value>,\n}\n""",
)

p = Path(interp)
text = p.read_text()
pattern = re.compile(r"            ExpressionKind::Binding\(binding\) => \{.*?                Ok\(Flow::Value\(value\.clone\(\)\)\)\n            \}", re.S)
replacement = """            ExpressionKind::Binding(reference) => {\n                self.validate_binding_reference(frame, reference, expression.span)?;\n                let slot = frame\n                    .get(&reference.binding)\n                    .expect(\"validated binding reference must have a runtime slot\");\n                if expression.ty != slot.ty {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            \"binding {} expression type {} does not match runtime slot type {}\",\n                            reference.binding.index(),\n                            expression.ty,\n                            slot.ty\n                        ),\n                    ));\n                }\n                let Some(value) = slot.value.as_ref() else {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            \"binding {} reached runtime before initialization\",\n                            reference.binding.index()\n                        ),\n                    ));\n                };\n                if !self.value_conforms_to_type(value, &slot.ty) {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            \"binding {} stored a runtime value that does not conform to slot type {}\",\n                            reference.binding.index(), slot.ty\n                        ),\n                    ));\n                }\n                Ok(Flow::Value(value.clone()))\n            }"""
text, n = pattern.subn(replacement, text, count=1)
if n != 1:
    raise SystemExit(f"{interp}: failed to replace binding read evaluator")
p.write_text(text)

old = """                    Flow::Value(value) => {\n                        let Some(slot) = frame.get_mut(target) else {\n                            return Err(self.invariant(\n                                statement.span,\n                                format!(\n                                    \"assignment target {} is absent from the frame\",\n                                    target.index()\n                                ),\n                            ));\n                        };\n                        if !slot.mutable {\n                            return Err(self.invariant(\n                                statement.span,\n                                format!(\n                                    \"assignment target {} resolved to an immutable runtime slot\",\n                                    target.index()\n                                ),\n                            ));\n                        }\n                        if !self.value_conforms_to_type(&value, &slot.ty) {\n                            return Err(self.invariant(\n                                statement.span,\n                                format!(\n                                    \"assignment target {} received a runtime value that does not conform to slot type {}\",\n                                    target.index(), slot.ty\n                                ),\n                            ));\n                        }\n                        slot.value = Some(value);\n                        Ok(None)\n                    }\n"""
new = """                    Flow::Value(value) => {\n                        self.validate_binding_reference(frame, target, statement.span)?;\n                        let slot = frame\n                            .get_mut(&target.binding)\n                            .expect(\"validated assignment target must have a runtime slot\");\n                        if !slot.mutable {\n                            return Err(self.invariant(\n                                statement.span,\n                                format!(\n                                    \"assignment target {} resolved to an immutable runtime slot\",\n                                    target.binding.index()\n                                ),\n                            ));\n                        }\n                        if !self.value_conforms_to_type(&value, &slot.ty) {\n                            return Err(self.invariant(\n                                statement.span,\n                                format!(\n                                    \"assignment target {} received a runtime value that does not conform to slot type {}\",\n                                    target.binding.index(), slot.ty\n                                ),\n                            ));\n                        }\n                        slot.value = Some(value);\n                        Ok(None)\n                    }\n"""
replace_exact(interp, old, new)

replace_exact(
    interp,
    """    fn bind_runtime_slot(\n""",
    """    fn validate_binding_reference(\n        &self,\n        frame: &Frame,\n        reference: &BindingReference,\n        span: nova_source::Span,\n    ) -> Result<(), Diagnostic> {\n        let Some(slot) = frame.get(&reference.binding) else {\n            return Err(self.invariant(\n                span,\n                format!(\n                    \"resolved binding {} is absent from the frame\",\n                    reference.binding.index()\n                ),\n            ));\n        };\n        if slot.binding_name != reference.binding_name\n            || slot.declaration_span != reference.declaration_span\n        {\n            return Err(self.invariant(\n                span,\n                format!(\n                    \"resolved binding reference `{}` does not match declaration identity for binding {}\",\n                    reference.binding_name,\n                    reference.binding.index()\n                ),\n            ));\n        }\n        Ok(())\n    }\n\n    fn bind_runtime_slot(\n""",
)
replace_exact(
    interp,
    """            if slot.ty != binding.ty || slot.mutable != binding.mutable {\n""",
    """            if slot.ty != binding.ty\n                || slot.mutable != binding.mutable\n                || slot.binding_name != binding.name\n                || slot.declaration_span != binding.span\n            {\n""",
)
replace_exact(
    interp,
    """            RuntimeSlot {\n                ty: binding.ty.clone(),\n                mutable: binding.mutable,\n                value,\n            },\n""",
    """            RuntimeSlot {\n                ty: binding.ty.clone(),\n                mutable: binding.mutable,\n                binding_name: binding.name.clone(),\n                declaration_span: binding.span,\n                value,\n            },\n""",
)

# Inspector: keep schema unchanged, independently validate name/id/declaration span.
inspect = "crates/nova-inspect/src/lib.rs"
replace_exact(
    inspect,
    """                let resolved = resolved.ok_or_else(|| {\n""",
    """                let resolved = resolved.as_ref().ok_or_else(|| {\n""",
)
replace_exact(
    inspect,
    """                let binding = self.require_known_binding(resolved, owner)?;\n                if !binding.mutable {\n                    return Err(InspectionError::invalid(format!(\n                        \"assignment targets immutable {}\",\n                        binding_id(resolved.index())\n                    )));\n                }\n                target = Some(binding_id(resolved.index()));\n""",
    """                let binding = self.require_binding_reference(resolved, owner)?;\n                if !binding.mutable {\n                    return Err(InspectionError::invalid(format!(\n                        \"assignment targets immutable {}\",\n                        binding_id(resolved.binding.index())\n                    )));\n                }\n                target = Some(binding_id(resolved.binding.index()));\n""",
)
replace_exact(
    inspect,
    """            hir::ExpressionKind::Binding(resolved) => {\n                self.require_known_binding(*resolved, owner)?;\n                target = Some(binding_id(resolved.index()));\n                v1::ExpressionKind::BindingReference\n            }\n""",
    """            hir::ExpressionKind::Binding(resolved) => {\n                self.require_binding_reference(resolved, owner)?;\n                target = Some(binding_id(resolved.binding.index()));\n                v1::ExpressionKind::BindingReference\n            }\n""",
)
replace_exact(
    inspect,
    """    fn require_known_binding(\n""",
    """    fn require_binding_reference(\n        &self,\n        reference: &hir::BindingReference,\n        owner: &str,\n    ) -> Result<&v1::Binding, InspectionError> {\n        let binding = self.require_known_binding(reference.binding, owner)?;\n        if binding.name != reference.binding_name {\n            return Err(InspectionError::invalid(format!(\n                \"binding reference `{}` does not match {} declaration name `{}`\",\n                reference.binding_name,\n                binding_id(reference.binding.index()),\n                binding.name\n            )));\n        }\n        let declaration_span = self.span(reference.declaration_span)?;\n        if binding.span != declaration_span {\n            return Err(InspectionError::invalid(format!(\n                \"binding reference `{}` does not match {} declaration span\",\n                reference.binding_name,\n                binding_id(reference.binding.index())\n            )));\n        }\n        Ok(binding)\n    }\n\n    fn require_known_binding(\n""",
)

# Keep the existing immutable-target corruption test focused on mutability by replacing
# the complete reference contract, not only its id.
frame_test = "crates/nova-interpreter/tests/frame_slot_integrity.rs"
replace_exact(
    frame_test,
    """    hir::{ExpressionKind, StatementKind, Type},\n""",
    """    hir::{BindingReference, ExpressionKind, StatementKind, Type},\n""",
)
replace_exact(
    frame_test,
    """    let fixed = match &main.body.statements[0].kind {\n        StatementKind::Binding { binding, .. } => binding.id,\n        _ => panic!(\"expected immutable binding\"),\n    };\n""",
    """    let fixed = match &main.body.statements[0].kind {\n        StatementKind::Binding { binding, .. } => binding.clone(),\n        _ => panic!(\"expected immutable binding\"),\n    };\n""",
)
replace_exact(
    frame_test,
    """    *target = Some(fixed);\n""",
    """    *target = Some(BindingReference {\n        binding: fixed.id,\n        binding_name: fixed.name,\n        declaration_span: fixed.span,\n    });\n""",
)

# Focused producer regressions.
Path("crates/nova-sema/tests/binding_reference_identity.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "binding-reference.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn binding_read_retains_resolved_declaration_identity_under_shadowing() {
    let analyzed = analyze_text("fn main() -> Int { let x: Int = 1; { let x: Int = 2; x } }");
    let main = &analyzed.program.functions[0];
    let StatementKind::Binding { binding: outer, .. } = &main.body.statements[0].kind else {
        panic!("outer binding");
    };
    let block = main.body.tail.as_deref().expect("block tail");
    let ExpressionKind::Block(block) = &block.kind else { panic!("inner block"); };
    let StatementKind::Binding { binding: inner, .. } = &block.statements[0].kind else {
        panic!("inner binding");
    };
    let reference = block.tail.as_deref().expect("inner read");
    let ExpressionKind::Binding(reference) = &reference.kind else { panic!("binding read"); };
    assert_eq!(reference.binding, inner.id);
    assert_eq!(reference.binding_name, "x");
    assert_eq!(reference.declaration_span, inner.span);
    assert_ne!(reference.declaration_span, outer.span);
}

#[test]
fn assignment_retains_resolved_target_identity() {
    let analyzed = analyze_text("fn main() -> Int { var left: Int = 1; var right: Int = 2; left = 3; left }");
    let main = &analyzed.program.functions[0];
    let StatementKind::Binding { binding: left, .. } = &main.body.statements[0].kind else {
        panic!("left binding");
    };
    let StatementKind::Assignment { target, .. } = &main.body.statements[2].kind else {
        panic!("assignment");
    };
    let target = target.as_ref().expect("resolved target");
    assert_eq!(target.binding, left.id);
    assert_eq!(target.binding_name, left.name);
    assert_eq!(target.declaration_span, left.span);
}
''')

# Runtime regressions: same-name/same-type shadow retarget, assignment retarget,
# and RHS structured-flow precedence.
Path("crates/nova-interpreter/tests/binding_reference_identity.rs").write_text(r'''use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "binding-reference-runtime.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn rejects_same_name_same_type_shadow_read_retargeting() {
    let mut analyzed = analyze_text("fn main() -> Int { let x: Int = 1; { let x: Int = 2; x } }");
    let main = &mut analyzed.program.functions[0];
    let outer = match &main.body.statements[0].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("outer binding"),
    };
    let tail = main.body.tail.as_deref_mut().expect("block tail");
    let ExpressionKind::Block(block) = &mut tail.kind else { panic!("inner block"); };
    let read = block.tail.as_deref_mut().expect("inner read");
    let ExpressionKind::Binding(reference) = &mut read.kind else { panic!("binding read"); };
    reference.binding = outer;

    let error = execute(&analyzed.program).expect_err("shadow retargeting must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_same_type_assignment_target_retargeting() {
    let mut analyzed = analyze_text("fn main() -> Int { var left: Int = 1; var right: Int = 2; left = 3; left }");
    let main = &mut analyzed.program.functions[0];
    let right = match &main.body.statements[1].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("right binding"),
    };
    let StatementKind::Assignment { target, .. } = &mut main.body.statements[2].kind else {
        panic!("assignment");
    };
    target.as_mut().expect("resolved target").binding = right;

    let error = execute(&analyzed.program).expect_err("assignment retargeting must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn assignment_rhs_return_precedes_value_only_target_validation() {
    let mut analyzed = analyze_text("fn main() -> Int { var left: Int = 1; var right: Int = 2; left = { return 9; }; left }");
    let main = &mut analyzed.program.functions[0];
    let right = match &main.body.statements[1].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("right binding"),
    };
    let StatementKind::Assignment { target, .. } = &mut main.body.statements[2].kind else {
        panic!("assignment");
    };
    target.as_mut().expect("resolved target").binding = right;

    let value = execute(&analyzed.program).expect("return must bypass unused assignment target validation");
    assert_eq!(value, Value::Int(9));
}
''')

# Inspector regressions, including same-name shadowing where name/type alone cannot help.
Path("crates/nova-inspect/tests/binding_reference_identity.rs").write_text(r'''use nova_inspect::build_document;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind},
};
use nova_source::{SourceFile, SourceId};

fn checked(text: &str) -> (SourceFile, nova_sema::hir::Program) {
    let source = SourceFile::new(SourceId::new(0), "binding-reference-inspect.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed.program)
}

#[test]
fn rejects_same_name_shadow_reference_id_drift() {
    let (source, mut program) = checked("fn main() -> Int { let x: Int = 1; { let x: Int = 2; x } }");
    let main = &mut program.functions[0];
    let outer = match &main.body.statements[0].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("outer binding"),
    };
    let tail = main.body.tail.as_deref_mut().expect("block tail");
    let ExpressionKind::Block(block) = &mut tail.kind else { panic!("inner block"); };
    let read = block.tail.as_deref_mut().expect("inner read");
    let ExpressionKind::Binding(reference) = &mut read.kind else { panic!("binding read"); };
    reference.binding = outer;

    let error = build_document(&program, &source).expect_err("binding drift must fail closed");
    assert!(error.message().contains("declaration span"));
}

#[test]
fn rejects_assignment_target_identity_drift() {
    let (source, mut program) = checked("fn main() -> Int { var left: Int = 1; var right: Int = 2; left = 3; left }");
    let main = &mut program.functions[0];
    let right = match &main.body.statements[1].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("right binding"),
    };
    let StatementKind::Assignment { target, .. } = &mut main.body.statements[2].kind else {
        panic!("assignment");
    };
    target.as_mut().expect("resolved target").binding = right;

    let error = build_document(&program, &source).expect_err("assignment drift must fail closed");
    assert!(error.message().contains("binding reference"));
}
''')
