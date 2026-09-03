use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
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

fn nova_with_stdin(arguments: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nova"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("nova binary should start");
    let mut stdin = child.stdin.take().expect("stdin pipe should be available");
    stdin.write_all(input).expect("source should be writable");
    drop(stdin);
    child.wait_with_output().expect("nova binary should finish")
}

#[test]
fn commands_accept_standard_input_as_the_source() {
    let source = b"fn main() -> Int { 42 }\n";

    let checked = nova_with_stdin(&["check", "-"], source);
    assert!(checked.status.success());
    assert!(checked.stdout.is_empty());
    assert!(checked.stderr.is_empty());

    let run = nova_with_stdin(&["run", "-"], source);
    assert!(run.status.success());
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42\n");
    assert!(run.stderr.is_empty());

    let ast = nova_with_stdin(&["ast", "-"], source);
    assert!(ast.status.success());
    let ast_stdout = String::from_utf8(ast.stdout).expect("AST output is UTF-8");
    assert!(ast_stdout.contains("Program {"));
    assert!(ast_stdout.contains("text: \"main\""));
    assert!(ast.stderr.is_empty());

    let inspected = nova_with_stdin(
        &["inspect", "-", "--format=json", "--schema-version=3"],
        source,
    );
    assert!(inspected.status.success());
    let document = String::from_utf8(inspected.stdout).expect("inspection output is UTF-8");
    assert!(document.contains("\"schema_version\": 3"));
    assert!(document.contains("\"name\": \"<stdin>\""));
    assert!(inspected.stderr.is_empty());
}

#[test]
fn standard_input_preserves_diagnostics_and_strict_warning_policy() {
    let unknown = nova_with_stdin(
        &["check", "-", "--message-format=json"],
        b"fn main() -> Int { missing }\n",
    );
    assert_eq!(unknown.status.code(), Some(1));
    assert!(unknown.stdout.is_empty());
    let stderr = String::from_utf8(unknown.stderr).expect("diagnostic JSON is UTF-8");
    assert!(stderr.contains("\"code\":\"N3003\""));
    assert!(stderr.contains("\"source\":\"<stdin>\""));

    let malformed = nova_with_stdin(&["check", "-"], &[b'f', b'n', 0xff]);
    assert_eq!(malformed.status.code(), Some(1));
    assert!(malformed.stdout.is_empty());
    let stderr = String::from_utf8(malformed.stderr).expect("diagnostic is UTF-8");
    assert!(stderr.contains("error[N0001]: standard input is not valid UTF-8"));
    assert!(stderr.contains("<stdin>: first invalid byte sequence begins at byte offset 2"));

    let warned = nova_with_stdin(
        &["run", "-", "--fail-on-warnings", "--message-format=json"],
        b"fn main() -> Int {\n    return 42;\n    0;\n    1\n}\n",
    );
    assert_eq!(warned.status.code(), Some(1));
    assert!(warned.stdout.is_empty());
    let stderr = String::from_utf8(warned.stderr).expect("warning JSON is UTF-8");
    assert!(stderr.contains("\"severity\":\"warning\""));
    assert!(stderr.contains("\"code\":\"N3033\""));
    assert!(stderr.contains("\"source\":\"<stdin>\""));
}

#[test]
fn standard_input_can_publish_an_explicit_source_name() {
    let display_name = "editor:///workspace/main.nv";
    let inspected = nova_with_stdin(
        &[
            "inspect",
            "-",
            "--format=json",
            "--schema-version=3",
            "--source-name",
            display_name,
        ],
        b"fn main() -> Int { 42 }\n",
    );
    assert!(inspected.status.success());
    let document = String::from_utf8(inspected.stdout).expect("inspection output is UTF-8");
    assert!(document.contains("\"schema_version\": 3"));
    assert!(document.contains(&format!("\"name\": \"{display_name}\"")));
    assert!(inspected.stderr.is_empty());

    let rejected = nova_with_stdin(
        &[
            "check",
            "-",
            "--source-name=editor:///workspace/main.nv",
            "--message-format=json",
        ],
        b"fn main() -> Int { missing }\n",
    );
    assert_eq!(rejected.status.code(), Some(1));
    assert!(rejected.stdout.is_empty());
    let stderr = String::from_utf8(rejected.stderr).expect("diagnostic JSON is UTF-8");
    assert!(stderr.contains("\"code\":\"N3003\""));
    assert!(stderr.contains(&format!("\"source\":\"{display_name}\"")));

    let malformed = nova_with_stdin(
        &["ast", "-", "--source-name", display_name],
        &[b'f', b'n', 0xff],
    );
    assert_eq!(malformed.status.code(), Some(1));
    assert!(malformed.stdout.is_empty());
    let stderr = String::from_utf8(malformed.stderr).expect("diagnostic is UTF-8");
    assert!(stderr.contains("error[N0001]: standard input is not valid UTF-8"));
    assert!(stderr.contains(&format!(
        "{display_name}: first invalid byte sequence begins at byte offset 2"
    )));
}

#[test]
fn source_name_is_rejected_for_file_input() {
    let path = fixture("valid/basic.nv");
    let output = nova(&[
        "check",
        path.to_str().expect("fixture path is UTF-8"),
        "--source-name",
        "virtual/main.nv",
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("usage error is UTF-8");
    assert!(stderr.contains("`--source-name` is only valid when the source input is `-`"));
}

#[test]
fn option_terminator_allows_hyphen_prefixed_source_paths() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is after the Unix epoch")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "nova-option-terminator-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&directory).expect("temporary directory should be creatable");
    let source = directory.join("--program.nv");
    fs::write(&source, "fn main() -> Int { 42 }\n")
        .expect("hyphen-prefixed source should be writable");

    let run = nova_in(
        &directory,
        &["run", "--message-format=json", "--", "--program.nv"],
    );
    let missing = nova_in(&directory, &["check", "--", "--missing.nv"]);
    let stdin = nova_with_stdin(&["run", "--", "-"], b"fn main() -> Int { 7 }\n");

    let _ = fs::remove_file(source);
    let _ = fs::remove_dir(directory);

    assert!(run.status.success());
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42\n");
    assert!(run.stderr.is_empty());

    assert_eq!(missing.status.code(), Some(1));
    assert!(missing.stdout.is_empty());
    let stderr = String::from_utf8(missing.stderr).expect("read diagnostic is UTF-8");
    assert!(stderr.contains("error[N0002]: could not read source file"));
    assert!(stderr.contains("--missing.nv"));

    assert!(stdin.status.success());
    assert_eq!(String::from_utf8_lossy(&stdin.stdout), "7\n");
    assert!(stdin.stderr.is_empty());
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
        "valid/pattern-payload-discard.nv",
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
        (
            "valid/literal-match-flow.nv",
            "42\n",
            Some("warning[N3034]"),
        ),
        ("valid/unit.nv", "42\n", Some("warning[N3034]")),
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
        ("valid/pattern-payload-discard.nv", "42\n", None),
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

    for version in ["1", "2", "3", "4"] {
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
        assert_eq!(document.contains("\"control_flow\":"), version != "1");
        assert_eq!(
            document.contains("\"match_patterns\":"),
            matches!(version, "3" | "4")
        );
        let stderr = String::from_utf8_lossy(&inspected.stderr);
        assert!(stderr.contains("warning[N3033]"));
        assert_eq!(stderr.matches("warning[N3033]").count(), 1);
    }
}

#[test]
fn fail_on_warnings_rejects_warnings_without_promoting_them() {
    let path = fixture("valid/unreachable-warning.nv");
    let path = path.to_str().expect("fixture path is UTF-8");

    let checked = nova(&["check", "--fail-on-warnings", path]);
    assert_eq!(checked.status.code(), Some(1));
    assert!(checked.stdout.is_empty());
    let check_stderr = String::from_utf8(checked.stderr).expect("warning is UTF-8");
    assert!(check_stderr.contains("warning[N3033]: unreachable code"));
    assert!(!check_stderr.contains("error[N3033]"));

    let run = nova(&["run", path, "--message-format=json", "--fail-on-warnings"]);
    assert_eq!(run.status.code(), Some(1));
    assert!(run.stdout.is_empty());
    let run_stderr = String::from_utf8(run.stderr).expect("warning JSON is UTF-8");
    assert_eq!(run_stderr.lines().count(), 1);
    assert!(run_stderr.contains("\"severity\":\"warning\""));
    assert!(run_stderr.contains("\"code\":\"N3033\""));

    let inspected = nova(&[
        "inspect",
        path,
        "--format=json",
        "--schema-version=3",
        "--fail-on-warnings",
    ]);
    assert_eq!(inspected.status.code(), Some(1));
    assert!(inspected.stdout.is_empty());
    let inspect_stderr = String::from_utf8(inspected.stderr).expect("warning is UTF-8");
    assert!(inspect_stderr.contains("warning[N3033]: unreachable code"));
}

#[test]
fn fail_on_warnings_preserves_clean_success_and_ordinary_errors() {
    let clean = fixture("valid/basic.nv");
    let clean = clean.to_str().expect("fixture path is UTF-8");

    let checked = nova(&["check", clean, "--fail-on-warnings"]);
    assert!(checked.status.success());
    assert!(checked.stdout.is_empty());
    assert!(checked.stderr.is_empty());

    let run = nova(&["run", "--fail-on-warnings", clean]);
    assert!(run.status.success());
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42\n");
    assert!(run.stderr.is_empty());

    let inspected = nova(&[
        "inspect",
        clean,
        "--format=json",
        "--schema-version=3",
        "--fail-on-warnings",
    ]);
    assert!(inspected.status.success());
    assert!(String::from_utf8_lossy(&inspected.stdout).contains("\"schema_version\": 3"));
    assert!(inspected.stderr.is_empty());

    let invalid = fixture("invalid/unknown-name.nv");
    let invalid = invalid.to_str().expect("fixture path is UTF-8");
    let rejected = nova(&[
        "check",
        invalid,
        "--message-format=json",
        "--fail-on-warnings",
    ]);
    assert_eq!(rejected.status.code(), Some(1));
    assert!(rejected.stdout.is_empty());
    let stderr = String::from_utf8(rejected.stderr).expect("diagnostic JSON is UTF-8");
    assert!(stderr.contains("\"severity\":\"error\""));
    assert!(stderr.contains("\"code\":\"N3003\""));
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
fn inspect_schema_v3_represents_payload_discard_without_reinterpreting_v1_or_v2() {
    let path = fixture("valid/pattern-payload-discard.nv");
    let path = path.to_str().expect("fixture path is UTF-8");

    for version in ["1", "2"] {
        let output = nova(&[
            "inspect",
            path,
            "--format=json",
            "--schema-version",
            version,
        ]);
        assert!(
            !output.status.success(),
            "legacy schema {version} must reject discard"
        );
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("N5001"), "{stderr}");
        assert!(stderr.contains("select schema v3"), "{stderr}");
    }

    for version in ["3", "4"] {
        let output = nova(&[
            "inspect",
            path,
            "--format=json",
            "--schema-version",
            version,
        ]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let stdout = String::from_utf8(output.stdout).expect("inspection output is UTF-8");
        assert!(stdout.contains(&format!("\"schema_version\": {version}")));
        assert!(stdout.contains("\"control_flow\":"));
        assert!(stdout.contains("\"match_patterns\":"));
        assert!(stdout.contains("\"payload_mode\": \"discard\""));
    }
}

#[test]
fn string_scalars_run_and_require_inspection_schema_v4() {
    let path = fixture("valid/string-scalars.nv");
    let path = path.to_str().expect("fixture path is UTF-8");

    let checked = nova(&["check", path]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert!(checked.stdout.is_empty());

    let run = nova(&["run", path]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8(run.stdout).expect("string output"),
        "Nova 🦀\nready\n"
    );

    for version in ["1", "2", "3"] {
        let output = nova(&[
            "inspect",
            path,
            "--format=json",
            "--schema-version",
            version,
        ]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("N5001"), "{stderr}");
        assert!(stderr.contains("select schema v4"), "{stderr}");
    }

    let output = nova(&["inspect", path, "--format=json", "--schema-version=4"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("v4 output is UTF-8");
    assert!(stdout.contains("\"schema_version\": 4"));
    assert!(stdout.contains("\"kind\": \"string\""));
    assert!(stdout.contains("\"display\": \"String\""));
}

#[test]
fn closures_run_and_require_inspection_schema_v5() {
    let path = fixture("valid/closures.nv");
    let path = path.to_str().expect("fixture path is UTF-8");

    let checked = nova(&["check", path]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    let run = nova(&["run", path]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8(run.stdout).expect("closure output"),
        "42\n"
    );

    for version in ["1", "2", "3", "4"] {
        let output = nova(&[
            "inspect",
            path,
            "--format=json",
            "--schema-version",
            version,
        ]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("N5001"), "{stderr}");
        assert!(stderr.contains("select schema v5"), "{stderr}");
    }

    let output = nova(&["inspect", path, "--format=json", "--schema-version=5"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("v5 output is UTF-8");
    assert!(stdout.contains("\"schema_version\": 5"));
    assert!(stdout.contains("\"kind\": \"closure\""));
    assert!(stdout.contains("\"binding\": \"binding:0\""));
    assert!(stdout.contains("\"closure_control_flow\":"));
}

#[test]
fn string_lexical_failures_have_structured_cli_diagnostics() {
    for (source, code) in [
        (
            b"fn main() -> String { \"unterminated }\n".as_slice(),
            "N1005",
        ),
        (b"fn main() -> String { \"bad\\q\" }\n".as_slice(), "N1006"),
    ] {
        let output = nova_with_stdin(&["check", "-", "--message-format=json"], source);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).expect("diagnostic JSON is UTF-8");
        assert!(stderr.contains(&format!("\"code\":\"{code}\"")), "{stderr}");
    }
}

#[test]
fn inspect_rejects_invalid_source_without_partial_output() {
    let path = fixture("invalid/unknown-name.nv");
    for version in ["1", "2", "3", "4"] {
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
