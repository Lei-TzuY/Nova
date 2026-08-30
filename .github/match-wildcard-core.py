from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected one anchor in {path}, found {count}: {old[:160]!r}")
    p.write_text(text.replace(old, new, 1))

# AST: match arms now carry either a concrete enum pattern or a bare wildcard.
replace_once(
    "crates/nova-parser/src/ast.rs",
    '''/// One arm in a `match` expression.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct MatchArm {\n    /// Variant pattern selecting this arm.\n    pub pattern: EnumPattern,\n    /// Value produced by the arm.\n    pub value: Expression,\n    /// Complete arm range, excluding a trailing comma.\n    pub span: Span,\n}\n''',
    '''/// Pattern accepted in one `match` arm.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub enum MatchPattern {\n    /// Qualified concrete enum variant.\n    Enum(EnumPattern),\n    /// Bare catch-all pattern written `_`.\n    Wildcard {\n        /// Exact `_` range.\n        span: Span,\n    },\n}\n\nimpl MatchPattern {\n    /// Returns the exact source range occupied by this pattern.\n    #[must_use]\n    pub fn span(&self) -> Span {\n        match self {\n            Self::Enum(pattern) => pattern.span,\n            Self::Wildcard { span } => *span,\n        }\n    }\n}\n\n/// One arm in a `match` expression.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct MatchArm {\n    /// Pattern selecting this arm.\n    pub pattern: MatchPattern,\n    /// Value produced by the arm.\n    pub value: Expression,\n    /// Complete arm range, excluding a trailing comma.\n    pub span: Span,\n}\n''',
)

# Parser: parse `_` as a catch-all only in arm-pattern position.
replace_once(
    "crates/nova-parser/src/parser.rs",
    '''            let pattern = self.parse_enum_pattern()?;\n            self.expect(TokenKind::FatArrow, "after the match pattern")?;\n            let value = self.parse_expression_with_binding_power(0)?;\n            let span = self.cover(pattern.span, value.span);\n''',
    '''            let pattern = self.parse_match_pattern()?;\n            self.expect(TokenKind::FatArrow, "after the match pattern")?;\n            let value = self.parse_expression_with_binding_power(0)?;\n            let span = self.cover(pattern.span(), value.span);\n''',
)
replace_once(
    "crates/nova-parser/src/parser.rs",
    '''    fn parse_enum_pattern(&mut self) -> Option<EnumPattern> {\n        let enumeration = self.parse_name("as a match pattern qualifier")?;\n        self.expect(\n            TokenKind::ColonColon,\n            "after the enum type name in a pattern",\n        )?;\n        let variant = self.parse_name("after `::` in a pattern")?;\n        let (binding, payload_discarded, end) = if self.consume(TokenKind::LeftParen).is_some() {\n            let payload = self.parse_name("as the variant payload binding or `_`")?;\n            let payload_discarded = payload.text == "_";\n            let binding = if payload_discarded {\n                None\n            } else {\n                Some(payload)\n            };\n            let closing = self.expect(TokenKind::RightParen, "after the payload pattern")?;\n            (binding, payload_discarded, closing.span)\n        } else {\n            (None, false, variant.span)\n        };\n        Some(EnumPattern {\n            span: self.cover(enumeration.span, end),\n            enumeration,\n            variant,\n            binding,\n            payload_discarded,\n        })\n    }\n''',
    '''    fn parse_match_pattern(&mut self) -> Option<MatchPattern> {\n        let enumeration = self.parse_name("as a match pattern or `_`")?;\n        if enumeration.text == "_" {\n            return Some(MatchPattern::Wildcard {\n                span: enumeration.span,\n            });\n        }\n        self.expect(\n            TokenKind::ColonColon,\n            "after the enum type name in a pattern",\n        )?;\n        let variant = self.parse_name("after `::` in a pattern")?;\n        let (binding, payload_discarded, end) = if self.consume(TokenKind::LeftParen).is_some() {\n            let payload = self.parse_name("as the variant payload binding or `_`")?;\n            let payload_discarded = payload.text == "_";\n            let binding = if payload_discarded {\n                None\n            } else {\n                Some(payload)\n            };\n            let closing = self.expect(TokenKind::RightParen, "after the payload pattern")?;\n            (binding, payload_discarded, closing.span)\n        } else {\n            (None, false, variant.span)\n        };\n        Some(MatchPattern::Enum(EnumPattern {\n            span: self.cover(enumeration.span, end),\n            enumeration,\n            variant,\n            binding,\n            payload_discarded,\n        }))\n    }\n''',
)

# HIR: wildcard has no fabricated enum-variant identity.
replace_once(
    "crates/nova-sema/src/hir.rs",
    '''/// One resolved arm in an exhaustive enum match.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct MatchArm {\n    /// Resolved variant spelling paired with the declaration-order slot.\n    pub variant_name: String,\n    /// Zero-based variant slot in declaration order.\n    pub variant_index: usize,\n    /// Optional immutable payload binding.\n    pub binding: Option<Binding>,\n    /// Whether a payload-bearing variant explicitly discards its payload with `_`.\n    pub payload_discarded: bool,\n    /// Typed arm value.\n    pub value: Expression,\n    /// Complete arm range.\n    pub span: Span,\n}\n''',
    '''/// Resolved pattern carried by one enum-match arm.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub enum MatchPattern {\n    /// Qualified concrete enum variant with retained source identity.\n    Variant {\n        /// Resolved variant spelling paired with the declaration-order slot.\n        variant_name: String,\n        /// Zero-based variant slot in declaration order.\n        variant_index: usize,\n        /// Optional immutable payload binding.\n        binding: Option<Binding>,\n        /// Whether a payload-bearing variant explicitly discards its payload with `_`.\n        payload_discarded: bool,\n    },\n    /// Bare catch-all pattern. It intentionally carries no fabricated variant identity.\n    Wildcard,\n}\n\n/// One resolved arm in an exhaustive enum match.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct MatchArm {\n    /// Resolved concrete-variant or catch-all pattern.\n    pub pattern: MatchPattern,\n    /// Typed arm value.\n    pub value: Expression,\n    /// Complete arm range.\n    pub span: Span,\n}\n''',
)

# Replace the semantic match lowering as one coherent written-order coverage algorithm.
p = Path("crates/nova-sema/src/analyzer.rs")
text = p.read_text()
start = text.index("    fn lower_match(\n")
end = text.index("    fn lower_field_access(\n", start)
new_lower_match = r'''    fn lower_match(
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

        // A non-continuing scrutinee has no runtime value from which to recover a nominal enum
        // identity. Preserve the existing rule by taking the first resolvable concrete pattern
        // as the static enum context; a wildcard alone cannot invent that identity.
        if scrutinee.ty.is_never() && scrutinee_enum.is_none() {
            for arm in arms {
                let ast::MatchPattern::Enum(pattern) = &arm.pattern else {
                    continue;
                };
                let Some(TypeSymbol {
                    definition: TypeDefinition::Enum(id),
                    ..
                }) = self.types.get(&pattern.enumeration.text).copied()
                else {
                    continue;
                };
                scrutinee_enum = Some(self.enum_definitions[id.index()].enum_type());
                break;
            }
        }

        let entry_state = self.capture_scope_flow_state();
        let mut explicit_seen = BTreeMap::<usize, Span>::new();
        let mut covered = BTreeMap::<usize, Span>::new();
        let mut lowered_arms = Vec::with_capacity(arms.len());
        let mut branch_states = Vec::with_capacity(arms.len());
        let mut branch_types = Vec::with_capacity(arms.len());
        let mut selected_branch = None::<(ScopeFlowState, Type)>;
        let mut selected_taken = false;
        let mut structural_error = scrutinee_enum.is_none() && !scrutinee.ty.is_never();

        for arm in arms {
            self.restore_scope_flow_state(&entry_state);
            let mut valid_pattern = true;
            let mut resolved_index = None;
            let mut resolved_variant_name = None::<String>;
            let mut bind_payload = None::<(ast::Name, Type)>;
            let mut payload_discarded = false;
            let pattern_span = arm.pattern.span();
            let is_wildcard = matches!(arm.pattern, ast::MatchPattern::Wildcard { .. });

            match &arm.pattern {
                ast::MatchPattern::Wildcard { .. } => {
                    if scrutinee_enum.is_none() {
                        valid_pattern = false;
                    }
                }
                ast::MatchPattern::Enum(pattern) => {
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
                            self.diagnostics.push(
                                Diagnostic::error("N3021", "unknown enum").with_primary(
                                    pattern.enumeration.span,
                                    format!(
                                        "cannot resolve enum `{}`",
                                        pattern.enumeration.text
                                    ),
                                ),
                            );
                            valid_pattern = false;
                            None
                        }
                    };

                    if let Some(pattern_enum_id) = pattern_enum_id {
                        let definition = self.enum_definitions[pattern_enum_id.index()].clone();
                        if let Some(expected) = &scrutinee_enum {
                            if expected.id != pattern_enum_id {
                                self.diagnostics.push(
                                    Diagnostic::error(
                                        "N3025",
                                        "pattern enum does not match scrutinee",
                                    )
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
                            resolved_variant_name = Some(pattern.variant.text.clone());
                            match (
                                &declared.payload,
                                &pattern.binding,
                                pattern.payload_discarded,
                            ) {
                                (Some(payload_type), Some(binding_name), false) => {
                                    bind_payload =
                                        Some((binding_name.clone(), payload_type.clone()));
                                }
                                (Some(_), None, true) => payload_discarded = true,
                                (None, None, false) => {}
                                (Some(_), None, false) => {
                                    self.diagnostics.push(
                                        Diagnostic::error(
                                            "N3022",
                                            "missing pattern payload binding",
                                        )
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
                                        Diagnostic::error(
                                            "N3022",
                                            "unexpected pattern payload discard",
                                        )
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
                                        Diagnostic::error(
                                            "N3022",
                                            "unexpected pattern payload binding",
                                        )
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
                }
            }

            if valid_pattern {
                if let Some(index) = resolved_index {
                    if let Some(previous) = explicit_seen.get(&index).copied() {
                        let variant_name = resolved_variant_name
                            .as_deref()
                            .unwrap_or("<unknown>");
                        let enum_name = scrutinee_enum
                            .as_ref()
                            .map_or("<unknown>", |enumeration| enumeration.name.as_str());
                        self.diagnostics.push(
                            Diagnostic::error("N3024", "duplicate match variant")
                                .with_primary(
                                    pattern_span,
                                    format!(
                                        "variant `{enum_name}::{variant_name}` is matched more than once"
                                    ),
                                )
                                .with_secondary(previous, "first matching arm is here"),
                        );
                        valid_pattern = false;
                    } else {
                        explicit_seen.insert(index, pattern_span);
                    }
                }
            }

            let mut useful = false;
            let mut previously_covering_span = None;
            if valid_pattern {
                if is_wildcard {
                    if let Some(enumeration) = &scrutinee_enum {
                        let variant_count = self.enum_definitions[enumeration.id.index()]
                            .variants
                            .len();
                        useful = (0..variant_count).any(|index| !covered.contains_key(&index));
                        if !useful {
                            previously_covering_span = covered.values().next().copied();
                        }
                        for index in 0..variant_count {
                            covered.entry(index).or_insert(pattern_span);
                        }
                    }
                } else if let Some(index) = resolved_index {
                    useful = !covered.contains_key(&index);
                    previously_covering_span = covered.get(&index).copied();
                    covered.entry(index).or_insert(pattern_span);
                }
            }

            let matches_selected = selected_variant_index.is_some_and(|selected| {
                is_wildcard || resolved_index == Some(selected)
            });
            let selected_arm = selected_variant_index.is_some()
                && valid_pattern
                && useful
                && matches_selected
                && !selected_taken;
            if selected_arm {
                selected_taken = true;
            }
            let arm_reachable = if scrutinee.ty.is_never() {
                false
            } else if selected_variant_index.is_some() {
                selected_arm
            } else {
                valid_pattern && useful && scrutinee_enum.is_some()
            };
            let arm_edge = if arm_reachable {
                FlowEdgeKind::Execution
            } else {
                FlowEdgeKind::Diagnostic
            };
            self.flow_fork_from(entry_state.flow_cursor, Some(arm.span), arm_edge);
            self.scopes.push(BTreeMap::new());

            let mut payload_binding = bind_payload.map(|(binding_name, payload_type)| {
                let binding = self.new_binding(&binding_name, payload_type, false);
                self.insert_local(&binding);
                self.record_initialization(binding.id, binding.span);
                binding
            });
            if payload_binding.is_none() {
                if let ast::MatchPattern::Enum(pattern) = &arm.pattern {
                    if let Some(binding_name) = &pattern.binding {
                        if !valid_pattern {
                            let binding = self.new_binding(binding_name, Type::Error, false);
                            self.insert_local(&binding);
                            self.record_initialization(binding.id, binding.span);
                            payload_binding = Some(binding);
                        }
                    }
                }
            }

            if valid_pattern && self.diagnostic_only_depth == 0 {
                if !useful {
                    let mut diagnostic = Diagnostic::warning(
                        "N3034",
                        "statically unreachable match arm",
                    )
                    .with_primary(
                        pattern_span,
                        if is_wildcard {
                            "earlier arms already cover every variant".to_owned()
                        } else {
                            let enumeration = scrutinee_enum
                                .as_ref()
                                .map_or("<unknown>", |value| value.name.as_str());
                            let variant = resolved_variant_name
                                .as_deref()
                                .unwrap_or("<unknown>");
                            format!(
                                "earlier arms already cover `{enumeration}::{variant}`"
                            )
                        },
                    )
                    .with_note(
                        "the arm remains name/type checked for deterministic diagnostics but contributes no reachable flow facts",
                    );
                    if let Some(previous) = previously_covering_span {
                        diagnostic = diagnostic.with_secondary(
                            previous,
                            "an earlier pattern already covers this arm",
                        );
                    }
                    self.deferred_warnings.push(diagnostic);
                } else if let Some(selected) = selected_variant_index {
                    if !selected_arm {
                        if let Some(enumeration) = &scrutinee_enum {
                            let selected_name = &self.enum_definitions[enumeration.id.index()]
                                .variants[selected]
                                .name;
                            let message = if is_wildcard {
                                format!(
                                    "this catch-all arm cannot be selected because an earlier arm already matches `{}::{selected_name}`",
                                    enumeration.name
                                )
                            } else {
                                format!(
                                    "this arm matches `{}::{}`, but this scrutinee can only select `{}::{selected_name}`",
                                    enumeration.name,
                                    resolved_variant_name.as_deref().unwrap_or("<unknown>"),
                                    enumeration.name
                                )
                            };
                            self.deferred_warnings.push(
                                Diagnostic::warning(
                                    "N3034",
                                    "statically unreachable match arm",
                                )
                                .with_primary(pattern_span, message)
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

            let value = if arm_reachable {
                self.lower_expression(&arm.value, return_type)
            } else {
                self.lower_expression_for_diagnostics(&arm.value, return_type)
            };
            let popped = self.scopes.pop();
            debug_assert!(popped.is_some());
            let branch_state = (self.capture_scope_flow_state(), value.ty.is_never());
            if selected_arm {
                selected_branch = Some((branch_state.0.clone(), value.ty.clone()));
            }
            branch_states.push(branch_state);
            branch_types.push((value.ty.clone(), value.span));

            let lowered_pattern = match (&arm.pattern, resolved_index) {
                (ast::MatchPattern::Wildcard { .. }, _) => Some(hir::MatchPattern::Wildcard),
                (ast::MatchPattern::Enum(_), Some(variant_index)) => {
                    Some(hir::MatchPattern::Variant {
                        variant_name: resolved_variant_name
                            .clone()
                            .unwrap_or_else(|| "<unknown>".to_owned()),
                        variant_index,
                        binding: payload_binding,
                        payload_discarded,
                    })
                }
                _ => None,
            };
            if let Some(pattern) = lowered_pattern {
                lowered_arms.push(MatchArm {
                    pattern,
                    value,
                    span: arm.span,
                });
            }
            structural_error |= !valid_pattern;
        }

        if scrutinee.ty.is_never() && scrutinee_enum.is_none() {
            self.diagnostics.push(
                Diagnostic::error("N3025", "cannot determine matched enum").with_primary(
                    span,
                    "a match with a non-continuing scrutinee still needs at least one qualified variant arm",
                ),
            );
            structural_error = true;
        }

        if let Some(enumeration) = &scrutinee_enum {
            let definition = &self.enum_definitions[enumeration.id.index()];
            let missing = definition
                .variants
                .iter()
                .enumerate()
                .filter(|(index, _)| !covered.contains_key(index))
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
p.write_text(text[:start] + new_lower_match + text[end:])

# Interpreter: validate explicit concrete duplicates separately from coverage and dispatch first-match.
p = Path("crates/nova-interpreter/src/lib.rs")
text = p.read_text()
old_start = text.index("            ExpressionKind::Match {\n")
old_end = text.index("            ExpressionKind::Error =>", old_start)
new_runtime = r'''            ExpressionKind::Match {
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
                        "resolved match enum identity does not match its declaration slot",
                    ));
                }
                let mut explicit_seen = vec![false; definition.variants.len()];
                let mut covered = vec![false; definition.variants.len()];
                for arm in arms {
                    match &arm.pattern {
                        nova_sema::hir::MatchPattern::Variant {
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
                            let Some(explicit_slot) = explicit_seen.get_mut(*variant_index) else {
                                return Err(self.invariant(
                                    arm.span,
                                    "resolved match arm targets a variant outside its enum",
                                ));
                            };
                            if *explicit_slot {
                                return Err(self.invariant(
                                    arm.span,
                                    "resolved match contains a duplicate concrete variant arm",
                                ));
                            }
                            *explicit_slot = true;
                            covered[*variant_index] = true;
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
                        nova_sema::hir::MatchPattern::Wildcard => {
                            covered.fill(true);
                        }
                    }
                }
                if covered.iter().any(|covered| !covered) {
                    return Err(self.invariant(
                        expression.span,
                        "resolved match is not exhaustive for its enum declaration",
                    ));
                }

                let Some(arm) = arms.iter().find(|arm| match &arm.pattern {
                    nova_sema::hir::MatchPattern::Variant {
                        variant_index: candidate,
                        ..
                    } => *candidate == variant_index,
                    nova_sema::hir::MatchPattern::Wildcard => true,
                }) else {
                    return Err(self.invariant(
                        expression.span,
                        "exhaustive match has no arm for the runtime variant",
                    ));
                };
                match &arm.pattern {
                    nova_sema::hir::MatchPattern::Wildcard => {}
                    nova_sema::hir::MatchPattern::Variant {
                        binding,
                        payload_discarded,
                        ..
                    } => match (binding, *payload_discarded, payload) {
                        (Some(binding), false, Some(payload)) => {
                            self.bind_runtime_slot(frame, binding, Some(*payload), arm.span)?;
                        }
                        (None, true, Some(_)) | (None, false, None) => {}
                        _ => {
                            return Err(self.invariant(
                                arm.span,
                                "runtime enum payload mode does not match the selected arm",
                            ));
                        }
                    },
                }
                self.eval_expression(&arm.value, frame)
            }
'''
p.write_text(text[:old_start] + new_runtime + text[old_end:])

# Parser regression.
Path("crates/nova-parser/tests").mkdir(parents=True, exist_ok=True)
Path("crates/nova-parser/tests/match_wildcard.rs").write_text(r'''use nova_lexer::lex;
use nova_parser::{ast, parse};
use nova_source::{SourceFile, SourceId};

fn parse_text(text: &str) -> ast::Program {
    let source = SourceFile::new(SourceId::new(0), "wildcard.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    parsed.program
}

#[test]
fn bare_underscore_is_a_catch_all_but_payload_underscore_remains_discard() {
    let program = parse_text(
        "enum Maybe { None, Some(Int) } fn f(x: Maybe) -> Int { match x { Maybe::Some(_) => 1, _ => 0, } }",
    );
    let tail = program.functions[0].body.tail.as_deref().unwrap();
    let ast::ExpressionKind::Match { arms, .. } = &tail.kind else {
        panic!("expected match");
    };
    let ast::MatchPattern::Enum(first) = &arms[0].pattern else {
        panic!("expected concrete pattern");
    };
    assert!(first.payload_discarded);
    assert!(first.binding.is_none());
    assert!(matches!(arms[1].pattern, ast::MatchPattern::Wildcard { .. }));
}
''')

# Semantic regressions.
Path("crates/nova-sema/tests/match_wildcard.rs").write_text(r'''use nova_diagnostics::Severity;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "wildcard.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn wildcard_completes_exhaustiveness_without_fabricating_a_variant() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int), Other } fn f(x: Maybe) -> Int { match x { Maybe::Some(value) => value, _ => 0, } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let tail = output.program.functions[0].body.tail.as_deref().unwrap();
    let hir::ExpressionKind::Match { arms, .. } = &tail.kind else {
        panic!("expected match");
    };
    assert_eq!(arms.len(), 2);
    assert!(matches!(arms[1].pattern, hir::MatchPattern::Wildcard));
}

#[test]
fn arm_after_wildcard_is_warned_but_remains_type_checked() {
    let output = analyze_text(
        "enum Flag { Off, On } fn f(x: Flag) -> Int { match x { _ => 1, Flag::On => 2, } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let warnings = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .collect::<Vec<_>>();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "N3034");
}

#[test]
fn wildcard_only_cannot_invent_enum_identity_for_never_scrutinee() {
    let output = analyze_text(
        "fn stop() -> ! { while true {} } fn f() -> Int { match stop() { _ => 0, } }",
    );
    assert!(output.has_errors());
    assert!(output.diagnostics.iter().any(|diagnostic| diagnostic.code == "N3025"));
}
''')

# Runtime and written-order dispatch regressions.
Path("crates/nova-interpreter/tests/match_wildcard.rs").write_text(r'''use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::analyze;
use nova_source::{SourceFile, SourceId};

fn execute_text(text: &str) -> Value {
    let source = SourceFile::new(SourceId::new(0), "wildcard.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    execute(&analyzed.program).expect("execution should succeed")
}

#[test]
fn wildcard_handles_payload_and_payload_free_variants() {
    let some = execute_text(
        "enum Maybe { None, Some(Int) } fn main() -> Int { let x = Maybe::Some(99); match x { Maybe::None => 0, _ => 42, } }",
    );
    assert_eq!(some, Value::Int(42));

    let none = execute_text(
        "enum Maybe { None, Some(Int) } fn main() -> Int { let x = Maybe::None; match x { Maybe::Some(_) => 0, _ => 42, } }",
    );
    assert_eq!(none, Value::Int(42));
}

#[test]
fn runtime_dispatch_uses_first_matching_arm_in_written_order() {
    let value = execute_text(
        "enum Flag { Off, On } fn main() -> Int { match Flag::On { _ => 42, Flag::On => 0, } }",
    );
    assert_eq!(value, Value::Int(42));
}
''')
