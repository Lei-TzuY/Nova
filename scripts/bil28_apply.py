from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one exact match, found {count}")
    p.write_text(text.replace(old, new, 1))


# HIR: make capture transfer mode explicit.
replace_once(
    "crates/nova-sema/src/hir.rs",
    """/// One lexical binding captured by value when a closure is created.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct Capture {\n    /// Resolved declaration identity and metadata.\n    pub reference: BindingReference,\n    /// Resolved value type copied into the closure environment.\n    pub ty: Type,\n    /// First lexical use that caused this capture, used for deterministic ordering and diagnostics.\n    pub first_use: Span,\n}\n""",
    """/// How one lexical binding enters a closure environment.\n#[derive(Clone, Copy, Debug, Eq, PartialEq)]\npub enum CaptureMode {\n    /// Copy the current value when the closure expression is evaluated.\n    ByValue,\n    /// Share one mutable runtime cell with the enclosing binding and closure aliases.\n    ByReference,\n}\n\n/// One lexical binding captured when a closure is created.\n#[derive(Clone, Debug, Eq, PartialEq)]\npub struct Capture {\n    /// Resolved declaration identity and metadata.\n    pub reference: BindingReference,\n    /// Resolved value type exposed through the closure environment.\n    pub ty: Type,\n    /// Environment transfer mode selected by semantic analysis.\n    pub mode: CaptureMode,\n    /// First lexical use that caused this capture, used for deterministic ordering and diagnostics.\n    pub first_use: Span,\n}\n""",
)
replace_once(
    "crates/nova-sema/src/hir.rs",
    "    /// By-value environment inputs in first-lexical-use order.\n    pub captures: Vec<Capture>,",
    "    /// Environment inputs in first-lexical-use order.\n    pub captures: Vec<Capture>,",
)

# Semantic analysis: read-only mutable captures remain snapshots; writes upgrade that capture to by-reference.
p = Path("crates/nova-sema/src/analyzer.rs")
text = p.read_text()
text = text.replace(
    "self.capture_binding_if_needed(scope_index, &name.text, name.span, &symbol)",
    "self.capture_binding_if_needed(\n                scope_index,\n                &name.text,\n                name.span,\n                &symbol,\n                hir::CaptureMode::ByValue,\n            )",
)
text = text.replace(
    "                    } else if !self.capture_binding_if_needed(\n                        scope_index,\n                        &target.text,\n                        target.span,\n                        &symbol,\n                    ) {",
    "                    } else if !self.capture_binding_if_needed(\n                        scope_index,\n                        &target.text,\n                        target.span,\n                        &symbol,\n                        hir::CaptureMode::ByValue,\n                    ) {",
)
old = """                    let writes_through_snapshot = self\n                        .closure_stack\n                        .last()\n                        .is_some_and(|context| scope_index < context.scope_base && symbol.mutable);\n                    if writes_through_snapshot {\n                        self.diagnostics.push(\n                            Diagnostic::error(\"N3035\", \"cannot assign through a lexical snapshot capture\")\n                                .with_primary(\n                                    target.span,\n                                    format!(\"`{}` is captured by value and cannot be assigned here\", target.text),\n                                )\n                                .with_secondary(symbol.span, \"mutable binding declared here\")\n                                .with_note(\n                                    \"reading an enclosing `var` snapshots its value when the closure is created; shared-cell mutation is not yet defined\",\n                                ),\n                        );\n                        self.require_type(&value.ty, &symbol.ty, value.span, \"assigned value\");\n                        if !value.ty.is_never() {\n                            self.restore_reachable_state(assignment_entry_state);\n                        }\n                        None\n"""
new = """                    let writes_through_capture = self\n                        .closure_stack\n                        .last()\n                        .is_some_and(|context| scope_index < context.scope_base && symbol.mutable);\n                    if writes_through_capture {\n                        self.capture_binding_if_needed(\n                            scope_index,\n                            &target.text,\n                            target.span,\n                            &symbol,\n                            hir::CaptureMode::ByReference,\n                        );\n                        self.require_type(&value.ty, &symbol.ty, value.span, \"assigned value\");\n                        if !value.ty.is_error()\n                            && !value.ty.is_never()\n                            && expected_type_compatible(&value.ty, &symbol.ty)\n                        {\n                            self.record_initialization(symbol.id, target.span);\n                        }\n                        Some(hir::BindingReference {\n                            binding: symbol.id,\n                            binding_name: target.text.clone(),\n                            declaration_span: symbol.span,\n                        })\n"""
if text.count(old) != 1:
    raise SystemExit(f"analyzer assignment replacement count={text.count(old)}")
text = text.replace(old, new, 1)

start = text.index("    fn capture_binding_if_needed(")
end = text.index("    fn record_capture_creation_read(", start)
replacement = """    fn capture_binding_if_needed(\n        &mut self,\n        scope_index: usize,\n        name: &str,\n        use_span: Span,\n        symbol: &LocalSymbol,\n        requested_mode: hir::CaptureMode,\n    ) -> bool {\n        let Some(scope_base) = self.closure_stack.last().map(|context| context.scope_base) else {\n            return true;\n        };\n        if scope_index >= scope_base {\n            return true;\n        }\n        let existing_index = self\n            .closure_stack\n            .last()\n            .and_then(|context| context.captures.iter().position(|capture| capture.reference.binding == symbol.id));\n        if let Some(index) = existing_index {\n            if requested_mode == hir::CaptureMode::ByReference {\n                self.closure_stack\n                    .last_mut()\n                    .expect(\"a capture requires an active closure context\")\n                    .captures[index]\n                    .mode = hir::CaptureMode::ByReference;\n            }\n            return true;\n        }\n\n        let binding = hir::Binding {\n            id: symbol.id,\n            name: name.to_owned(),\n            ty: symbol.ty.clone(),\n            mutable: requested_mode == hir::CaptureMode::ByReference,\n            span: symbol.span,\n        };\n        self.flow\n            .as_mut()\n            .expect(\"closure capture requires an active CFG\")\n            .register_binding(&binding);\n        self.record_initialization(binding.id, use_span);\n        let context = self\n            .closure_stack\n            .last_mut()\n            .expect(\"a capture requires an active closure context\");\n        context.captured_bindings.insert(binding.id);\n        context.captures.push(hir::Capture {\n            reference: hir::BindingReference {\n                binding: binding.id,\n                binding_name: binding.name,\n                declaration_span: binding.span,\n            },\n            ty: binding.ty,\n            mode: requested_mode,\n            first_use: use_span,\n        });\n        true\n    }\n\n"""
text = text[:start] + replacement + text[end:]
text = text.replace(
    """        if self.capture_binding_if_needed(\n            scope_index,\n            &capture.reference.binding_name,\n            capture.first_use,\n            &symbol,\n        ) {""",
    """        if self.capture_binding_if_needed(\n            scope_index,\n            &capture.reference.binding_name,\n            capture.first_use,\n            &symbol,\n            capture.mode,\n        ) {""",
)
p.write_text(text)

# Interpreter: by-reference captures point at interpreter-owned cells; by-value captures stay snapshots.
p = Path("crates/nova-interpreter/src/lib.rs")
text = p.read_text()
text = text.replace(
    "    Binding, BindingId, BindingReference, Block, Closure, EnumId, Expression, ExpressionKind,\n    Function, FunctionId, Program, RecordId, Statement, StatementKind, Type,",
    "    Binding, BindingId, BindingReference, Block, CaptureMode, Closure, EnumId, Expression,\n    ExpressionKind, Function, FunctionId, Program, RecordId, Statement, StatementKind, Type,",
)
text = text.replace(
    "        /// Captured values aligned with `closure.captures`.\n        captures: Vec<Value>,",
    "        /// Runtime captures aligned with `closure.captures`.\n        captures: Vec<RuntimeCapture>,",
)
marker = "impl fmt::Display for Value {"
runtime_capture = """#[derive(Clone, Debug, Eq, PartialEq)]\nenum RuntimeCapture {\n    ByValue(Value),\n    ByReference(usize),\n}\n\n"""
if text.count(marker) != 1:
    raise SystemExit("interpreter display marker mismatch")
text = text.replace(marker, runtime_capture + marker, 1)
text = text.replace(
    """struct RuntimeSlot {\n    ty: Type,\n    mutable: bool,\n    binding_name: String,\n    declaration_span: nova_source::Span,\n    value: Option<Value>,\n}""",
    """struct RuntimeSlot {\n    ty: Type,\n    mutable: bool,\n    binding_name: String,\n    declaration_span: nova_source::Span,\n    value: Option<Value>,\n    shared_cell: Option<usize>,\n}""",
)
text = text.replace(
    """    next_closure_instance: usize,\n}""",
    """    next_closure_instance: usize,\n    shared_cells: Vec<Option<Value>>,\n}""",
    1,
)
text = text.replace(
    """            next_closure_instance: 0,\n        }""",
    """            next_closure_instance: 0,\n            shared_cells: Vec::new(),\n        }""",
    1,
)
text = text.replace("        captured_values: Vec<Value>,", "        captured_values: Vec<RuntimeCapture>,", 1)

old = """            if !self.value_conforms_to_type(value, &capture.ty) {\n                return Err(self.invariant(\n                    capture.first_use,\n                    format!(\n                        \"captured `{}` value does not conform to resolved type {}\",\n                        capture.reference.binding_name, capture.ty\n                    ),\n                ));\n            }"""
new = """            match (capture.mode, value) {\n                (CaptureMode::ByValue, RuntimeCapture::ByValue(value)) => {\n                    if !self.value_conforms_to_type(value, &capture.ty) {\n                        return Err(self.invariant(\n                            capture.first_use,\n                            format!(\n                                \"captured `{}` value does not conform to resolved type {}\",\n                                capture.reference.binding_name, capture.ty\n                            ),\n                        ));\n                    }\n                }\n                (CaptureMode::ByReference, RuntimeCapture::ByReference(cell)) => {\n                    let Some(value) = self.shared_cells.get(*cell).and_then(Option::as_ref) else {\n                        return Err(self.invariant(\n                            capture.first_use,\n                            \"by-reference closure capture points at an absent or uninitialized shared cell\",\n                        ));\n                    };\n                    if !self.value_conforms_to_type(value, &capture.ty) {\n                        return Err(self.invariant(\n                            capture.first_use,\n                            \"by-reference closure capture cell does not conform to its resolved type\",\n                        ));\n                    }\n                }\n                _ => {\n                    return Err(self.invariant(\n                        capture.first_use,\n                        \"closure runtime capture mode does not match typed HIR\",\n                    ));\n                }\n            }"""
if text.count(old) != 1:
    raise SystemExit(f"interpreter capture validation count={text.count(old)}")
text = text.replace(old, new, 1)

old = """        for (capture, value) in closure.captures.iter().zip(captured_values) {\n            let binding = Binding {\n                id: capture.reference.binding,\n                name: capture.reference.binding_name.clone(),\n                ty: capture.ty.clone(),\n                mutable: false,\n                span: capture.reference.declaration_span,\n            };\n            self.bind_runtime_slot(&mut frame, &binding, Some(value), capture.first_use)?;\n        }"""
new = """        for (capture, value) in closure.captures.iter().zip(captured_values) {\n            match (capture.mode, value) {\n                (CaptureMode::ByValue, RuntimeCapture::ByValue(value)) => {\n                    let binding = Binding {\n                        id: capture.reference.binding,\n                        name: capture.reference.binding_name.clone(),\n                        ty: capture.ty.clone(),\n                        mutable: false,\n                        span: capture.reference.declaration_span,\n                    };\n                    self.bind_runtime_slot(&mut frame, &binding, Some(value), capture.first_use)?;\n                }\n                (CaptureMode::ByReference, RuntimeCapture::ByReference(cell)) => {\n                    frame.insert(\n                        capture.reference.binding,\n                        RuntimeSlot {\n                            ty: capture.ty.clone(),\n                            mutable: true,\n                            binding_name: capture.reference.binding_name.clone(),\n                            declaration_span: capture.reference.declaration_span,\n                            value: None,\n                            shared_cell: Some(cell),\n                        },\n                    );\n                }\n                _ => {\n                    return Err(self.invariant(\n                        capture.first_use,\n                        \"closure runtime capture mode changed while entering the call frame\",\n                    ));\n                }\n            }\n        }"""
if text.count(old) != 1:
    raise SystemExit(f"interpreter call frame capture count={text.count(old)}")
text = text.replace(old, new, 1)

old = """                        slot.value = Some(value);\n                        Ok(None)"""
new = """                        if let Some(cell) = slot.shared_cell {\n                            let Some(destination) = self.shared_cells.get_mut(cell) else {\n                                return Err(self.invariant(\n                                    statement.span,\n                                    \"assignment target points at a missing shared capture cell\",\n                                ));\n                            };\n                            *destination = Some(value);\n                        } else {\n                            slot.value = Some(value);\n                        }\n                        Ok(None)"""
if text.count(old) != 1:
    raise SystemExit(f"interpreter assignment store count={text.count(old)}")
text = text.replace(old, new, 1)

old = """                    let Some(value) = slot.value.as_ref() else {\n                        return Err(self.invariant(\n                            capture.first_use,\n                            \"closure captured a runtime slot before initialization\",\n                        ));\n                    };\n                    if !self.value_conforms_to_type(value, &capture.ty) {\n                        return Err(self.invariant(\n                            capture.first_use,\n                            \"closure capture value does not conform to its resolved type\",\n                        ));\n                    }\n                    captured_values.push(value.clone());"""
new = """                    match capture.mode {\n                        CaptureMode::ByValue => {\n                            let value = if let Some(cell) = slot.shared_cell {\n                                self.shared_cells\n                                    .get(cell)\n                                    .and_then(Option::as_ref)\n                                    .ok_or_else(|| self.invariant(\n                                        capture.first_use,\n                                        \"closure captured a shared runtime slot before initialization\",\n                                    ))?\n                            } else {\n                                slot.value.as_ref().ok_or_else(|| self.invariant(\n                                    capture.first_use,\n                                    \"closure captured a runtime slot before initialization\",\n                                ))?\n                            };\n                            if !self.value_conforms_to_type(value, &capture.ty) {\n                                return Err(self.invariant(\n                                    capture.first_use,\n                                    \"closure capture value does not conform to its resolved type\",\n                                ));\n                            }\n                            captured_values.push(RuntimeCapture::ByValue(value.clone()));\n                        }\n                        CaptureMode::ByReference => {\n                            if !slot.mutable {\n                                return Err(self.invariant(\n                                    capture.first_use,\n                                    \"by-reference capture resolved to an immutable runtime slot\",\n                                ));\n                            }\n                            let existing = slot.shared_cell;\n                            let cell = if let Some(cell) = existing {\n                                cell\n                            } else {\n                                let slot = frame\n                                    .get_mut(&capture.reference.binding)\n                                    .expect(\"validated by-reference capture must have a runtime slot\");\n                                let value = slot.value.take().ok_or_else(|| self.invariant(\n                                    capture.first_use,\n                                    \"by-reference closure captured a runtime slot before initialization\",\n                                ))?;\n                                let cell = self.shared_cells.len();\n                                self.shared_cells.push(Some(value));\n                                slot.shared_cell = Some(cell);\n                                cell\n                            };\n                            let Some(value) = self.shared_cells.get(cell).and_then(Option::as_ref) else {\n                                return Err(self.invariant(\n                                    capture.first_use,\n                                    \"by-reference closure capture points at an uninitialized shared cell\",\n                                ));\n                            };\n                            if !self.value_conforms_to_type(value, &capture.ty) {\n                                return Err(self.invariant(\n                                    capture.first_use,\n                                    \"by-reference closure capture cell does not conform to its resolved type\",\n                                ));\n                            }\n                            captured_values.push(RuntimeCapture::ByReference(cell));\n                        }\n                    }"""
if text.count(old) != 1:
    raise SystemExit(f"interpreter closure creation count={text.count(old)}")
text = text.replace(old, new, 1)

old = """                let Some(value) = slot.value.as_ref() else {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            \"binding {} reached runtime before initialization\",\n                            reference.binding.index()\n                        ),\n                    ));\n                };\n                if !self.value_conforms_to_type(value, &slot.ty) {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            \"binding {} stored a runtime value that does not conform to slot type {}\",\n                            reference.binding.index(), slot.ty\n                        ),\n                    ));\n                }\n                Ok(Flow::Value(value.clone()))"""
new = """                let value = if let Some(cell) = slot.shared_cell {\n                    self.shared_cells.get(cell).and_then(Option::as_ref)\n                } else {\n                    slot.value.as_ref()\n                };\n                let Some(value) = value else {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            \"binding {} reached runtime before initialization\",\n                            reference.binding.index()\n                        ),\n                    ));\n                };\n                if !self.value_conforms_to_type(value, &slot.ty) {\n                    return Err(self.invariant(\n                        expression.span,\n                        format!(\n                            \"binding {} stored a runtime value that does not conform to slot type {}\",\n                            reference.binding.index(), slot.ty\n                        ),\n                    ));\n                }\n                Ok(Flow::Value(value.clone()))"""
if text.count(old) != 1:
    raise SystemExit(f"interpreter binding read count={text.count(old)}")
text = text.replace(old, new, 1)

text = text.replace(
    """                declaration_span: binding.span,\n                value,\n            },""",
    """                declaration_span: binding.span,\n                value,\n                shared_cell: None,\n            },""",
    1,
)
p.write_text(text)
