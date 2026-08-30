from pathlib import Path

# Preserve the existing enum-identity adversarial tests against the new pattern shape.
p = Path('crates/nova-interpreter/tests/enum_variant_identity.rs')
text = p.read_text()
text = text.replace(
    '    hir::{ExpressionKind, StatementKind},\n',
    '    hir::{ExpressionKind, MatchPattern, StatementKind},\n',
)
text = text.replace(
    '''    arms[0].variant_index = 1;\n    arms[1].variant_index = 0;\n''',
    '''    let MatchPattern::Variant { variant_index, .. } = &mut arms[0].pattern else {\n        panic!("concrete arm pattern");\n    };\n    *variant_index = 1;\n    let MatchPattern::Variant { variant_index, .. } = &mut arms[1].pattern else {\n        panic!("concrete arm pattern");\n    };\n    *variant_index = 0;\n''',
)
text = text.replace(
    '''    arms[0].variant_index = 99;\n\n    let value = execute(&analyzed.program)\n''',
    '''    let MatchPattern::Variant { variant_index, .. } = &mut arms[0].pattern else {\n        panic!("concrete arm pattern");\n    };\n    *variant_index = 99;\n\n    let value = execute(&analyzed.program)\n''',
)
p.write_text(text)

# Preserve the payload-discard corruption proof: remove only the real concrete binding
# while leaving explicit discard intent false.
p = Path('crates/nova-interpreter/tests/pattern_payload_discard.rs')
text = p.read_text()
text = text.replace(
    'use nova_sema::{analyze, hir::ExpressionKind};\n',
    'use nova_sema::{analyze, hir::{ExpressionKind, MatchPattern}};\n',
)
text = text.replace(
    '''    arms[1].binding = None;\n    assert!(!arms[1].payload_discarded);\n''',
    '''    let MatchPattern::Variant {\n        binding,\n        payload_discarded,\n        ..\n    } = &mut arms[1].pattern\n    else {\n        panic!("concrete arm pattern");\n    };\n    *binding = None;\n    assert!(!*payload_discarded);\n''',
)
p.write_text(text)

# Preserve the frame-slot corruption proof by reaching the payload binding through the
# concrete pattern instead of the old flat MatchArm field.
p = Path('crates/nova-interpreter/tests/frame_slot_integrity.rs')
text = p.read_text()
text = text.replace(
    '    hir::{BindingReference, ExpressionKind, StatementKind, Type},\n',
    '    hir::{BindingReference, ExpressionKind, MatchPattern, StatementKind, Type},\n',
)
text = text.replace(
    '''    let binding = arms[0].binding.as_mut().expect("payload binding");\n    binding.ty = Type::Bool;\n''',
    '''    let MatchPattern::Variant { binding, .. } = &mut arms[0].pattern else {\n        panic!("concrete arm pattern");\n    };\n    let binding = binding.as_mut().expect("payload binding");\n    binding.ty = Type::Bool;\n''',
)
p.write_text(text)

# Keep semantic-inspection v1/v2/v3 semantically frozen: adapt concrete HIR patterns,
# but fail closed if an accepted program contains the new catch-all until schema v4 exists.
p = Path('crates/nova-inspect/src/lib.rs')
text = p.read_text()
start = text.index('                for (arm_index, arm) in arms.iter().enumerate() {')
end = text.index('                if seen.len() != declaration.variants.len() {', start)
replacement = r'''                for (arm_index, arm) in arms.iter().enumerate() {
                    let hir::MatchPattern::Variant {
                        variant_name,
                        variant_index,
                        binding: arm_binding,
                        payload_discarded,
                    } = &arm.pattern
                    else {
                        return Err(InspectionError::invalid(
                            "semantic-inspection schema v1/v2/v3 cannot represent a catch-all wildcard arm; select schema v4",
                        ));
                    };
                    let variant = declaration.variants.get(*variant_index).ok_or_else(|| {
                        InspectionError::invalid(format!(
                            "match references out-of-range variant slot {variant_index}"
                        ))
                    })?;
                    if variant.name != *variant_name {
                        return Err(InspectionError::invalid(format!(
                            "match variant `{variant_name}` does not match slot {variant_index} declaration `{}`",
                            variant.name
                        )));
                    }
                    if !seen.insert(*variant_index) {
                        return Err(InspectionError::invalid(format!(
                            "match repeats variant slot {variant_index}"
                        )));
                    }
                    let arm_identity = match_arm_id(match_index, arm_index);
                    self.active_scopes.push(arm_identity.clone());
                    let arm_contents = (|| {
                        let (binding, payload_mode) = match (
                            &variant.payload,
                            arm_binding,
                            *payload_discarded,
                        ) {
                            (Some(expected), Some(binding), false) => {
                                if &binding.ty != expected {
                                    return Err(InspectionError::invalid(format!(
                                        "match payload binding type does not match {}",
                                        variant_id(enumeration.index(), *variant_index)
                                    )));
                                }
                                (
                                    Some(self.add_binding(
                                        binding,
                                        v1::BindingRole::MatchPayload,
                                        owner,
                                        &arm_identity,
                                    )?),
                                    v3::MatchPayloadMode::Bind,
                                )
                            }
                            (Some(_), None, true) if self.allow_payload_discard => {
                                (None, v3::MatchPayloadMode::Discard)
                            }
                            (Some(_), None, true) => {
                                return Err(InspectionError::invalid(
                                    "semantic-inspection schema v1/v2 cannot represent an explicitly discarded enum payload; select schema v3",
                                ));
                            }
                            (None, None, false) => (None, v3::MatchPayloadMode::None),
                            _ => {
                                return Err(InspectionError::invalid(format!(
                                    "match payload mode does not match {}",
                                    variant_id(enumeration.index(), *variant_index)
                                )));
                            }
                        };
                        let value = self.collect_expression(&arm.value, owner)?;
                        Ok::<_, InspectionError>((binding, value, payload_mode))
                    })();
                    self.active_scopes.pop();
                    let (binding, value, payload_mode) = arm_contents?;
                    self.match_patterns.push(v3::MatchPattern {
                        arm: arm_identity.clone(),
                        payload_mode,
                    });
                    children.push(value.clone());
                    arm_facts.push(v1::MatchArm {
                        id: arm_identity,
                        variant: variant_id(enumeration.index(), *variant_index),
                        binding,
                        value,
                        result_type: self.intern_type(&arm.value.ty)?,
                        span: self.span(arm.span)?,
                    });
                }
'''
p.write_text(text[:start] + replacement + text[end:])
