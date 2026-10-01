use std::process::Command;

#[test]
fn contract_check_is_finite_read_only_and_declares_the_authenticated_sse_boundary() {
    let output = Command::new(env!("CARGO_BIN_EXE_crowsi-service-telemetry"))
        .arg("contract-check")
        .output()
        .expect("contract-check process");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("one JSON report");
    assert_eq!(
        report["schema"],
        "crowsi://service-telemetry/contract-check/v1"
    );
    assert_eq!(report["status"], "ok");
    assert_eq!(report["mode"], "validate");
    assert_eq!(report["execution"]["runtime"], "not-started");
    assert_eq!(report["execution"]["network"], "not-accessed");
    assert_eq!(report["execution"]["fileWrites"], "none");
    assert_eq!(report["execution"]["secretResolution"], "not-resolved");
    assert_eq!(report["execution"]["externalActions"], "none");
    assert_eq!(
        report["authentication"]["locality"],
        "ipv4-loopback-required"
    );
    assert_eq!(
        report["authentication"]["tokenPolicy"],
        "read-ingest-separated"
    );
    assert_eq!(report["sseEvent"], "projection");

    let routes = report["routes"].as_array().expect("closed route list");
    assert!(routes.iter().any(|route| {
        route["method"] == "GET"
            && route["path"] == "/v1/telemetry/stream"
            && route["authorization"] == "read-token"
            && route["transport"] == "sse"
    }));
    assert!(routes.iter().any(|route| {
        route["method"] == "POST"
            && route["path"] == "/v1/telemetry/signals"
            && route["authorization"] == "ingest-token"
    }));
}

#[test]
fn contract_check_rejects_unbounded_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_crowsi-service-telemetry"))
        .args(["contract-check", "--write"])
        .output()
        .expect("contract-check process");
    assert!(!output.status.success());
}
