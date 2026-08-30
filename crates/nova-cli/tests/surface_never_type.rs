use std::path::Path;
use std::process::Command;

fn fixture() -> &'static str {
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/valid/surface-never-type.nv"
    )
}

#[test]
fn check_run_and_all_inspection_versions_accept_surface_never() {
    let checked = Command::new(env!("CARGO_BIN_EXE_nova"))
        .args(["check", fixture()])
        .output()
        .expect("nova check should execute");
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    let run = Command::new(env!("CARGO_BIN_EXE_nova"))
        .args(["run", fixture()])
        .output()
        .expect("nova run should execute");
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42\n");

    for version in ["1", "2", "3"] {
        let inspected = Command::new(env!("CARGO_BIN_EXE_nova"))
            .args([
                "inspect",
                fixture(),
                "--format=json",
                "--schema-version",
                version,
            ])
            .output()
            .expect("nova inspect should execute");
        assert!(
            inspected.status.success(),
            "schema {version}: {}",
            String::from_utf8_lossy(&inspected.stderr)
        );
        let json = String::from_utf8(inspected.stdout).expect("inspection output is UTF-8");
        assert!(
            json.contains("\"kind\": \"never\""),
            "schema {version}: {json}"
        );
        assert!(
            json.contains("\"display\": \"!\""),
            "schema {version}: {json}"
        );
    }

    assert!(Path::new(fixture()).exists());
}
