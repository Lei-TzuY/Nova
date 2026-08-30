from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected one anchor in {path}, found {count}: {old[:120]!r}")
    p.write_text(text.replace(old, new, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    p = Path(path)
    text = p.read_text()
    a = text.find(start)
    if a < 0:
        raise RuntimeError(f"missing start marker in {path}: {start!r}")
    b = text.find(end, a)
    if b < 0:
        raise RuntimeError(f"missing end marker in {path}: {end!r}")
    p.write_text(text[:a] + replacement + text[b:])


# Parser AST: make match patterns explicit so bare `_` is not confused with payload discard.
replace_once(
    "crates/nova-parser/src/ast.rs",
    '''/// One arm in a `match` expression.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct MatchArm {\n    /// Variant pattern selecting this arm.\n    pub pattern: EnumPattern,\n''',
    '''/// Surface pattern accepted by one `match` arm.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub enum MatchPattern {\n    /// Qualified concrete enum variant pattern.\n    Variant(EnumPattern),\n    /// Bare catch-all pattern written `_`.\n    Wildcard {\n        /// Exact underscore range.\n        span: Span,\n    },\n}\n\n/// One arm in a `match` expression.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct MatchArm {\n    /// Pattern selecting this arm.\n    pub pattern: MatchPattern,\n''',
)

replace_once(
    "crates/nova-parser/src/parser.rs",
    '''    BinaryOperator, Block, Enum, EnumPattern, EnumVariant, Expression, ExpressionKind, Function,\n    MatchArm, Name, Parameter, Program, Record, RecordField, RecordLiteralField, Statement,\n''',
    '''    BinaryOperator, Block, Enum, EnumPattern, EnumVariant, Expression, ExpressionKind, Function,\n    MatchArm, MatchPattern, Name, Parameter, Program, Record, RecordField, RecordLiteralField,\n    Statement,\n''',
)
replace_once(
    "crates/nova-parser/src/parser.rs",
    '''            let pattern = self.parse_enum_pattern()?;\n            self.expect(TokenKind::FatArrow, "after the match pattern")?;\n            let value = self.parse_expression_with_binding_power(0)?;\n            let span = self.cover(pattern.span, value.span);\n''',
    '''            let pattern = self.parse_match_pattern()?;\n            self.expect(TokenKind::FatArrow, "after the match pattern")?;\n            let value = self.parse_expression_with_binding_power(0)?;\n            let pattern_span = match &pattern {\n                MatchPattern::Variant(pattern) => pattern.span,\n                MatchPattern::Wildcard { span } => *span,\n            };\n            let span = self.cover(pattern_span, value.span);\n''',
)
replace_once(
    "crates/nova-parser/src/parser.rs",
    '''    fn parse_enum_pattern(&mut self) -> Option<EnumPattern> {\n''',
    '''    fn parse_match_pattern(&mut self) -> Option<MatchPattern> {\n        if self.at(TokenKind::Identifier)\n            && self.source.slice(self.current().span) == Some("_")\n        {\n            let wildcard = self.bump();\n            return Some(MatchPattern::Wildcard { span: wildcard.span });\n        }\n        self.parse_enum_pattern().map(MatchPattern::Variant)\n    }\n\n    fn parse_enum_pattern(&mut self) -> Option<EnumPattern> {\n''',
)

# HIR: retain wildcard versus concrete variant identity without synthetic slots.
replace_once(
    "crates/nova-sema/src/hir.rs",
    '''/// One resolved arm in an exhaustive enum match.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct MatchArm {\n    /// Resolved variant spelling paired with the declaration-order slot.\n    pub variant_name: String,\n    /// Zero-based variant slot in declaration order.\n    pub variant_index: usize,\n    /// Optional immutable payload binding.\n    pub binding: Option<Binding>,\n    /// Whether a payload-bearing variant explicitly discards its payload with `_`.\n    pub payload_discarded: bool,\n    /// Typed arm value.\n    pub value: Expression,\n    /// Complete arm range.\n    pub span: Span,\n}\n''',
    '''/// Resolved pattern carried by one match arm.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub enum MatchArmPattern {\n    /// Source-resolved concrete enum variant.\n    Variant {\n        /// Resolved variant spelling paired with the declaration-order slot.\n        variant_name: String,\n        /// Zero-based variant slot in declaration order.\n        variant_index: usize,\n        /// Optional immutable payload binding.\n        binding: Option<Binding>,\n        /// Whether a payload-bearing variant explicitly discards its payload with `_`.\n        payload_discarded: bool,\n    },\n    /// Bare source-level catch-all pattern.\n    Wildcard,\n}\n\n/// One resolved arm in an exhaustive enum match.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct MatchArm {\n    /// Resolved source pattern, preserving concrete versus wildcard identity.\n    pub pattern: MatchArmPattern,\n    /// Typed arm value.\n    pub value: Expression,\n    /// Complete arm range.\n    pub span: Span,\n}\n''',
)

replace_once(
    "crates/nova-sema/src/analyzer.rs",
    '''    self, BindingId, EnumId, EnumType, ExpressionKind, FunctionId, FunctionType, MatchArm,\n    RecordFieldValue, RecordId, RecordType, StatementKind, Type,\n''',
    '''    self, BindingId, EnumId, EnumType, ExpressionKind, FunctionId, FunctionType, MatchArm,\n    MatchArmPattern, RecordFieldValue, RecordId, RecordType, StatementKind, Type,\n''',
)

lower_match = r'''    fn lower_match(
        &mut self,
        scrutinee: &ast::Expression,
        arms: &[ast::MatchArm],
        return_type: &Type,
        span: Span,
    ) -> (ExpressionKind, Type) {
        let scrutinee = self.lower_expression(scrutinee, return_type);
        let selected_variant_index = match (&scrutinee.kind, &scrutinee.ty) {
            (
                ExpressionKind::EnumConstructor {
                    enumeration,
                    variant_index,
                    ..
                },
                Type::Enum(scrutinee_type),
            ) if *enumeration == scrutinee_type.id => Some(*variant_index),
            _ => None,
        };
        let post_scrutinee_loop_stack = self.loop_stack.clone();
        let mut scrutinee_enum = match &scrutinee.ty {
            Type::Enum(enumeration) => Some(enumeration.clone()),
            Type::Error | Type::Never => None,
            actual => {
                self.diagnostics.push(
                    Diagnostic::error("N3025", "match requires an enum value").with_primary(
                        scrutinee.span,
                        format!("cannot match a value of type {actual}"),
                    ),
                );
                None
            }
        };
        let entry_state = self.capture_scope_flow_state();
        let mut seen = BTreeMap::<usize, Span>::new();
        let mut wildcard_span = None::<Span>;
        let mut lowered_arms = Vec::with_capacity(arms.len());
        let mut branch_states = Vec::with_capacity(arms.len());
        let mut branch_types = Vec::with_capacity(arms.len());
        let mut selected_branch = None::<(ScopeFlowState, Type)>;
        let mut selected_claimed = false;
        let mut structural_error = scrutinee_enum.is_none() && !scrutinee.ty.is_never();

        for arm in arms {
            self.restore_scope_flow_state(&entry_state);
            let (pattern_span, concrete_pattern) = match &arm.pattern {
                ast::MatchPattern::Variant(pattern) => (pattern.span, Some(pattern)),
                ast::MatchPattern::Wildcard { span } => (*span, None),
            };
            let shadowed_by_wildcard = wildcard_span.is_some();
            let wildcard_covers_nothing = concrete_pattern.is_none()
                && !shadowed_by_wildcard
                && scrutinee_enum.as_ref().is_some_and(|enumeration| {
                    seen.len()
                        == self.enum_definitions[enumeration.id.index()]
                            .variants
                            .len()
                });
            let generally_useful = !shadowed_by_wildcard && !wildcard_covers_nothing;
            let resolves_to_selected = selected_variant_index.is_some_and(|selected| {
                if concrete_pattern.is_none() {
                    true
                } else {
                    let pattern = concrete_pattern.expect("checked concrete pattern");
                    scrutinee_enum.as_ref().is_some_and(|enumeration| {
                        pattern.enumeration.text == enumeration.name
                            && self.enum_definitions[enumeration.id.index()]
                                .variants
                                .iter()
                                .position(|variant| variant.name == pattern.variant.text)
                                == Some(selected)
                    })
                }
            });
            let arm_edge = if scrutinee.ty.is_never() {
                FlowEdgeKind::Diagnostic
            } else if selected_variant_index.is_some() {
                if generally_useful && resolves_to_selected && !selected_claimed {
                    FlowEdgeKind::Execution
                } else {
                    FlowEdgeKind::Diagnostic
                }
            } else if matches!(scrutinee.ty, Type::Enum(_)) && generally_useful {
                FlowEdgeKind::Execution
            } else {
                FlowEdgeKind::Diagnostic
            };
            self.flow_fork_from(entry_state.flow_cursor, Some(arm.span), arm_edge);
            self.scopes.push(BTreeMap::new());
            let mut valid_pattern = true;
            let mut resolved_index = None;
            let mut payload_binding = None;

            if let Some(pattern) = concrete_pattern {
                let symbol = self.types.get(&pattern.enumeration.text).copied();
                let pattern_enum_id = match symbol {
                    Some(TypeSymbol {
                        definition: TypeDefinition::Enum(id),
                        ..
                    }) => Some(id),
                    Some(symbol) => {
                        self.diagnostics.push(
                            Diagnostic::error("N3021", "invalid enum pattern")
                                .with_primary(
                                    pattern.enumeration.span,
                                    format!(
                                        "`{}` is a record, not an enum",
                                        pattern.enumeration.text
                                    ),
                                )
                                .with_secondary(symbol.span, "record declared here"),
                        );
                        valid_pattern = false;
                        None
                    }
                    None => {
                        self.diagnostics
                            .push(Diagnostic::error("N3021", "unknown enum").with_primary(
                                pattern.enumeration.span,
                                format!("cannot resolve enum `{}`", pattern.enumeration.text),
                            ));
                        valid_pattern = false;
                        None
                    }
                };

                if let Some(pattern_enum_id) = pattern_enum_id {
                    let definition = self.enum_definitions[pattern_enum_id.index()].clone();
                    if scrutinee.ty.is_never() && scrutinee_enum.is_none() {
                        scrutinee_enum = Some(definition.enum_type());
                    }
                    if let Some(expected) = &scrutinee_enum {
                        if expected.id != pattern_enum_id {
                            self.diagnostics.push(
                                Diagnostic::error("N3025", "pattern enum does not match scrutinee")
                                    .with_primary(
                                        pattern.enumeration.span,
                                        format!(
                                            "pattern names `{}`, but the scrutinee has type {}",
                                            pattern.enumeration.text, expected.name
                                        ),
                                    ),
                            );
                            valid_pattern = false;
                        }
                    }

                    if let Some(variant_index) = definition
                        .variants
                        .iter()
                        .position(|declared| declared.name == pattern.variant.text)
                    {
                        let declared = &definition.variants[variant_index];
                        resolved_index = Some(variant_index);
                        match (&declared.payload, &pattern.binding, pattern.payload_discarded) {
                            (Some(payload_type), Some(binding_name), false) => {
                                let binding =
                                    self.new_binding(binding_name, payload_type.clone(), false);
                                self.insert_local(&binding);
                                self.record_initialization(binding.id, binding.span);
                                payload_binding = Some(binding);
                            }
                            (Some(_), None, true) | (None, None, false) => {}
                            (Some(_), None, false) => {
                                self.diagnostics.push(
                                    Diagnostic::error("N3022", "missing pattern payload binding")
                                        .with_primary(
                                            pattern.variant.span,
                                            format!(
                                                "variant `{}` carries one payload; bind it or write `_` to discard it",
                                                declared.name
                                            ),
                                        )
                                        .with_secondary(
                                            declared.span,
                                            "variant declared with a payload here",
                                        ),
                                );
                                valid_pattern = false;
                            }
                            (None, None, true) => {
                                self.diagnostics.push(
                                    Diagnostic::error("N3022", "unexpected pattern payload discard")
                                        .with_primary(
                                            pattern.span,
                                            format!(
                                                "variant `{}` has no payload to discard",
                                                declared.name
                                            ),
                                        )
                                        .with_secondary(
                                            declared.span,
                                            "payload-free variant declared here",
                                        ),
                                );
                                valid_pattern = false;
                            }
                            (None, Some(binding_name), false) => {
                                self.diagnostics.push(
                                    Diagnostic::error("N3022", "unexpected pattern payload binding")
                                        .with_primary(
                                            binding_name.span,
                                            format!(
                                                "variant `{}` does not carry a payload",
                                                declared.name
                                            ),
                                        )
                                        .with_secondary(
                                            declared.span,
                                            "payload-free variant declared here",
                                        ),
                                );
                                valid_pattern = false;
                            }
                            (_, Some(binding_name), true) => {
                                self.diagnostics.push(
                                    Diagnostic::error("N3022", "invalid pattern payload")
                                        .with_primary(
                                            binding_name.span,
                                            "a payload pattern cannot both bind and discard",
                                        ),
                                );
                                valid_pattern = false;
                            }
                        }
                    } else {
                        self.diagnostics.push(
                            Diagnostic::error("N3021", "unknown enum variant")
                                .with_primary(
                                    pattern.variant.span,
                                    format!(
                                        "enum `{}` has no variant named `{}`",
                                        definition.name, pattern.variant.text
                                    ),
                                )
                                .with_secondary(definition.span, "enum declared here"),
                        );
                        valid_pattern = false;
                    }
                }

                if payload_binding.is_none() {
                    if let Some(binding_name) = &pattern.binding {
                        let binding = self.new_binding(binding_name, Type::Error, false);
                        self.insert_local(&binding);
                        self.record_initialization(binding.id, binding.span);
                        payload_binding = Some(binding);
                    }
                }

                if valid_pattern {
                    if let (Some(expected), Some(index)) = (&scrutinee_enum, resolved_index) {
                        if let Some(previous) = seen.get(&index).copied() {
                            self.diagnostics.push(
                                Diagnostic::error("N3024", "duplicate match variant")
                                    .with_primary(
                                        pattern.span,
                                        format!(
                                            "variant `{}::{}` is matched more than once",
                                            expected.name, pattern.variant.text
                                        ),
                                    )
                                    .with_secondary(previous, "first matching arm is here"),
                            );
                            valid_pattern = false;
                        } else {
                            seen.insert(index, pattern.span);
                        }
                    }
                }
            }

            let pattern_matches_selected = selected_variant_index.is_some_and(|selected| {
                concrete_pattern
                    .map_or(true, |_| valid_pattern && resolved_index == Some(selected))
            });
            let selected_arm = selected_variant_index.is_some()
                && valid_pattern
                && generally_useful
                && pattern_matches_selected
                && !selected_claimed;
            if selected_arm {
                selected_claimed = true;
            }

            if self.diagnostic_only_depth == 0 && valid_pattern {
                if let Some(previous_wildcard) = wildcard_span {
                    self.deferred_warnings.push(
                        Diagnostic::warning("N3034", "statically unreachable match arm")
                            .with_primary(
                                pattern_span,
                                "this arm cannot be selected because an earlier `_` arm already covers every remaining variant",
                            )
                            .with_secondary(previous_wildcard, "catch-all arm is here")
                            .with_note(
                                "the arm remains name/type checked for deterministic diagnostics but contributes no reachable flow facts",
                            ),
                    );
                } else if wildcard_covers_nothing {
                    self.deferred_warnings.push(
                        Diagnostic::warning("N3034", "statically unreachable match arm")
                            .with_primary(
                                pattern_span,
                                "this `_` arm cannot be selected because earlier concrete arms already cover every enum variant",
                            )
                            .with_note(
                                "the arm remains type checked but contributes no reachable flow facts",
                            ),
                    );
                } else if let Some(selected) = selected_variant_index {
                    if !selected_arm {
                        if let Some(enumeration) = &scrutinee_enum {
                            let selected_name = &self.enum_definitions[enumeration.id.index()]
                                .variants[selected]
                                .name;
                            let description = match concrete_pattern {
                                Some(pattern) => format!(
                                    "this arm matches `{}::{}`, but this scrutinee can only select `{}::{selected_name}`",
                                    enumeration.name, pattern.variant.text, enumeration.name
                                ),
                                None => format!(
                                    "this catch-all arm is not reached because an earlier arm already selects `{}::{selected_name}`",
                                    enumeration.name
                                ),
                            };
                            self.deferred_warnings.push(
                                Diagnostic::warning("N3034", "statically unreachable match arm")
                                    .with_primary(pattern_span, description)
                                    .with_secondary(
                                        scrutinee.span,
                                        format!(
                                            "this direct constructor selects variant `{selected_name}`"
                                        ),
                                    )
                                    .with_note(
                                        "the arm remains name/type checked for deterministic diagnostics but contributes no reachable flow facts",
                                    ),
                            );
                        }
                    }
                }
            }

            let value = if scrutinee.ty.is_never()
                || (selected_variant_index.is_some() && !selected_arm)
                || (!selected_variant_index.is_some() && !generally_useful)
            {
                self.lower_expression_for_diagnostics(&arm.value, return_type)
            } else {
                self.lower_expression(&arm.value, return_type)
            };
            let popped = self.scopes.pop();
            debug_assert!(popped.is_some());
            let branch_state = (self.capture_scope_flow_state(), value.ty.is_never());
            if selected_arm {
                selected_branch = Some((branch_state.0.clone(), value.ty.clone()));
            }
            branch_states.push(branch_state);
            branch_types.push((value.ty.clone(), value.span));

            match (&arm.pattern, resolved_index) {
                (ast::MatchPattern::Variant(pattern), Some(variant_index)) => {
                    lowered_arms.push(MatchArm {
                        pattern: MatchArmPattern::Variant {
                            variant_name: pattern.variant.text.clone(),
                            variant_index,
                            binding: payload_binding,
                            payload_discarded: pattern.payload_discarded,
                        },
                        value,
                        span: arm.span,
                    });
                }
                (ast::MatchPattern::Wildcard { .. }, _) => {
                    lowered_arms.push(MatchArm {
                        pattern: MatchArmPattern::Wildcard,
                        value,
                        span: arm.span,
                    });
                    if valid_pattern && wildcard_span.is_none() {
                        wildcard_span = Some(pattern_span);
                    }
                }
                _ => {}
            }
            structural_error |= !valid_pattern;
        }

        if scrutinee.ty.is_never() && scrutinee_enum.is_none() {
            self.diagnostics.push(
                Diagnostic::error("N3025", "cannot determine matched enum").with_primary(
                    span,
                    "a match with a non-continuing scrutinee still needs at least one qualified variant arm; `_` carries no enum identity",
                ),
            );
            structural_error = true;
        }

        if wildcard_span.is_none() {
            if let Some(enumeration) = &scrutinee_enum {
                let definition = &self.enum_definitions[enumeration.id.index()];
                let missing = definition
                    .variants
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| !seen.contains_key(index))
                    .map(|(_, variant)| variant.name.as_str())
                    .collect::<Vec<_>>();
                if !missing.is_empty() {
                    self.diagnostics.push(
                        Diagnostic::error("N3023", "non-exhaustive match")
                            .with_primary(span, format!("missing variant(s): {}", missing.join(", ")))
                            .with_secondary(definition.span, "enum variants declared here"),
                    );
                    structural_error = true;
                }
            }
        }

        let joined_type = self.join_match_arm_types(&branch_types);
        let ty = if scrutinee.ty.is_never() {
            self.restore_scope_flow_state(&entry_state);
            self.loop_stack = post_scrutinee_loop_stack;
            Type::Never
        } else if structural_error {
            self.restore_scope_flow_state(&entry_state);
            self.loop_stack = post_scrutinee_loop_stack;
            Type::Error
        } else if let Some((selected_state, selected_type)) = selected_branch {
            self.restore_scope_flow_state(&selected_state);
            if joined_type.is_error() {
                Type::Error
            } else {
                selected_type
            }
        } else {
            self.merge_match_flow(&entry_state, &branch_states);
            joined_type
        };

        match (structural_error, scrutinee_enum) {
            (false, Some(enumeration)) => (
                ExpressionKind::Match {
                    scrutinee: Box::new(scrutinee),
                    enumeration: enumeration.id,
                    arms: lowered_arms,
                },
                ty,
            ),
            _ => (ExpressionKind::Error, ty),
        }
    }

'''
replace_between(
    "crates/nova-sema/src/analyzer.rs",
    "    fn lower_match(\n",
    "    fn lower_field_access(\n",
    lower_match,
)

# Runtime: validate source-order pattern table, allow wildcard exhaustiveness, and choose first match.
replace_once(
    "crates/nova-interpreter/src/lib.rs",
    '''    BindingId, BindingReference, Block, EnumId, Expression, ExpressionKind, Function, FunctionId,\n    Program, RecordId, Statement, StatementKind, Type,\n''',
    '''    BindingId, BindingReference, Block, EnumId, Expression, ExpressionKind, Function, FunctionId,\n    MatchArmPattern, Program, RecordId, Statement, StatementKind, Type,\n''',
)

runtime_match = r'''            ExpressionKind::Match {
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
                if definition.id != *enumeration {
                    return Err(self.invariant(
                        expression.span,
                        "enum declaration index does not match its resolved identity",
                    ));
                }
                let mut covered = vec![false; definition.variants.len()];
                let mut has_wildcard = false;
                for arm in arms {
                    match &arm.pattern {
                        MatchArmPattern::Variant {
                            variant_name,
                            variant_index,
                            binding,
                            payload_discarded,
                        } => {
                            let (_, declared) = self.resolved_enum_variant(
                                *enumeration,
                                *variant_index,
                                variant_name,
                                arm.span,
                            )?;
                            let Some(slot) = covered.get_mut(*variant_index) else {
                                return Err(self.invariant(
                                    arm.span,
                                    "resolved match arm targets a variant outside its enum",
                                ));
                            };
                            if *slot {
                                return Err(self.invariant(
                                    arm.span,
                                    "resolved match contains a duplicate concrete variant arm",
                                ));
                            }
                            *slot = true;
                            let payload_shape_valid = matches!(
                                (
                                    declared.payload.is_some(),
                                    binding.is_some(),
                                    *payload_discarded,
                                ),
                                (true, true, false)
                                    | (true, false, true)
                                    | (false, false, false)
                            );
                            if !payload_shape_valid {
                                return Err(self.invariant(
                                    arm.span,
                                    "resolved match payload mode does not match its variant",
                                ));
                            }
                        }
                        MatchArmPattern::Wildcard => has_wildcard = true,
                    }
                }
                if !has_wildcard && covered.iter().any(|covered| !covered) {
                    return Err(self.invariant(
                        expression.span,
                        "resolved match is not exhaustive for its enum declaration",
                    ));
                }

                let Some(arm) = arms.iter().find(|arm| match &arm.pattern {
                    MatchArmPattern::Variant {
                        variant_index: arm_variant,
                        ..
                    } => *arm_variant == variant_index,
                    MatchArmPattern::Wildcard => true,
                }) else {
                    return Err(self.invariant(
                        expression.span,
                        "exhaustive match has no arm for the runtime variant",
                    ));
                };
                match (&arm.pattern, payload) {
                    (
                        MatchArmPattern::Variant {
                            binding: Some(binding),
                            payload_discarded: false,
                            ..
                        },
                        Some(payload),
                    ) => {
                        self.bind_runtime_slot(frame, binding, Some(*payload), arm.span)?;
                    }
                    (
                        MatchArmPattern::Variant {
                            binding: None,
                            payload_discarded: true,
                            ..
                        },
                        Some(_),
                    )
                    | (
                        MatchArmPattern::Variant {
                            binding: None,
                            payload_discarded: false,
                            ..
                        },
                        None,
                    )
                    | (MatchArmPattern::Wildcard, _) => {}
                    _ => {
                        return Err(self.invariant(
                            arm.span,
                            "runtime enum payload mode does not match the selected arm",
                        ));
                    }
                }
                self.eval_expression(&arm.value, frame)
            }
'''
replace_between(
    "crates/nova-interpreter/src/lib.rs",
    "            ExpressionKind::Match {\n",
    "            ExpressionKind::Error =>",
    runtime_match + "            ExpressionKind::Error =>",
)

# Existing payload-discard tests now unwrap the explicit variant pattern.
replace_once(
    "crates/nova-parser/tests/pattern_payload_discard.rs",
    "use nova_parser::{ast::ExpressionKind, parse};\n",
    "use nova_parser::{ast::{ExpressionKind, MatchPattern}, parse};\n",
)
replace_once(
    "crates/nova-parser/tests/pattern_payload_discard.rs",
    '''    assert!(!arms[0].pattern.payload_discarded);\n    assert!(arms[0].pattern.binding.is_none());\n    assert!(arms[1].pattern.payload_discarded);\n    assert!(arms[1].pattern.binding.is_none());\n    assert_eq!(source.slice(arms[1].pattern.span), Some("Maybe::Some(_)"));\n''',
    '''    let MatchPattern::Variant(first) = &arms[0].pattern else { panic!("variant"); };\n    let MatchPattern::Variant(second) = &arms[1].pattern else { panic!("variant"); };\n    assert!(!first.payload_discarded);\n    assert!(first.binding.is_none());\n    assert!(second.payload_discarded);\n    assert!(second.binding.is_none());\n    assert_eq!(source.slice(second.span), Some("Maybe::Some(_)"));\n''',
)
replace_once(
    "crates/nova-sema/tests/pattern_payload_discard.rs",
    "use nova_sema::{analyze, hir::ExpressionKind};\n",
    "use nova_sema::{analyze, hir::{ExpressionKind, MatchArmPattern}};\n",
)
replace_once(
    "crates/nova-sema/tests/pattern_payload_discard.rs",
    '''    assert!(!arms[0].payload_discarded);\n    assert!(arms[0].binding.is_none());\n    assert!(arms[1].payload_discarded);\n    assert!(arms[1].binding.is_none());\n''',
    '''    let MatchArmPattern::Variant { binding: first_binding, payload_discarded: first_discarded, .. } = &arms[0].pattern else { panic!("variant"); };\n    let MatchArmPattern::Variant { binding: second_binding, payload_discarded: second_discarded, .. } = &arms[1].pattern else { panic!("variant"); };\n    assert!(!first_discarded);\n    assert!(first_binding.is_none());\n    assert!(*second_discarded);\n    assert!(second_binding.is_none());\n''',
)

Path("crates/nova-parser/tests/match_wildcard.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::{ast::{ExpressionKind, MatchPattern}, parse};
use nova_source::{SourceFile, SourceId};

#[test]
fn parses_bare_wildcard_separately_from_payload_discard() {
    let source = SourceFile::new(
        SourceId::new(0),
        "match-wildcard.nv",
        "enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(9) { Maybe::None => 0, _ => 1 } }",
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let tail = parsed.program.functions[0].body.tail.as_deref().expect("match tail");
    let ExpressionKind::Match { arms, .. } = &tail.kind else { panic!("expected match"); };
    assert!(matches!(arms[0].pattern, MatchPattern::Variant(_)));
    let MatchPattern::Wildcard { span } = arms[1].pattern else { panic!("wildcard"); };
    assert_eq!(source.slice(span), Some("_"));
}
''')

Path("crates/nova-sema/tests/match_wildcard.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::{ExpressionKind, MatchArmPattern}};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "match-wildcard.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn wildcard_completes_exhaustiveness_without_synthetic_variant_identity() {
    let analyzed = analyze_text(
        "enum Maybe { None, Some(Int) } fn choose(value: Maybe) -> Int { match value { Maybe::None => 0, _ => 1 } } fn main() -> Int { choose(Maybe::Some(9)) }",
    );
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    let tail = analyzed.program.functions[0].body.tail.as_deref().expect("tail");
    let ExpressionKind::Match { arms, .. } = &tail.kind else { panic!("match"); };
    assert!(matches!(arms[0].pattern, MatchArmPattern::Variant { .. }));
    assert!(matches!(arms[1].pattern, MatchArmPattern::Wildcard));
}

#[test]
fn wildcard_shadows_later_arms_with_nonfatal_usefulness_warning() {
    let analyzed = analyze_text(
        "enum Choice { A, B } fn choose(value: Choice) -> Int { match value { _ => 1, Choice::B => 2 } } fn main() -> Int { choose(Choice::B) }",
    );
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    assert!(analyzed.diagnostics.iter().any(|d| d.code == "N3034" && d.message == "statically unreachable match arm"));
}

#[test]
fn wildcard_after_complete_concrete_coverage_is_useless_but_valid() {
    let analyzed = analyze_text(
        "enum Choice { A, B } fn choose(value: Choice) -> Int { match value { Choice::A => 1, Choice::B => 2, _ => 3 } } fn main() -> Int { choose(Choice::A) }",
    );
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    assert_eq!(analyzed.diagnostics.iter().filter(|d| d.code == "N3034").count(), 1);
}

#[test]
fn duplicate_concrete_variants_remain_an_error_even_with_wildcard() {
    let analyzed = analyze_text(
        "enum Choice { A, B } fn main() -> Int { match Choice::A { Choice::A => 1, Choice::A => 2, _ => 3 } }",
    );
    assert!(analyzed.diagnostics.iter().any(|d| d.code == "N3024"));
}

#[test]
fn wildcard_alone_cannot_invent_enum_identity_for_never_scrutinee() {
    let analyzed = analyze_text(
        "fn forever() -> ! { while true {} } fn main() -> Int { match forever() { _ => 1 } }",
    );
    assert!(analyzed.diagnostics.iter().any(|d| d.code == "N3025" && d.message == "cannot determine matched enum"));
}
''')

Path("crates/nova-interpreter/tests/match_wildcard.rs").write_text(r'''use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::{ExpressionKind, MatchArmPattern}};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "match-wildcard.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn wildcard_catches_payload_bearing_variant_without_binding() {
    let analyzed = analyze_text(
        "enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(9) { Maybe::None => 0, _ => 42 } }",
    );
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    assert_eq!(execute(&analyzed.program).expect("execute"), Value::Int(42));
}

#[test]
fn runtime_dispatch_uses_first_matching_arm_in_written_order() {
    let wildcard_first = analyze_text(
        "enum Choice { A, B } fn main() -> Int { match Choice::B { _ => 1, Choice::B => 2 } }",
    );
    assert!(wildcard_first.is_success(), "{:?}", wildcard_first.diagnostics);
    assert_eq!(execute(&wildcard_first.program).expect("execute"), Value::Int(1));

    let concrete_first = analyze_text(
        "enum Choice { A, B } fn main() -> Int { match Choice::B { Choice::B => 2, _ => 1 } }",
    );
    assert!(concrete_first.is_success(), "{:?}", concrete_first.diagnostics);
    assert_eq!(execute(&concrete_first.program).expect("execute"), Value::Int(2));
}

#[test]
fn runtime_still_rejects_duplicate_concrete_slots_in_malformed_hir() {
    let mut analyzed = analyze_text(
        "enum Choice { A, B } fn main() -> Int { match Choice::A { Choice::A => 1, _ => 2 } }",
    );
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    let tail = analyzed.program.functions[0].body.tail.as_deref_mut().expect("tail");
    let ExpressionKind::Match { arms, .. } = &mut tail.kind else { panic!("match"); };
    let duplicate = arms[0].clone();
    arms.push(duplicate);
    assert!(matches!(arms[0].pattern, MatchArmPattern::Variant { .. }));
    let error = execute(&analyzed.program).expect_err("malformed HIR must fail");
    assert_eq!(error.code, "N4005");
}
''')
