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
fn uint_executes_through_check_run_and_inspect() {
    let source = "fn main() -> UInt { UInt::from(40) + UInt::from(2) }";
    assert!(run_stdin(&["check", "-"], source).status.success());
    let run = run_stdin(&["run", "-"], source);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "42\
"
    );
    let inspect = run_stdin(
        &["inspect", "-", "--format=json", "--schema-version", "6"],
        source,
    );
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    let json = String::from_utf8_lossy(&inspect.stdout);
    assert!(json.contains("\\\"uint\\\""));
    assert!(json.contains("numeric_conversion"));
}

#[test]
fn uint_boundaries_and_checked_narrowing_execute() {
    let max = run_stdin(&["run", "-"], "fn main() -> UInt { UInt::MAX }");
    assert!(
        max.status.success(),
        "{}",
        String::from_utf8_lossy(&max.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&max.stdout),
        "18446744073709551615\
"
    );
    let narrow = run_stdin(
        &["run", "-"],
        "fn main() -> Int { Int::from_uint(UInt::from(42)) }",
    );
    assert!(
        narrow.status.success(),
        "{}",
        String::from_utf8_lossy(&narrow.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&narrow.stdout),
        "42\
"
    );
}

#[test]
fn explicit_cross_family_conversions_are_checked() {
    for source in [
        "fn main() -> UInt { UInt::from(-1) }",
        "fn main() -> Int { Int::from_uint(UInt::MAX) }",
    ] {
        let output = run_stdin(&["run", "-"], source);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("N4007"));
    }
}

#[test]
fn uint_arithmetic_is_checked_and_zero_division_is_rejected() {
    for (source, code) in [
        ("fn main() -> UInt { UInt::MAX + UInt::from(1) }", "N4002"),
        ("fn main() -> UInt { UInt::MIN - UInt::from(1) }", "N4002"),
        ("fn main() -> UInt { UInt::from(4) / UInt::MIN }", "N4003"),
    ] {
        let output = run_stdin(&["run", "-"], source);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(code));
    }
}

#[test]
fn uint_has_no_implicit_int_conversion_and_rejects_bad_members() {
    for source in [
        "fn main() -> UInt { UInt::from(1) + 2 }",
        "fn main() -> UInt { UInt::from(true) }",
        "fn main() -> UInt { UInt::from }",
        "fn main() -> UInt { UInt::wat }",
    ] {
        let output = run_stdin(&["check", "-"], source);
        assert!(
            !output.status.success(),
            "source unexpectedly accepted: {source}"
        );
        assert!(!String::from_utf8_lossy(&output.stderr).trim().is_empty());
    }
}
