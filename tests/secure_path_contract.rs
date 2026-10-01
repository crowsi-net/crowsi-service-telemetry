#![cfg(unix)]

use crowsi_service_telemetry::{
    ExpectedRuntime, RuntimeConfig, TelemetryRuntime, TelemetryStore, generate_runtime_token,
};
use std::io::Write;
use std::net::SocketAddr;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::fs::symlink;

#[test]
fn database_and_new_token_reject_a_symlinked_ancestor() {
    let temporary = tempfile::tempdir().expect("tempdir");
    let physical = temporary.path().join("physical");
    std::fs::create_dir(&physical).expect("physical directory");
    let alias = temporary.path().join("alias");
    symlink(&physical, &alias).expect("ancestor symlink");
    assert!(
        TelemetryStore::initialize(
            &alias.join("database/state.sqlite3"),
            "service",
            ExpectedRuntime::Continuous
        )
        .is_err()
    );
    assert!(generate_runtime_token(&alias.join("security/read.token")).is_err());
}

#[test]
fn runtime_token_read_rejects_a_symlinked_ancestor() {
    let temporary = tempfile::tempdir().expect("tempdir");
    let physical = temporary.path().join("physical");
    let database = physical.join("database/state.sqlite3");
    TelemetryStore::initialize(&database, "service", ExpectedRuntime::Continuous)
        .expect("database");
    let read = physical.join("security/read.token");
    let ingest = physical.join("security/ingest.token");
    generate_runtime_token(&read).expect("read token");
    generate_runtime_token(&ingest).expect("ingest token");
    let alias = temporary.path().join("alias");
    symlink(&physical, &alias).expect("ancestor symlink");
    let config = RuntimeConfig {
        bind: "127.0.0.1:4212".parse::<SocketAddr>().expect("bind"),
        service_databases: vec![("service".into(), database)],
        read_token_file: alias.join("security/read.token"),
        ingest_token_file: ingest,
    };
    assert!(TelemetryRuntime::load(&config).is_err());
}

#[test]
fn runtime_accepts_only_exact_lowercase_64_hex_tokens() {
    let temporary = tempfile::tempdir().expect("tempdir");
    let database = temporary.path().join("state/service.sqlite3");
    TelemetryStore::initialize(&database, "service", ExpectedRuntime::Continuous)
        .expect("database");
    let read = temporary.path().join("state/read.token");
    let ingest = temporary.path().join("state/ingest.token");
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&read)
        .expect("invalid read token");
    file.write_all(&[b'A'; 64]).expect("uppercase token");
    drop(file);
    generate_runtime_token(&ingest).expect("ingest token");
    let config = RuntimeConfig {
        bind: "127.0.0.1:4212".parse().expect("bind"),
        service_databases: vec![("service".into(), database)],
        read_token_file: read.clone(),
        ingest_token_file: ingest,
    };
    assert!(TelemetryRuntime::load(&config).is_err());
    std::fs::write(&read, "a".repeat(65)).expect("oversized token");
    assert!(TelemetryRuntime::load(&config).is_err());
}
