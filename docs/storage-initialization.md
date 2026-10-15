# Durable initialization and recovery

Schema creation, service ownership and expected runtime registration commit in one
SQLite IMMEDIATE transaction. Concurrent initialization is serialized. A failure
before commit rolls back all new schema and metadata. A lost commit acknowledgement
leaves a complete database that the same owner/runtime can initialize again.

Opening or reinitializing validates schema version, revision, required metadata and
all required telemetry table columns. Partial or foreign schemas are rejected;
initialization does not overwrite or silently repair an existing database. Different
service owners or expected runtimes remain rejected. Preserve a rejected database
for diagnosis; do not delete it as an automatic recovery step.

Run `cargo test --locked` and `cargo clippy --locked --all-targets --all-features -- -D warnings`.
`store_initialization_tests` injects failure during DDL, at each registration boundary,
and after commit; it checks rollback/retry, durable reopen and partial-schema rejection.
All fixtures use temporary private SQLite files. No live service registration changes.
