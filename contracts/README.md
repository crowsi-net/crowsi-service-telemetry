# Crowsi telemetry contracts

This package is the transport-neutral producer boundary for ecosystem telemetry. It contains only
Serde wire types, deterministic validation, receipt types, and the asynchronous `TelemetrySink`
port. It does not select or implement HTTP, gRPC, IPC, TLS, authentication, storage, retry,
process placement, OpenTelemetry, or a vendor exporter.

An adapter receives endpoint and authority bindings from ecosystem configuration and implements
the port. Producers therefore cannot infer an address, read a token, or fall back to a public
collector. Sink failures expose only a closed category and reason code.

Current JSON signal variants are `health`, `event`, and `metric`. The receipt schema is
`crowsi://service-telemetry/ingest-receipt/v1`. Attribute minimization remains an owner policy of
the receiving service, in addition to the contract's identifier, time, duration, and finite-number
validation.

The contract intentionally has no batch or timeline type. Batch retry/idempotency requires a
separate reviewed receipt-retention design. Workspace timeline facts use source references and CAS
revisions and must not be tunneled through operational telemetry.
