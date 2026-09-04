from pathlib import Path

p = Path('crates/nova-inspect/src/lib.rs')
text = p.read_text()
text = text.replace(
    '    active_capture_bindings: Vec<BTreeSet<hir::BindingId>>,\n    active_capture_uses: Vec<BTreeSet<hir::BindingId>>,\n',
    '    active_capture_bindings: Vec<BTreeSet<hir::BindingId>>,\n    active_reference_capture_bindings: Vec<BTreeSet<hir::BindingId>>,\n    active_capture_uses: Vec<BTreeSet<hir::BindingId>>,\n', 1)
text = text.replace(
    '            active_capture_bindings: Vec::new(),\n            active_capture_uses: Vec::new(),\n',
    '            active_capture_bindings: Vec::new(),\n            active_reference_capture_bindings: Vec::new(),\n            active_capture_uses: Vec::new(),\n', 1)
text = text.replace(
    '                let mut capture_modes = Vec::with_capacity(closure.captures.len());\n',
    '                let mut capture_modes = Vec::with_capacity(closure.captures.len());\n                let mut reference_capture_ids = BTreeSet::new();\n', 1)
needle = '''                    if capture.mode == hir::CaptureMode::ByReference
                        && !self.allow_reference_captures
                    {
                        return Err(InspectionError::invalid(
                            "semantic-inspection schema v1-v7 cannot represent a by-reference closure capture; select schema v8",
                        ));
                    }
                    capture_modes.push(capture.mode);
'''
replacement = '''                    if capture.mode == hir::CaptureMode::ByReference {
                        if !self.allow_reference_captures {
                            return Err(InspectionError::invalid(
                                "semantic-inspection schema v1-v7 cannot represent a by-reference closure capture; select schema v8",
                            ));
                        }
                        if !binding.mutable {
                            return Err(InspectionError::invalid(format!(
                                "{} by-reference capture targets immutable {}",
                                closure_owner, binding.id
                            )));
                        }
                        reference_capture_ids.insert(capture.reference.binding);
                    }
                    capture_modes.push(capture.mode);
'''
if text.count(needle) != 1:
    raise SystemExit('reference validation anchor mismatch')
text = text.replace(needle, replacement, 1)
text = text.replace(
    '                self.active_capture_bindings.push(capture_ids.clone());\n                self.active_capture_uses.push(BTreeSet::new());\n',
    '                self.active_capture_bindings.push(capture_ids.clone());\n                self.active_reference_capture_bindings.push(reference_capture_ids);\n                self.active_capture_uses.push(BTreeSet::new());\n', 1)
text = text.replace(
    '                self.active_capture_bindings.pop();\n                let body = body_result?;\n',
    '                self.active_reference_capture_bindings.pop();\n                self.active_capture_bindings.pop();\n                let body = body_result?;\n', 1)
old = '''                if binding.owner != owner {
                    return Err(InspectionError::invalid(format!(
                        "assignment targets captured snapshot {}",
                        binding_id(resolved.binding.index())
                    )));
                }
'''
new = '''                if binding.owner != owner
                    && !self
                        .active_reference_capture_bindings
                        .last()
                        .is_some_and(|captures| captures.contains(&resolved.binding))
                {
                    return Err(InspectionError::invalid(format!(
                        "assignment targets captured snapshot {}",
                        binding_id(resolved.binding.index())
                    )));
                }
'''
if text.count(old) != 1:
    raise SystemExit('assignment capture validation anchor mismatch')
text = text.replace(old, new, 1)
p.write_text(text)
