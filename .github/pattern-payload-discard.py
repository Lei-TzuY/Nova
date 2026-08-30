from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    if text.count(old) != 1:
        raise RuntimeError(f"expected one anchor in {path}, found {text.count(old)}")
    p.write_text(text.replace(old, new, 1))

# Parser AST: preserve whether a payload position explicitly discarded its value.
replace_once(
    "crates/nova-parser/src/ast.rs",
    '''pub struct EnumPattern {\n    /// Enum type qualifier.\n    pub enumeration: Name,\n    /// Selected variant.\n    pub variant: Name,\n    /// Optional immutable binding for the variant payload.\n    pub binding: Option<Name>,\n    /// Complete pattern range.\n    pub span: Span,\n}\n''',
    '''pub struct EnumPattern {\n    /// Enum type qualifier.\n    pub enumeration: Name,\n    /// Selected variant.\n    pub variant: Name,\n    /// Optional immutable binding for the variant payload.\n    pub binding: Option<Name>,\n    /// Whether the payload position was explicitly discarded with `_`.\n    pub payload_discarded: bool,\n    /// Complete pattern range.\n    pub span: Span,\n}\n''',
)

# Parser: `_` is special only in the payload slot of a qualified enum pattern.
replace_once(
    "crates/nova-parser/src/parser.rs",
    '''        let variant = self.parse_name("after `::` in a pattern")?;\n        let (binding, end) = if self.consume(TokenKind::LeftParen).is_some() {\n            let binding = self.parse_name("as the variant payload binding")?;\n            let closing = self.expect(TokenKind::RightParen, "after the payload binding")?;\n            (Some(binding), closing.span)\n        } else {\n            (None, variant.span)\n        };\n        Some(EnumPattern {\n            span: self.cover(enumeration.span, end),\n            enumeration,\n            variant,\n            binding,\n        })\n''',
    '''        let variant = self.parse_name("after `::` in a pattern")?;\n        let (binding, payload_discarded, end) =\n            if self.consume(TokenKind::LeftParen).is_some() {\n                let payload = self.parse_name("as the variant payload binding or `_`")?;\n                let payload_discarded = payload.text == "_";\n                let binding = if payload_discarded { None } else { Some(payload) };\n                let closing = self.expect(TokenKind::RightParen, "after the payload pattern")?;\n                (binding, payload_discarded, closing.span)\n            } else {\n                (None, false, variant.span)\n            };\n        Some(EnumPattern {\n            span: self.cover(enumeration.span, end),\n            enumeration,\n            variant,\n            binding,\n            payload_discarded,\n        })\n''',
)

# HIR: retain discard intent so later trust boundaries cannot confuse a removed binding with `_`.
replace_once(
    "crates/nova-sema/src/hir.rs",
    '''    /// Optional immutable payload binding.\n    pub binding: Option<Binding>,\n    /// Typed arm value.\n''',
    '''    /// Optional immutable payload binding.\n    pub binding: Option<Binding>,\n    /// Whether a payload-bearing variant explicitly discards its payload with `_`.\n    pub payload_discarded: bool,\n    /// Typed arm value.\n''',
)

# Semantic lowering: payload-bearing variants may bind OR explicitly discard, never silently omit.
replace_once(
    "crates/nova-sema/src/analyzer.rs",
    '''                    match (&declared.payload, &arm.pattern.binding) {\n                        (Some(payload_type), Some(binding_name)) => {\n                            let binding =\n                                self.new_binding(binding_name, payload_type.clone(), false);\n                            self.insert_local(&binding);\n                            self.record_initialization(binding.id, binding.span);\n                            payload_binding = Some(binding);\n                        }\n                        (None, None) => {}\n                        (Some(_), None) => {\n                            self.diagnostics.push(\n                                Diagnostic::error("N3022", "missing pattern payload binding")\n                                    .with_primary(\n                                        arm.pattern.variant.span,\n                                        format!("variant `{}` carries one payload", declared.name),\n                                    )\n                                    .with_secondary(\n                                        declared.span,\n                                        "variant declared with a payload here",\n                                    ),\n                            );\n                            valid_pattern = false;\n                        }\n                        (None, Some(binding_name)) => {\n                            self.diagnostics.push(\n                                Diagnostic::error("N3022", "unexpected pattern payload binding")\n                                    .with_primary(\n                                        binding_name.span,\n                                        format!(\n                                            "variant `{}` does not carry a payload",\n                                            declared.name\n                                        ),\n                                    )\n                                    .with_secondary(\n                                        declared.span,\n                                        "payload-free variant declared here",\n                                    ),\n                            );\n                            valid_pattern = false;\n                        }\n                    }\n''',
    '''                    match (\n                        &declared.payload,\n                        &arm.pattern.binding,\n                        arm.pattern.payload_discarded,\n                    ) {\n                        (Some(payload_type), Some(binding_name), false) => {\n                            let binding =\n                                self.new_binding(binding_name, payload_type.clone(), false);\n                            self.insert_local(&binding);\n                            self.record_initialization(binding.id, binding.span);\n                            payload_binding = Some(binding);\n                        }\n                        (Some(_), None, true) | (None, None, false) => {}\n                        (Some(_), None, false) => {\n                            self.diagnostics.push(\n                                Diagnostic::error("N3022", "missing pattern payload binding")\n                                    .with_primary(\n                                        arm.pattern.variant.span,\n                                        format!(\n                                            "variant `{}` carries one payload; bind it or write `_` to discard it",\n                                            declared.name\n                                        ),\n                                    )\n                                    .with_secondary(\n                                        declared.span,\n                                        "variant declared with a payload here",\n                                    ),\n                            );\n                            valid_pattern = false;\n                        }\n                        (None, None, true) => {\n                            self.diagnostics.push(\n                                Diagnostic::error("N3022", "unexpected pattern payload discard")\n                                    .with_primary(\n                                        arm.pattern.span,\n                                        format!(\n                                            "variant `{}` has no payload to discard",\n                                            declared.name\n                                        ),\n                                    )\n                                    .with_secondary(\n                                        declared.span,\n                                        "payload-free variant declared here",\n                                    ),\n                            );\n                            valid_pattern = false;\n                        }\n                        (None, Some(binding_name), false) => {\n                            self.diagnostics.push(\n                                Diagnostic::error("N3022", "unexpected pattern payload binding")\n                                    .with_primary(\n                                        binding_name.span,\n                                        format!(\n                                            "variant `{}` does not carry a payload",\n                                            declared.name\n                                        ),\n                                    )\n                                    .with_secondary(\n                                        declared.span,\n                                        "payload-free variant declared here",\n                                    ),\n                            );\n                            valid_pattern = false;\n                        }\n                        (_, Some(binding_name), true) => {\n                            self.diagnostics.push(\n                                Diagnostic::error("N3022", "invalid pattern payload")\n                                    .with_primary(\n                                        binding_name.span,\n                                        "a payload pattern cannot both bind and discard",\n                                    ),\n                            );\n                            valid_pattern = false;\n                        }\n                    }\n''',
)
replace_once(
    "crates/nova-sema/src/analyzer.rs",
    '''                    binding: payload_binding,\n                    value,\n''',
    '''                    binding: payload_binding,\n                    payload_discarded: arm.pattern.payload_discarded,\n                    value,\n''',
)

# Runtime: validate discard metadata independently, then drop the selected payload without binding it.
replace_once(
    "crates/nova-interpreter/src/lib.rs",
    '''                    if declared.payload.is_some() != arm.binding.is_some() {\n                        return Err(self.invariant(\n                            arm.span,\n                            "resolved match binding arity does not match its variant",\n                        ));\n                    }\n''',
    '''                    let payload_shape_valid = matches!(\n                        (\n                            declared.payload.is_some(),\n                            arm.binding.is_some(),\n                            arm.payload_discarded,\n                        ),\n                        (true, true, false) | (true, false, true) | (false, false, false)\n                    );\n                    if !payload_shape_valid {\n                        return Err(self.invariant(\n                            arm.span,\n                            "resolved match payload mode does not match its variant",\n                        ));\n                    }\n''',
)
replace_once(
    "crates/nova-interpreter/src/lib.rs",
    '''                match (&arm.binding, payload) {\n                    (Some(binding), Some(payload)) => {\n                        self.bind_runtime_slot(frame, binding, Some(*payload), arm.span)?;\n                    }\n                    (None, None) => {}\n                    _ => {\n                        return Err(self.invariant(\n                            arm.span,\n                            "runtime enum payload arity does not match the selected arm",\n                        ));\n                    }\n                }\n''',
    '''                match (&arm.binding, arm.payload_discarded, payload) {\n                    (Some(binding), false, Some(payload)) => {\n                        self.bind_runtime_slot(frame, binding, Some(*payload), arm.span)?;\n                    }\n                    (None, true, Some(_)) | (None, false, None) => {}\n                    _ => {\n                        return Err(self.invariant(\n                            arm.span,\n                            "runtime enum payload mode does not match the selected arm",\n                        ));\n                    }\n                }\n''',
)

# Semantic inspection: schema stays unchanged; a payload variant with binding=null means explicit discard.
replace_once(
    "crates/nova-inspect/src/lib.rs",
    '''                        let binding = match (&variant.payload, &arm.binding) {\n                            (Some(expected), Some(binding)) => {\n                                if &binding.ty != expected {\n                                    return Err(InspectionError::invalid(format!(\n                                        "match payload binding type does not match {}",\n                                        variant_id(enumeration.index(), arm.variant_index)\n                                    )));\n                                }\n                                Some(self.add_binding(\n                                    binding,\n                                    v1::BindingRole::MatchPayload,\n                                    owner,\n                                    &arm_identity,\n                                )?)\n                            }\n                            (None, None) => None,\n                            _ => {\n                                return Err(InspectionError::invalid(format!(\n                                    "match payload binding arity does not match {}",\n                                    variant_id(enumeration.index(), arm.variant_index)\n                                )));\n                            }\n                        };\n''',
    '''                        let binding = match (\n                            &variant.payload,\n                            &arm.binding,\n                            arm.payload_discarded,\n                        ) {\n                            (Some(expected), Some(binding), false) => {\n                                if &binding.ty != expected {\n                                    return Err(InspectionError::invalid(format!(\n                                        "match payload binding type does not match {}",\n                                        variant_id(enumeration.index(), arm.variant_index)\n                                    )));\n                                }\n                                Some(self.add_binding(\n                                    binding,\n                                    v1::BindingRole::MatchPayload,\n                                    owner,\n                                    &arm_identity,\n                                )?)\n                            }\n                            (Some(_), None, true) | (None, None, false) => None,\n                            _ => {\n                                return Err(InspectionError::invalid(format!(\n                                    "match payload mode does not match {}",\n                                    variant_id(enumeration.index(), arm.variant_index)\n                                )));\n                            }\n                        };\n''',
)

# Focused parser coverage.
Path("crates/nova-parser/tests/pattern_payload_discard.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::{ast::ExpressionKind, parse};
use nova_source::{SourceFile, SourceId};

#[test]
fn parses_underscore_as_an_explicit_payload_discard() {
    let source = SourceFile::new(
        SourceId::new(0),
        "pattern-payload-discard.nv",
        "enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(9) { Maybe::None => 0, Maybe::Some(_) => 1 } }",
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let tail = parsed.program.functions[0].body.tail.as_deref().expect("match tail");
    let ExpressionKind::Match { arms, .. } = &tail.kind else { panic!("expected match"); };
    assert!(!arms[0].pattern.payload_discarded);
    assert!(arms[0].pattern.binding.is_none());
    assert!(arms[1].pattern.payload_discarded);
    assert!(arms[1].pattern.binding.is_none());
    assert_eq!(source.slice(arms[1].pattern.span), Some("Maybe::Some(_)"));
}
''')

# Semantic producer + rejection coverage.
Path("crates/nova-sema/tests/pattern_payload_discard.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "pattern-payload-discard.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn hir_distinguishes_payload_discard_from_absent_payload() {
    let analyzed = analyze_text("enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(9) { Maybe::None => 0, Maybe::Some(_) => 42 } }");
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    let tail = analyzed.program.functions[0].body.tail.as_deref().expect("tail");
    let ExpressionKind::Match { arms, .. } = &tail.kind else { panic!("expected match"); };
    assert!(!arms[0].payload_discarded);
    assert!(arms[0].binding.is_none());
    assert!(arms[1].payload_discarded);
    assert!(arms[1].binding.is_none());
}

#[test]
fn payload_free_variant_cannot_use_discard_syntax() {
    let analyzed = analyze_text("enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::None { Maybe::None(_) => 0, Maybe::Some(value) => value } }");
    assert!(analyzed.diagnostics.iter().any(|diagnostic| diagnostic.code == "N3022" && diagnostic.message == "unexpected pattern payload discard"), "{:?}", analyzed.diagnostics);
}
''')

# Runtime execution + malformed-HIR proof.
Path("crates/nova-interpreter/tests/pattern_payload_discard.rs").write_text(r'''use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn accepted(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "pattern-payload-discard.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn discarded_payload_is_not_bound_and_selected_arm_executes() {
    let analyzed = accepted("enum Packet { Empty, Data(Int) } fn score(packet: Packet) -> Int { match packet { Packet::Empty => 0, Packet::Data(_) => 42 } } fn main() -> Int { score(Packet::Data(99)) }");
    assert_eq!(execute(&analyzed.program), Ok(Value::Int(42)));
}

#[test]
fn deleting_a_real_payload_binding_is_not_reinterpreted_as_discard() {
    let mut analyzed = accepted("enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(7) { Maybe::None => 0, Maybe::Some(value) => value } }");
    let tail = analyzed.program.functions[0].body.tail.as_deref_mut().expect("tail");
    let ExpressionKind::Match { arms, .. } = &mut tail.kind else { panic!("expected match"); };
    arms[1].binding = None;
    assert!(!arms[1].payload_discarded);
    let diagnostic = execute(&analyzed.program).expect_err("malformed HIR must fail");
    assert_eq!(diagnostic.code, "N4005");
}
''')

# Inspection projection + malformed-HIR proof; v1/v2 schema shape remains unchanged.
Path("crates/nova-inspect/tests/pattern_payload_discard.rs").write_text(r'''use nova_inspect::build_document;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn accepted(text: &str) -> (SourceFile, nova_sema::AnalysisOutput) {
    let source = SourceFile::new(SourceId::new(0), "pattern-payload-discard.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed)
}

#[test]
fn schema_v1_projects_discard_as_payload_variant_without_a_binding() {
    let (source, analyzed) = accepted("enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(7) { Maybe::None => 0, Maybe::Some(_) => 1 } }");
    let document = build_document(&analyzed.program, &source).expect("inspection must succeed");
    assert!(document.program.matches[0].arms[1].binding.is_none());
    assert!(document.program.matches[0].arms[1].variant.ends_with("variant:1"));
}

#[test]
fn inspector_rejects_discard_metadata_removed_from_payload_variant() {
    let (source, mut analyzed) = accepted("enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(7) { Maybe::None => 0, Maybe::Some(_) => 1 } }");
    let tail = analyzed.program.functions[0].body.tail.as_deref_mut().expect("tail");
    let ExpressionKind::Match { arms, .. } = &mut tail.kind else { panic!("expected match"); };
    arms[1].payload_discarded = false;
    let error = build_document(&analyzed.program, &source).expect_err("malformed HIR must fail");
    assert!(error.message().contains("match payload mode does not match"), "{}", error.message());
}
''')
