use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative)
}

fn nova(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nova"))
        .args(arguments)
        .output()
        .expect("nova binary should execute")
}

#[test]
fn accepts_positive_fixtures() {
    for relative in ["valid/basic.nv", "valid/precedence.nv"] {
        let path = fixture(relative);
        let output = nova(&["check", path.to_str().expect("fixture path is UTF-8")]);
        assert!(
            output.status.success(),
            "fixture {relative}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn ast_command_prints_a_span_preserving_tree() {
    let path = fixture("valid/basic.nv");
    let output = nova(&["ast", path.to_str().expect("fixture path is UTF-8")]);
    let stdout = String::from_utf8(output.stdout).expect("AST output is UTF-8");

    assert!(output.status.success());
    assert!(stdout.contains("Program {"));
    assert!(stdout.contains("Function {"));
    assert!(stdout.contains("text: \"main\""));
    assert!(stdout.contains("Span {"));
}

#[test]
fn rejects_negative_fixtures_with_stable_codes() {
    for (relative, code) in [
        ("invalid/missing-return-type.nv", "N2001"),
        ("invalid/malformed-expression.nv", "N2002"),
        ("invalid/unterminated-comment.nv", "N1003"),
        ("invalid/integer-overflow.nv", "N1004"),
        ("invalid/missing-else.nv", "N2006"),
    ] {
        let path = fixture(relative);
        let output = nova(&["check", path.to_str().expect("fixture path is UTF-8")]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "fixture {relative} unexpectedly passed");
        assert!(stderr.contains(code), "fixture {relative}: {stderr}");
    }
}

#[test]
fn emits_one_json_object_per_diagnostic() {
    let path = fixture("invalid/malformed-expression.nv");
    let output = nova(&[
        "check",
        path.to_str().expect("fixture path is UTF-8"),
        "--message-format=json",
    ]);
    let stderr = String::from_utf8(output.stderr).expect("diagnostics are UTF-8");

    assert!(!output.status.success());
    for line in stderr.lines() {
        assert!(line.starts_with("{\"severity\":\"error\""), "{line}");
        assert!(line.ends_with('}'), "{line}");
        assert!(line.contains("\"span\":{"), "{line}");
    }
}

#[test]
fn rejects_malformed_utf8_before_lexing() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is after the Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "nova-invalid-utf8-{}-{unique}.nv",
        std::process::id()
    ));
    fs::write(&path, [b'f', b'n', 0xff]).expect("temporary fixture should be writable");

    let output = nova(&["check", path.to_str().expect("temporary path is UTF-8")]);
    let _ = fs::remove_file(&path);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("N0001"));
    assert!(stderr.contains("byte offset 2"));
}
