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
fn mutable_source_snapshot_runs_and_requires_schema_v7() {
    let source =
        "fn main() -> Int { var value = 40; let get = fn() -> Int { value }; value = 99; get() }";
    let run = run_stdin(&["run", "-"], source);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "40\n");

    for version in ["5", "6"] {
        let output = run_stdin(
            &["inspect", "-", "--format=json", "--schema-version", version],
            source,
        );
        assert_eq!(output.status.code(), Some(1), "schema v{version}");
        assert!(output.stdout.is_empty(), "schema v{version} leaked output");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("N5001"), "{stderr}");
        assert!(stderr.contains("select schema v7"), "{stderr}");
    }

    let output = run_stdin(
        &["inspect", "-", "--format=json", "--schema-version", "7"],
        source,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("inspection output is UTF-8");
    assert!(stdout.contains("\"schema_version\": 7"));
    assert!(stdout.contains("\"mode\": \"by_value\""));
    assert!(stdout.contains("\"mutable\": true"));
}

#[test]
fn mutable_write_capture_is_no_longer_rejected_as_snapshot_assignment() {
    let source =
        "fn main() -> Int { var value = 40; let set = fn() -> Int { value = 99; value }; set() }";
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
    assert_eq!(String::from_utf8_lossy(&run.stdout), "99\n");
}
