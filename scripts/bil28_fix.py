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
text = text.replace(
    "enum RuntimeCapture {\n    ByValue(Value),\n    ByReference(usize),\n}",
    "pub enum RuntimeCapture {\n    ByValue(Value),\n    ByReference(usize),\n}",
    1,
)
old = """                    && closure\n                        .captures\n                        .iter()\n                        .zip(captures)\n                        .all(|(capture, value)| self.value_conforms_to_type(value, &capture.ty))"""
new = """                    && closure.captures.iter().zip(captures).all(|(capture, value)| {\n                        match (capture.mode, value) {\n                            (CaptureMode::ByValue, RuntimeCapture::ByValue(value)) => {\n                                self.value_conforms_to_type(value, &capture.ty)\n                            }\n                            (CaptureMode::ByReference, RuntimeCapture::ByReference(cell)) => self\n                                .shared_cells\n                                .get(*cell)\n                                .and_then(Option::as_ref)\n                                .is_some_and(|value| self.value_conforms_to_type(value, &capture.ty)),\n                            _ => false,\n                        }\n                    })"""
if text.count(old) != 1:
    raise SystemExit(f"closure conformity replacement count={text.count(old)}")
p.write_text(text.replace(old, new, 1))

p = Path("crates/nova-cli/tests/closure_snapshot.rs")
text = p.read_text()
old = '''#[test]\nfn assignment_through_snapshot_has_human_and_json_diagnostics() {\n    let source =\n        "fn main() -> Int { var value = 40; let set = fn() -> Int { value = 99; value }; set() }";\n    let human = run_stdin(&["check", "-"], source);\n    assert_eq!(human.status.code(), Some(1));\n    assert!(human.stdout.is_empty());\n    let stderr = String::from_utf8_lossy(&human.stderr);\n    assert!(stderr.contains("error[N3035]"), "{stderr}");\n    assert!(stderr.contains("captured by value"), "{stderr}");\n\n    let json = run_stdin(&["check", "-", "--message-format=json"], source);\n    assert_eq!(json.status.code(), Some(1));\n    assert!(json.stdout.is_empty());\n    let stderr = String::from_utf8_lossy(&json.stderr);\n    assert!(stderr.contains("\\\"code\\\":\\\"N3035\\\""), "{stderr}");\n    assert!(stderr.contains("lexical snapshot capture"), "{stderr}");\n}\n'''
new = '''#[test]\nfn mutable_write_capture_is_no_longer_rejected_as_snapshot_assignment() {\n    let source =\n        "fn main() -> Int { var value = 40; let set = fn() -> Int { value = 99; value }; set() }";\n    let check = run_stdin(&["check", "-"], source);\n    assert!(\n        check.status.success(),\n        "{}",\n        String::from_utf8_lossy(&check.stderr)\n    );\n    let run = run_stdin(&["run", "-"], source);\n    assert!(\n        run.status.success(),\n        "{}",\n        String::from_utf8_lossy(&run.stderr)\n    );\n    assert_eq!(String::from_utf8_lossy(&run.stdout), "99\\n");\n}\n'''
if text.count(old) != 1:
    raise SystemExit(f"snapshot CLI regression replacement count={text.count(old)}")
p.write_text(text.replace(old, new, 1))
