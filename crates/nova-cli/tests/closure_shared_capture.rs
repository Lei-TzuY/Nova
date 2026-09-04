use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run_stdin(arguments: &[&str], source: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nova"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("nova command starts");
    child
        .stdin
        .as_mut()
        .expect("stdin is piped")
        .write_all(source.as_bytes())
        .expect("source is written");
    child.wait_with_output().expect("nova command completes")
}

#[test]
fn mutable_write_capture_shares_one_cell_with_outer_scope() {
    let source = "fn main() -> Int { var value = 40; let bump = fn() -> Int { value = value + 1; value }; let first = bump(); let second = bump(); value + first + second }";
    let output = run_stdin(&["run", "-"], source);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "125\n");
}

#[test]
fn read_only_mutable_capture_keeps_creation_time_snapshot() {
    let source = "fn main() -> Int { var value = 40; let get = fn() -> Int { value }; value = 99; get() }";
    let output = run_stdin(&["run", "-"], source);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "40\n");
}

#[test]
fn immutable_capture_assignment_remains_rejected() {
    let source = "fn main() -> Int { let value = 40; let set = fn() -> Int { value = 99; value }; set() }";
    let output = run_stdin(&["check", "-"], source);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error[N3008]") || stderr.contains("immutable"), "{stderr}");
}
