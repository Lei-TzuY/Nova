use std::process::Command;

fn fixture(kind: &str, name: &str) -> String {
    format!(
        "{}/tests/fixtures/{kind}/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[test]
fn inferred_generic_identity_executes_at_multiple_types() {
    let path = fixture("valid", "generic-identity.nv");
    let check = Command::new(env!("CARGO_BIN_EXE_nova"))
        .args(["check", &path])
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );

    let run = Command::new(env!("CARGO_BIN_EXE_nova"))
        .args(["run", &path])
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42\n");
}

#[test]
fn generic_inference_conflicts_and_missing_arguments_fail_closed() {
    for (name, code) in [
        ("generic-conflict.nv", "N3037"),
        ("generic-uninferred.nv", "N3038"),
    ] {
        let path = fixture("invalid", name);
        let output = Command::new(env!("CARGO_BIN_EXE_nova"))
            .args(["check", &path])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(code),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn frozen_inspection_schema_rejects_generic_type_parameters() {
    let path = fixture("valid", "generic-identity.nv");
    let output = Command::new(env!("CARGO_BIN_EXE_nova"))
        .args(["inspect", &path, "--format=json", "--schema-version", "8"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot represent generic type parameter")
    );
}
