use std::process::Command;

#[test]
#[allow(clippy::too_many_lines)]
fn malformed_options_fail_before_state_or_token_changes() {
    let dir = tempfile::tempdir().expect("valid test fixture");
    let db = dir.path().join("telemetry.sqlite");
    let output = dir.path().join("token");
    let db = db.to_str().expect("valid test fixture");
    let output = output.to_str().expect("valid test fixture");
    let cases = vec![
        vec![
            "initialize",
            "--service",
            "test",
            "--db",
            db,
            "--expected-runtime",
            "continuous",
            "--typo",
            "secret-canary",
        ],
        vec![
            "initialize",
            "--service",
            "test",
            "--db",
            db,
            "--db",
            db,
            "--expected-runtime",
            "continuous",
        ],
        vec![
            "initialize",
            "--service",
            "test",
            "--db",
            db,
            "--expected-runtime",
        ],
        vec!["token", "generate", "--output", output, "extra"],
        vec!["token", "generate", "--output", output, "--output", output],
        vec![
            "record",
            "--db",
            db,
            "--input",
            "/missing",
            "--typo",
            "secret-canary",
        ],
        vec!["project", "--db", db, "--db", db],
        vec!["query", "--db", db, "--typo"],
        vec![
            "serve",
            "--bind",
            "127.0.0.1:0",
            "--bind",
            "127.0.0.1:1",
            "--read-token-file",
            output,
            "--ingest-token-file",
            output,
            "--service-db",
            "test=/missing",
        ],
        vec![
            "serve",
            "--bind",
            "127.0.0.1:0",
            "--read-token-file",
            output,
            "--ingest-token-file",
            output,
            "--service-db",
            "test=/missing",
            "--service-db",
        ],
        vec![
            "serve",
            "--bind",
            "127.0.0.1:0",
            "--read-token-file",
            output,
            "--ingest-token-file",
            output,
            "--service-db",
            "test=/missing",
            "--service-db",
            "test=/other",
        ],
    ];
    for args in cases {
        let result = Command::new(env!("CARGO_BIN_EXE_crowsi-service-telemetry"))
            .args(args)
            .output()
            .expect("valid test fixture");
        assert!(!result.status.success());
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(!error.contains("secret-canary"));
        assert!(
            !error.contains("No such file"),
            "argv must fail before file access: {error}"
        );
        assert_eq!(
            std::fs::read_dir(dir.path())
                .expect("valid test fixture")
                .count(),
            0
        );
    }
}

#[test]
fn valid_order_independent_options_preserve_initialize_record_and_query() {
    let dir = tempfile::tempdir().expect("valid test fixture");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("valid test fixture");
    }
    let db = dir.path().join("telemetry.sqlite");
    let token = dir.path().join("token");
    for args in [
        vec![
            "initialize",
            "--expected-runtime",
            "continuous",
            "--db",
            db.to_str().expect("valid test fixture"),
            "--service",
            "test",
        ],
        vec!["project", "--db", db.to_str().expect("valid test fixture")],
        vec![
            "token",
            "generate",
            "--output",
            token.to_str().expect("valid test fixture"),
        ],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_crowsi-service-telemetry"))
            .args(args)
            .output()
            .expect("valid test fixture");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
