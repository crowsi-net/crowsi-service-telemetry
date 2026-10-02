# crowsi-service-telemetry

Retain service-owned operational signals and publish only an explicitly allowed projection.

## What you can do

- Write bounded signals into a local SQLite store.
- Expose selected, sanitized operational summaries.

## Current scope

The service owns its raw records and retention policy. Public projections must not expose private payloads or credentials.

Package distribution is not activated by this documentation. Use the checked-in source and the declared dependency versions; published availability must be verified separately.

## Getting started

Install Rust 1.97 or newer and make the declared dependencies available. Use the configured private registry when a dependency is not distributed publicly. Run from this repository:

```sh
cargo test --locked
```

## Examples and interface details

## Contracts

- `crowsi://service-telemetry/projection/v1` — current aggregate health, recent diagnostics,
  and recent metrics.
- `crowsi://service-telemetry/diagnostic-event/v1` — sensitive-value-free diagnostic input.
- `crowsi://service-telemetry/ingest-receipt/v1` — accepted-write receipt.
- SSE event `projection` — the same projection JSON, identified by runtime generation and revision.

Health keeps `liveness`, `readiness`, dependency, telemetry freshness, verification, and
operation result as independent dimensions. `expectedRuntime` is immutable registration
metadata. Producers send `validForMs`; the store derives freshness as `fresh`, `degraded`,
then `stale`, so a stopped producer cannot remain healthy indefinitely. Every signal timestamp
must be no more than five minutes ahead of the local runtime clock.

## Documentation and source

[Interface reference](docs/interface-reference.md)

[Usage guide](docs/getting-started.md)

[Examples](examples) · [Implementation and public interfaces](src) · [Verification cases](tests) · [Contributing](CONTRIBUTING.md) · [Security reporting](SECURITY.md) · [License](LICENSE) · [Attribution notices](NOTICE)
