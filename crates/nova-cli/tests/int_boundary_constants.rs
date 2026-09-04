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
fn int_min_and_max_execute_through_the_complete_pipeline() {
    let source = "fn main() -> Int { Int::MAX + Int::MIN + 43 }";

    let check = run_stdin(&["check", "-"], source);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );

    let run = run_stdin(&["run", "-"], source);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42\n");

    let inspect = run_stdin(
        &["inspect", "-", "--format=json", "--schema-version", "6"],
        source,
    );
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    assert!(!inspect.stdout.is_empty());
}

#[test]
fn unknown_or_payload_bearing_int_members_remain_rejected() {
    for source in [
        "fn main() -> Int { Int::UNKNOWN }",
        "fn main() -> Int { Int::MAX(1) }",
    ] {
        let output = run_stdin(&["check", "-"], source);
        assert!(
            !output.status.success(),
            "source unexpectedly accepted: {source}"
        );
        assert!(
            !String::from_utf8_lossy(&output.stderr).trim().is_empty(),
            "rejection should retain a deterministic diagnostic"
        );
    }
}
