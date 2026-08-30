from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one anchor, found {count}")
    return text.replace(old, new, 1)

# HIR: retain source-resolved variant spelling alongside declaration slot identity.
path = Path("crates/nova-sema/src/hir.rs")
text = path.read_text()
text = replace_once(
    text,
    "pub struct MatchArm {\n    /// Zero-based variant slot in declaration order.\n    pub variant_index: usize,",
    "pub struct MatchArm {\n    /// Resolved variant spelling paired with the declaration-order slot.\n    pub variant_name: String,\n    /// Zero-based variant slot in declaration order.\n    pub variant_index: usize,",
    "MatchArm variant identity",
)
text = replace_once(
    text,
    "        /// Resolved nominal enum identity.\n        enumeration: EnumId,\n        /// Zero-based variant slot in declaration order.\n        variant_index: usize,",
    "        /// Resolved nominal enum identity.\n        enumeration: EnumId,\n        /// Resolved variant spelling paired with the declaration-order slot.\n        variant_name: String,\n        /// Zero-based variant slot in declaration order.\n        variant_index: usize,",
    "EnumConstructor variant identity",
)
path.write_text(text)

# Analyzer: producer must preserve the spelling it actually resolved.
path = Path("crates/nova-sema/src/analyzer.rs")
text = path.read_text()
text = replace_once(
    text,
    "                ExpressionKind::EnumConstructor {\n                    enumeration: enum_id,\n                    variant_index,\n                    payload,\n                },",
    "                ExpressionKind::EnumConstructor {\n                    enumeration: enum_id,\n                    variant_name: variant.text.clone(),\n                    variant_index,\n                    payload,\n                },",
    "enum constructor lowering",
)
text = replace_once(
    text,
    "                lowered_arms.push(MatchArm {\n                    variant_index,\n                    binding: payload_binding,",
    "                lowered_arms.push(MatchArm {\n                    variant_name: arm.pattern.variant.text.clone(),\n                    variant_index,\n                    binding: payload_binding,",
    "match arm lowering",
)
path.write_text(text)

# Interpreter: validate member identity only after child control flow has produced an ordinary value.
path = Path("crates/nova-interpreter/src/lib.rs")
text = path.read_text()
constructor_pattern = re.compile(
    r"            ExpressionKind::EnumConstructor \{\n"
    r"                enumeration,\n"
    r"                variant_index,\n"
    r"                payload,\n"
    r"            \} => \{.*?"
    r"            \}\n"
    r"            ExpressionKind::FieldAccess \{",
    re.S,
)
constructor_replacement = '''            ExpressionKind::EnumConstructor {
                enumeration,
                variant_name,
                variant_index,
                payload,
            } => {
                let payload = if let Some(payload) = payload {
                    match self.eval_expression(payload, frame)? {
                        Flow::Value(value) => Some(Box::new(value)),
                        flow => return Ok(flow),
                    }
                } else {
                    None
                };
                let (definition, variant) = self.resolved_enum_variant(
                    *enumeration,
                    *variant_index,
                    variant_name,
                    expression.span,
                )?;
                if variant.payload.is_some() != payload.is_some() {
                    return Err(self.invariant(
                        expression.span,
                        "resolved enum constructor payload arity does not match its variant",
                    ));
                }
                if let (Some(payload_type), Some(payload_value)) =
                    (&variant.payload, payload.as_deref())
                {
                    if !self.value_conforms_to_type(payload_value, payload_type) {
                        return Err(self.invariant(
                            expression.span,
                            format!(
                                "enum variant `{}` of `{}` received a runtime payload that does not conform to declared type {}",
                                variant.name, definition.name, payload_type
                            ),
                        ));
                    }
                }
                Ok(Flow::Value(Value::Enum {
                    enumeration: *enumeration,
                    variant_index: *variant_index,
                    payload,
                }))
            }
            ExpressionKind::FieldAccess {'''
text, count = constructor_pattern.subn(constructor_replacement, text, count=1)
if count != 1:
    raise SystemExit(f"interpreter enum constructor arm: expected one replacement, found {count}")

match_pattern = re.compile(
    r"            ExpressionKind::Match \{\n"
    r"                scrutinee,\n"
    r"                enumeration,\n"
    r"                arms,\n"
    r"            \} => \{.*?"
    r"            \}\n"
    r"            ExpressionKind::Error =>",
    re.S,
)
match_replacement = '''            ExpressionKind::Match {
                scrutinee,
                enumeration,
                arms,
            } => {
                let scrutinee = match self.eval_expression(scrutinee, frame)? {
                    Flow::Value(value) => value,
                    flow => return Ok(flow),
                };
                let Value::Enum {
                    enumeration: actual_enum,
                    variant_index,
                    payload,
                } = scrutinee
                else {
                    return Err(self.invariant(
                        expression.span,
                        "semantically accepted match did not evaluate to an enum",
                    ));
                };
                if actual_enum != *enumeration {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "match expected enum {}, found enum {}",
                            enumeration.index(),
                            actual_enum.index()
                        ),
                    ));
                }

                let Some(definition) = self.program.enums.get(enumeration.index()) else {
                    return Err(self.invariant(
                        expression.span,
                        format!(
                            "resolved enum id {} is outside the program",
                            enumeration.index()
                        ),
                    ));
                };
                if definition.id != *enumeration || arms.len() != definition.variants.len() {
                    return Err(self.invariant(
                        expression.span,
                        "resolved match is not exhaustive for its enum declaration",
                    ));
                }
                let mut covered = vec![false; definition.variants.len()];
                for arm in arms {
                    let (_, declared) = self.resolved_enum_variant(
                        *enumeration,
                        arm.variant_index,
                        &arm.variant_name,
                        arm.span,
                    )?;
                    let Some(slot) = covered.get_mut(arm.variant_index) else {
                        return Err(self.invariant(
                            arm.span,
                            "resolved match arm targets a variant outside its enum",
                        ));
                    };
                    if *slot {
                        return Err(self.invariant(
                            arm.span,
                            "resolved match contains a duplicate variant arm",
                        ));
                    }
                    *slot = true;
                    if declared.payload.is_some() != arm.binding.is_some() {
                        return Err(self.invariant(
                            arm.span,
                            "resolved match binding arity does not match its variant",
                        ));
                    }
                }

                let Some(arm) = arms.iter().find(|arm| arm.variant_index == variant_index) else {
                    return Err(self.invariant(
                        expression.span,
                        "exhaustive match has no arm for the runtime variant",
                    ));
                };
                match (&arm.binding, payload) {
                    (Some(binding), Some(payload)) => {
                        self.bind_runtime_slot(frame, binding, Some(*payload), arm.span)?;
                    }
                    (None, None) => {}
                    _ => {
                        return Err(self.invariant(
                            arm.span,
                            "runtime enum payload arity does not match the selected arm",
                        ));
                    }
                }
                self.eval_expression(&arm.value, frame)
            }
            ExpressionKind::Error =>'''
text, count = match_pattern.subn(match_replacement, text, count=1)
if count != 1:
    raise SystemExit(f"interpreter match arm: expected one replacement, found {count}")

helper_anchor = "    fn resolved_record_field(\n"
if text.count(helper_anchor) != 1:
    raise SystemExit("resolved_record_field helper anchor missing")
enum_helper = '''    fn resolved_enum_variant(
        &self,
        enumeration: EnumId,
        variant_index: usize,
        variant_name: &str,
        span: nova_source::Span,
    ) -> Result<(&nova_sema::hir::Enum, &nova_sema::hir::EnumVariant), Diagnostic> {
        let Some(definition) = self.program.enums.get(enumeration.index()) else {
            return Err(self.invariant(
                span,
                format!("resolved enum id {} is outside the program", enumeration.index()),
            ));
        };
        if definition.id != enumeration {
            return Err(self.invariant(
                span,
                "enum declaration index does not match its resolved identity",
            ));
        }
        let Some(variant) = definition.variants.get(variant_index) else {
            return Err(self.invariant(
                span,
                format!(
                    "variant slot {variant_index} is outside enum `{}`",
                    definition.name
                ),
            ));
        };
        if variant.name != variant_name {
            return Err(self.invariant(
                span,
                format!(
                    "resolved enum variant `{variant_name}` does not match declaration slot {variant_index} (`{}`)",
                    variant.name
                ),
            ));
        }
        Ok((definition, variant))
    }

'''
text = text.replace(helper_anchor, enum_helper + helper_anchor, 1)
path.write_text(text)

# Inspector: independently cross-check compiler-owned spelling before publishing existing schema IDs.
path = Path("crates/nova-inspect/src/lib.rs")
text = path.read_text()
text = replace_once(
    text,
    "            hir::ExpressionKind::EnumConstructor {\n                enumeration,\n                variant_index,\n                payload,\n            } => {",
    "            hir::ExpressionKind::EnumConstructor {\n                enumeration,\n                variant_name,\n                variant_index,\n                payload,\n            } => {",
    "inspector enum constructor destructure",
)
text = replace_once(
    text,
    "                let variant = declaration.variants.get(*variant_index).ok_or_else(|| {\n                    InspectionError::invalid(format!(\n                        \"enum construction references out-of-range variant slot {variant_index}\"\n                    ))\n                })?;\n                if variant.payload.is_some() != payload.is_some() {",
    "                let variant = declaration.variants.get(*variant_index).ok_or_else(|| {\n                    InspectionError::invalid(format!(\n                        \"enum construction references out-of-range variant slot {variant_index}\"\n                    ))\n                })?;\n                if variant.name != *variant_name {\n                    return Err(InspectionError::invalid(format!(\n                        \"enum construction variant `{variant_name}` does not match slot {variant_index} declaration `{}`\",\n                        variant.name\n                    )));\n                }\n                if variant.payload.is_some() != payload.is_some() {",
    "inspector constructor variant check",
)
text = replace_once(
    text,
    "                    let variant = declaration.variants.get(arm.variant_index).ok_or_else(|| {\n                        InspectionError::invalid(format!(\n                            \"match references out-of-range variant slot {}\",\n                            arm.variant_index\n                        ))\n                    })?;\n                    if !seen.insert(arm.variant_index) {",
    "                    let variant = declaration.variants.get(arm.variant_index).ok_or_else(|| {\n                        InspectionError::invalid(format!(\n                            \"match references out-of-range variant slot {}\",\n                            arm.variant_index\n                        ))\n                    })?;\n                    if variant.name != arm.variant_name {\n                        return Err(InspectionError::invalid(format!(\n                            \"match variant `{}` does not match slot {} declaration `{}`\",\n                            arm.variant_name, arm.variant_index, variant.name\n                        )));\n                    }\n                    if !seen.insert(arm.variant_index) {",
    "inspector match variant check",
)
path.write_text(text)

# Sema producer regression.
Path("crates/nova-sema/tests/enum_variant_identity.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

#[test]
fn enum_hir_retains_resolved_variant_spelling_and_slot() {
    let source = SourceFile::new(
        SourceId::new(0),
        "enum-variant-identity.nv",
        "enum Choice { Left(Int), Right(Int), } fn main() -> Int { match Choice::Right(7) { Choice::Left(value) => 0, Choice::Right(value) => value, } }",
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);

    let tail = analyzed.program.functions[0]
        .body
        .tail
        .as_deref()
        .expect("match tail");
    let ExpressionKind::Match { scrutinee, arms, .. } = &tail.kind else {
        panic!("expected match HIR");
    };
    let ExpressionKind::EnumConstructor {
        variant_name,
        variant_index,
        ..
    } = &scrutinee.kind
    else {
        panic!("expected enum constructor scrutinee");
    };
    assert_eq!(variant_name, "Right");
    assert_eq!(*variant_index, 1);
    assert_eq!(arms.len(), 2);
    assert_eq!(arms[0].variant_name, "Left");
    assert_eq!(arms[0].variant_index, 0);
    assert_eq!(arms[1].variant_name, "Right");
    assert_eq!(arms[1].variant_index, 1);
}
''')

# Runtime corruption + structured-flow regressions.
Path("crates/nova-interpreter/tests/enum_variant_identity.rs").write_text(r'''use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::{ExpressionKind, StatementKind}};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "enum-variant-identity.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "lex diagnostics: {:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "parse diagnostics: {:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "semantic diagnostics: {:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn rejects_same_payload_type_constructor_slot_retargeting() {
    let mut analyzed = analyze_text(
        "enum Choice { Left(Int), Right(Int), } fn main() -> Int { match Choice::Left(7) { Choice::Left(value) => value, Choice::Right(value) => 0, } }",
    );
    let tail = analyzed.program.functions[0].body.tail.as_deref_mut().expect("match tail");
    let ExpressionKind::Match { scrutinee, .. } = &mut tail.kind else { panic!("match HIR"); };
    let ExpressionKind::EnumConstructor { variant_name, variant_index, .. } = &mut scrutinee.kind else { panic!("constructor HIR"); };
    assert_eq!(variant_name, "Left");
    *variant_index = 1;

    let error = execute(&analyzed.program).expect_err("retargeted constructor must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_exhaustive_same_shape_match_arm_swap() {
    let mut analyzed = analyze_text(
        "enum Flag { Off, On, } fn main() -> Int { match Flag::Off { Flag::Off => 1, Flag::On => 2, } }",
    );
    let tail = analyzed.program.functions[0].body.tail.as_deref_mut().expect("match tail");
    let ExpressionKind::Match { arms, .. } = &mut tail.kind else { panic!("match HIR"); };
    arms[0].variant_index = 1;
    arms[1].variant_index = 0;

    let error = execute(&analyzed.program).expect_err("retargeted patterns must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn constructor_payload_return_precedes_malformed_variant_identity() {
    let mut analyzed = analyze_text(
        "enum Choice { Left(Int), Right(Int), } fn main() -> Int { Choice::Left({ return 9; 0 }); 0 }",
    );
    let StatementKind::Expression(expression) = &mut analyzed.program.functions[0].body.statements[0].kind else { panic!("constructor statement"); };
    let ExpressionKind::EnumConstructor { variant_index, .. } = &mut expression.kind else { panic!("constructor HIR"); };
    *variant_index = 99;

    let value = execute(&analyzed.program).expect("structured return must win before value-only identity validation");
    assert_eq!(value, Value::Int(9));
}

#[test]
fn match_scrutinee_return_precedes_malformed_arm_identity() {
    let mut analyzed = analyze_text(
        "enum Flag { Off, On, } fn main() -> Int { match { return 8; Flag::Off } { Flag::Off => 1, Flag::On => 2, } }",
    );
    let tail = analyzed.program.functions[0].body.tail.as_deref_mut().expect("match tail");
    let ExpressionKind::Match { arms, .. } = &mut tail.kind else { panic!("match HIR"); };
    arms[0].variant_index = 99;

    let value = execute(&analyzed.program).expect("scrutinee return must win before value-only arm validation");
    assert_eq!(value, Value::Int(8));
}
''')

# Inspection corruption regressions; schema surface remains unchanged.
Path("crates/nova-inspect/tests").mkdir(parents=True, exist_ok=True)
Path("crates/nova-inspect/tests/enum_variant_identity.rs").write_text(r'''use nova_inspect::build_document;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn checked(text: &str) -> (SourceFile, nova_sema::hir::Program) {
    let source = SourceFile::new(SourceId::new(0), "enum-variant-inspect.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed.program)
}

#[test]
fn rejects_constructor_variant_name_slot_drift() {
    let (source, mut program) = checked(
        "enum Choice { Left(Int), Right(Int), } fn main() -> Int { match Choice::Left(7) { Choice::Left(value) => value, Choice::Right(value) => 0, } }",
    );
    let tail = program.functions[0].body.tail.as_deref_mut().expect("match tail");
    let ExpressionKind::Match { scrutinee, .. } = &mut tail.kind else { panic!("match HIR"); };
    let ExpressionKind::EnumConstructor { variant_index, .. } = &mut scrutinee.kind else { panic!("constructor HIR"); };
    *variant_index = 1;

    let error = build_document(&program, &source).expect_err("variant drift must fail closed");
    assert!(error.message().contains("enum construction variant"));
}

#[test]
fn rejects_match_arm_variant_name_slot_drift() {
    let (source, mut program) = checked(
        "enum Flag { Off, On, } fn main() -> Int { match Flag::Off { Flag::Off => 1, Flag::On => 2, } }",
    );
    let tail = program.functions[0].body.tail.as_deref_mut().expect("match tail");
    let ExpressionKind::Match { arms, .. } = &mut tail.kind else { panic!("match HIR"); };
    arms[0].variant_index = 1;
    arms[1].variant_index = 0;

    let error = build_document(&program, &source).expect_err("pattern drift must fail closed");
    assert!(error.message().contains("match variant"));
}
''')
