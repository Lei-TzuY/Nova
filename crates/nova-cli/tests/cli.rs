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

fn nova_in(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nova"))
        .current_dir(directory)
        .args(arguments)
        .output()
        .expect("nova binary should execute")
}

#[test]
fn accepts_positive_fixtures() {
    for relative in [
        "valid/basic.nv",
        "valid/precedence.nv",
        "valid/assignment.nv",
        "valid/definite-assignment.nv",
        "valid/while-loop.nv",
        "valid/loop-control.nv",
        "valid/guaranteed-loop-break.nv",
        "valid/short-circuit-flow.nv",
        "valid/literal-if-flow.nv",
        "valid/constant-condition-flow.nv",
        "valid/noncontinuing-successors.nv",
        "valid/literal-match-flow.nv",
        "valid/unit.nv",
        "valid/unit-equality.nv",
        "valid/unit-main.nv",
        "valid/inspection-v2.nv",
        "valid/unreachable-warning.nv",
        "valid/payload-free-enum-equality.nv",
        "valid/function-equality.nv",
        "valid/records.nv",
        "valid/enums-match.nv",
        "valid/int-boundaries.nv",
        "valid/int-division.nv",
        "valid/radix-integers.nv",
        "valid/higher-order-functions.nv",
    ] {
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
fn run_command_executes_checked_program() {
    for (relative, expected, warning) in [
        ("valid/basic.nv", "42\n", None),
        ("valid/while-loop.nv", "5\n", None),
        ("valid/loop-control.nv", "42\n", None),
        ("valid/guaranteed-loop-break.nv", "42\n", None),
        ("valid/short-circuit-flow.nv", "42\n", None),
        ("valid/literal-if-flow.nv", "42\n", None),
        ("valid/constant-condition-flow.nv", "42\n", None),
        (
            "valid/noncontinuing-successors.nv",
            "42\n",
            Some("warning[N3033]"),
        ),
        ("valid/literal-match-flow.nv", "42\n", None),
        ("valid/unit.nv", "42\n", None),
        ("valid/unit-equality.nv", "true\n", None),
        ("valid/unit-main.nv", "()\n", None),
        ("valid/payload-free-enum-equality.nv", "true\n", None),
        ("valid/function-equality.nv", "true\n", None),
        ("valid/records.nv", "42\n", None),
        ("valid/enums-match.nv", "42\n", None),
        ("valid/int-boundaries.nv", "-9223372036854775808\n", None),
        ("valid/int-division.nv", "-21\n", None),
        ("valid/radix-integers.nv", "42\n", None),
        ("valid/higher-order-functions.nv", "42\n", None),
    ] {
        let path = fixture(relative);
        let output = nova(&["run", path.to_str().expect("fixture path is UTF-8")]);

        assert!(
            output.status.success(),
            "fixture {relative}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
        match warning {
            Some(warning) => assert!(String::from_utf8_lossy(&output.stderr).contains(warning)),
            None => assert!(output.stderr.is_empty(), "fixture {relative}"),
        }
    }
}

#[test]
fn run_command_reports_runtime_failures() {
    for (relative, code) in [
        ("runtime/overflow.nv", "N4002"),
        ("runtime/min-negate-overflow.nv", "N4002"),
        ("runtime/min-divide-overflow.nv", "N4002"),
        ("runtime/min-remainder-overflow.nv", "N4002"),
        ("runtime/divide-by-zero.nv", "N4003"),
        ("runtime/remainder-by-zero.nv", "N4003"),
        ("runtime/invalid-main.nv", "N4001"),
        ("runtime/nonterminating-loop.nv", "N4006"),
    ] {
        let path = fixture(relative);
        let output = nova(&["run", path.to_str().expect("fixture path is UTF-8")]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "fixture {relative} unexpectedly passed"
        );
        assert!(stderr.contains(code), "fixture {relative}: {stderr}");
        assert!(output.stdout.is_empty());
    }

    let missing_main = fixture("runtime/missing-main.nv");
    let output = nova(&[
        "run",
        missing_main.to_str().expect("fixture path is UTF-8"),
        "--message-format=json",
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("\"code\":\"N4001\""), "{stderr}");
    assert!(output.stdout.is_empty());
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
fn ast_command_can_inspect_a_semantically_invalid_program() {
    let path = fixture("invalid/unknown-name.nv");
    let output = nova(&["ast", path.to_str().expect("fixture path is UTF-8")]);
    let stdout = String::from_utf8(output.stdout).expect("AST output is UTF-8");

    assert!(output.status.success());
    assert!(stdout.contains("text: \"missing\""));
    assert!(output.stderr.is_empty());
}

#[test]
fn rejects_negative_fixtures_with_stable_codes() {
    for (relative, code) in [
        ("invalid/missing-return-type.nv", "N2001"),
        ("invalid/malformed-expression.nv", "N2002"),
        ("invalid/unterminated-comment.nv", "N1003"),
        ("invalid/integer-overflow.nv", "N3030"),
        ("invalid/integer-magnitude-overflow.nv", "N1004"),
        ("invalid/constant-overflow.nv", "N3031"),
        ("invalid/constant-zero-divisor.nv", "N3032"),
        ("invalid/missing-else.nv", "N2006"),
        ("invalid/unknown-name.nv", "N3003"),
        ("invalid/type-mismatch.nv", "N3004"),
        ("invalid/unit-type-mismatch.nv", "N3004"),
        ("invalid/payload-enum-equality.nv", "N3004"),
        ("invalid/assignment-type-mismatch.nv", "N3004"),
        ("invalid/immutable-assignment.nv", "N3008"),
        ("invalid/uninitialized-read.nv", "N3009"),
        ("invalid/loop-definite-assignment.nv", "N3009"),
        ("invalid/guaranteed-loop-break-uninitialized.nv", "N3009"),
        ("invalid/short-circuit-uninitialized.nv", "N3009"),
        ("invalid/literal-if-uninitialized.nv", "N3009"),
        ("invalid/literal-match-uninitialized.nv", "N3009"),
        ("invalid/loop-control-outside-loop.nv", "N3013"),
        ("invalid/missing-record-field.nv", "N3012"),
        ("invalid/non-exhaustive-match.nv", "N3023"),
        ("invalid/enum-payload-arity.nv", "N3022"),
    ] {
        let path = fixture(relative);
        let output = nova(&["check", path.to_str().expect("fixture path is UTF-8")]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "fixture {relative} unexpectedly passed"
        );
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
fn emits_semantic_diagnostics_as_json() {
    let path = fixture("invalid/unknown-name.nv");
    let output = nova(&[
        "check",
        path.to_str().expect("fixture path is UTF-8"),
        "--message-format=json",
    ]);
    let stderr = String::from_utf8(output.stderr).expect("diagnostics are UTF-8");

    assert!(!output.status.success());
    assert_eq!(stderr.lines().count(), 1);
    assert!(stderr.contains("\"code\":\"N3003\""));
    assert!(stderr.contains("\"message\":\"unknown name\""));
}

#[test]
fn warnings_are_nonfatal_for_check_run_and_inspect() {
    let path = fixture("valid/unreachable-warning.nv");
    let path = path.to_str().expect("fixture path is UTF-8");

    let checked = nova(&["check", path]);
    assert!(checked.status.success());
    assert!(checked.stdout.is_empty());
    let check_stderr = String::from_utf8(checked.stderr).expect("warning is UTF-8");
    assert!(check_stderr.contains("warning[N3033]: unreachable code"));
    assert!(check_stderr.contains("this return leaves the function"));
    assert_eq!(check_stderr.matches("warning[N3033]").count(), 1);

    let run = nova(&["run", path, "--message-format=json"]);
    assert!(run.status.success());
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42\n");
    let run_stderr = String::from_utf8(run.stderr).expect("warning JSON is UTF-8");
    assert_eq!(run_stderr.lines().count(), 1);
    assert!(run_stderr.contains("\"severity\":\"warning\""));
    assert!(run_stderr.contains("\"code\":\"N3033\""));

    for version in ["1", "2"] {
        let inspected = nova(&[
            "inspect",
            path,
            "--format=json",
            "--schema-version",
            version,
        ]);
        assert!(inspected.status.success());
        let document = String::from_utf8(inspected.stdout).expect("inspection remains UTF-8 JSON");
        assert!(document.contains(&format!("\"schema_version\": {version}")));
        assert_eq!(document.contains("\"control_flow\":"), version == "2");
        let stderr = String::from_utf8_lossy(&inspected.stderr);
        assert!(stderr.contains("warning[N3033]"));
        assert_eq!(stderr.matches("warning[N3033]").count(), 1);
    }
}

#[test]
fn inspect_command_matches_the_versioned_golden_document() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let default_output = nova_in(
        manifest,
        &[
            "inspect",
            "tests/fixtures/valid/enums-match.nv",
            "--format",
            "json",
        ],
    );
    let explicit_output = nova_in(
        manifest,
        &[
            "inspect",
            "tests/fixtures/valid/enums-match.nv",
            "--format=json",
            "--schema-version=1",
        ],
    );

    assert!(
        default_output.status.success(),
        "{}",
        String::from_utf8_lossy(&default_output.stderr)
    );
    let expected = include_str!("golden/semantic-inspection-v1.json");
    assert_eq!(
        String::from_utf8(default_output.stdout).expect("inspection output is UTF-8"),
        expected
    );
    assert!(default_output.stderr.is_empty());
    assert!(
        explicit_output.status.success(),
        "{}",
        String::from_utf8_lossy(&explicit_output.stderr)
    );
    assert_eq!(
        String::from_utf8(explicit_output.stdout).expect("inspection output is UTF-8"),
        expected
    );
    assert!(explicit_output.stderr.is_empty());
}

#[test]
fn inspect_command_emits_explicit_schema_v2_cfg_facts() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = nova_in(
        manifest,
        &[
            "inspect",
            "tests/fixtures/valid/inspection-v2.nv",
            "--format=json",
            "--schema-version=2",
        ],
    );

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("inspection output is UTF-8"),
        include_str!("golden/semantic-inspection-v2.json")
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn inspect_rejects_invalid_source_without_partial_output() {
    let path = fixture("invalid/unknown-name.nv");
    for version in ["1", "2"] {
        let output = nova(&[
            "inspect",
            path.to_str().expect("fixture path is UTF-8"),
            "--format=json",
            "--schema-version",
            version,
            "--message-format=json",
        ]);
        let stderr = String::from_utf8(output.stderr).expect("diagnostics are UTF-8");

        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(stderr.contains("\"code\":\"N3003\""), "{stderr}");
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
