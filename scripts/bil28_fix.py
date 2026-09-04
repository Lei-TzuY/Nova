from pathlib import Path

p = Path("crates/nova-sema/src/analyzer.rs")
text = p.read_text()
text = text.replace(
    "                let assignment_entry_state = self.capture_reachable_state();\n",
    "",
    1,
)
p.write_text(text)

p = Path("crates/nova-interpreter/src/lib.rs")
text = p.read_text()
old = """                    && closure\n                        .captures\n                        .iter()\n                        .zip(captures)\n                        .all(|(capture, value)| self.value_conforms_to_type(value, &capture.ty))"""
new = """                    && closure.captures.iter().zip(captures).all(|(capture, value)| {\n                        match (capture.mode, value) {\n                            (CaptureMode::ByValue, RuntimeCapture::ByValue(value)) => {\n                                self.value_conforms_to_type(value, &capture.ty)\n                            }\n                            (CaptureMode::ByReference, RuntimeCapture::ByReference(cell)) => self\n                                .shared_cells\n                                .get(*cell)\n                                .and_then(Option::as_ref)\n                                .is_some_and(|value| self.value_conforms_to_type(value, &capture.ty)),\n                            _ => false,\n                        }\n                    })"""
if text.count(old) != 1:
    raise SystemExit(f"closure conformity replacement count={text.count(old)}")
p.write_text(text.replace(old, new, 1))
