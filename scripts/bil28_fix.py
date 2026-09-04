from pathlib import Path
import re

p = Path("crates/nova-sema/src/analyzer.rs")
text = p.read_text()
text = text.replace(
    "                let assignment_entry_state = self.capture_reachable_state();\n",
    "",
    1,
)
old = """        for capture in &context.captures {
            self.record_capture_creation_read(capture);
        }"""
new = """        for capture in &context.captures {
            if capture.mode == hir::CaptureMode::ByValue {
                self.record_capture_creation_read(capture);
            }
        }"""
if text.count(old) != 1:
    raise SystemExit(f"capture creation read replacement count={text.count(old)}")
text = text.replace(old, new, 1)
p.write_text(text)

p = Path("crates/nova-interpreter/src/lib.rs")
text = p.read_text()
text = text.replace(
    "enum RuntimeCapture {\n    ByValue(Value),\n    ByReference(usize),\n}",
    "pub enum RuntimeCapture {\n    ByValue(Value),\n    ByReference(usize),\n}",
    1,
)
old = """                    && closure
                        .captures
                        .iter()
                        .zip(captures)
                        .all(|(capture, value)| self.value_conforms_to_type(value, &capture.ty))"""
new = """                    && closure.captures.iter().zip(captures).all(|(capture, value)| {
                        match (capture.mode, value) {
                            (CaptureMode::ByValue, RuntimeCapture::ByValue(value)) => {
                                self.value_conforms_to_type(value, &capture.ty)
                            }
                            (CaptureMode::ByReference, RuntimeCapture::ByReference(cell)) => self
                                .shared_cells
                                .get(*cell)
                                .and_then(Option::as_ref)
                                .is_some_and(|value| self.value_conforms_to_type(value, &capture.ty)),
                            _ => false,
                        }
                    })"""
if text.count(old) != 1:
    raise SystemExit(f"closure conformity replacement count={text.count(old)}")
text = text.replace(old, new, 1)

old = "pub enum RuntimeCapture {\n    ByValue(Value),\n    ByReference(usize),\n}"
new = "pub enum RuntimeCapture {\n    ByValue(Box<Value>),\n    ByReference(usize),\n}"
if text.count(old) != 1:
    raise SystemExit(f"RuntimeCapture boxing replacement count={text.count(old)}")
text = text.replace(old, new, 1)
text = text.replace(
    "RuntimeCapture::ByValue(value.clone())",
    "RuntimeCapture::ByValue(Box::new(value.clone()))",
)
text = text.replace(
    "self.bind_runtime_slot(&mut frame, &binding, Some(value), capture.first_use)?;",
    "self.bind_runtime_slot(&mut frame, &binding, Some(*value), capture.first_use)?;",
    1,
)
old = "const MAX_CALL_DEPTH: usize = 64;"
new = "const MAX_CALL_DEPTH: usize = 48;"
if text.count(old) != 1:
    raise SystemExit(f"call-depth budget replacement count={text.count(old)}")
text = text.replace(old, new, 1)
p.write_text(text)

p = Path("crates/nova-cli/tests/closure_snapshot.rs")
text = p.read_text()
old = '''#[test]\nfn assignment_through_snapshot_has_human_and_json_diagnostics() {\n    let source =\n        "fn main() -> Int { var value = 40; let set = fn() -> Int { value = 99; value }; set() }";\n    let human = run_stdin(&["check", "-"], source);\n    assert_eq!(human.status.code(), Some(1));\n    assert!(human.stdout.is_empty());\n    let stderr = String::from_utf8_lossy(&human.stderr);\n    assert!(stderr.contains("error[N3035]"), "{stderr}");\n    assert!(stderr.contains("captured by value"), "{stderr}");\n\n    let json = run_stdin(&["check", "-", "--message-format=json"], source);\n    assert_eq!(json.status.code(), Some(1));\n    assert!(json.stdout.is_empty());\n    let stderr = String::from_utf8_lossy(&json.stderr);\n    assert!(stderr.contains("\\\"code\\\":\\\"N3035\\\""), "{stderr}");\n    assert!(stderr.contains("lexical snapshot capture"), "{stderr}");\n}\n'''
new = '''#[test]\nfn mutable_write_capture_is_no_longer_rejected_as_snapshot_assignment() {\n    let source =\n        "fn main() -> Int { var value = 40; let set = fn() -> Int { value = 99; value }; set() }";\n    let check = run_stdin(&["check", "-"], source);\n    assert!(\n        check.status.success(),\n        "{}",\n        String::from_utf8_lossy(&check.stderr)\n    );\n    let run = run_stdin(&["run", "-"], source);\n    assert!(\n        run.status.success(),\n        "{}",\n        String::from_utf8_lossy(&run.stderr)\n    );\n    assert_eq!(String::from_utf8_lossy(&run.stdout), "99\\n");\n}\n'''
if text.count(old) != 1:
    raise SystemExit(f"snapshot CLI regression replacement count={text.count(old)}")
p.write_text(text.replace(old, new, 1))

p = Path("crates/nova-sema/tests/closures.rs")
text = p.read_text()
text = text.replace(
    "use nova_sema::hir::{ExpressionKind, StatementKind, Type};",
    "use nova_sema::hir::{CaptureMode, ExpressionKind, StatementKind, Type};",
    1,
)

def replace_test(name: str, body: str) -> None:
    global text
    pattern = rf"#\[test\]\nfn {name}\(\) \{{.*?\n\}}\n(?=\n#\[test\])"
    text, count = re.subn(pattern, body.rstrip() + "\n", text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"test replacement {name} count={count}")

replace_test(
    "rejects_assignment_through_mutable_outer_snapshot_capture",
    '''#[test]
fn mutable_outer_write_upgrades_capture_to_by_reference() {
    let output = analyze_text(
        "fn main() -> Int { var value = 40; let set = fn() -> Int { value = 99; value }; set() }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let StatementKind::Binding { initializer, .. } =
        &output.program.functions[0].body.statements[1].kind
    else {
        panic!("closure binding");
    };
    let ExpressionKind::Closure(closure) = &initializer.kind else {
        panic!("closure initializer");
    };
    assert_eq!(closure.captures.len(), 1);
    assert_eq!(closure.captures[0].mode, CaptureMode::ByReference);
}''',
)
replace_test(
    "rejected_snapshot_assignment_does_not_initialize_the_outer_binding",
    '''#[test]
fn write_capture_does_not_initialize_outer_binding_before_call() {
    let output = analyze_text(
        "fn main() -> Int { var value: Int; let set = fn() -> Unit { value = 1; }; value }",
    );
    let uninitialized = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "N3009")
        .count();
    assert_eq!(uninitialized, 1, "{:?}", output.diagnostics);
}''',
)
replace_test(
    "rejected_snapshot_assignment_rolls_back_rhs_initialization",
    '''#[test]
fn by_reference_assignment_preserves_rhs_initialization() {
    let output = analyze_text(
        "fn main() -> Int { var outer = 0; let set = fn() -> Int { var local: Int; outer = { local = 1; 0 }; local }; set() }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}''',
)
replace_test(
    "rejected_snapshot_assignment_preserves_noncontinuing_rhs_flow",
    '''#[test]
fn by_reference_assignment_preserves_noncontinuing_rhs_flow() {
    let output = analyze_text(
        "fn stop() -> ! { while true {} }\n\\
         fn main() -> Int { var outer = 0; let set = fn() -> Int { outer = stop(); 42 }; 0 }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let StatementKind::Binding { initializer, .. } =
        &output.program.functions[1].body.statements[1].kind
    else {
        panic!("closure binding");
    };
    let ExpressionKind::Closure(closure) = &initializer.kind else {
        panic!("closure initializer");
    };
    assert_eq!(closure.body.ty, Type::Never);
    let StatementKind::Assignment { value, .. } = &closure.body.statements[0].kind else {
        panic!("by-reference assignment");
    };
    assert_eq!(value.ty, Type::Never);
}''',
)
p.write_text(text)
