use std::process::Command;

fn fixture() -> &'static str {
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/valid/bare-unit-return.nv"
    )
}

#[test]
fn check_run_ast_and_existing_inspection_schemas_accept_bare_unit_return() {
    for command in ["check", "ast"] {
        let output = Command::new(env!("CARGO_BIN_EXE_nova"))
            .args([command, fixture()])
            .output()
            .expect("nova command executes");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let run = Command::new(env!("CARGO_BIN_EXE_nova"))
        .args(["run", fixture()])
        .output()
        .expect("nova run executes");
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42\n");

    for version in ["1", "2", "3"] {
        let output = Command::new(env!("CARGO_BIN_EXE_nova"))
            .args([
                "inspect",
                fixture(),
                "--format=json",
                "--schema-version",
                version,
            ])
            .output()
            .expect("nova inspect executes");
        assert!(
            output.status.success(),
            "schema {version}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
