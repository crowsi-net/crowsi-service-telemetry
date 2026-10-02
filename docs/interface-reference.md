# crowsi-service-telemetry interface reference

Use the [usage guide](getting-started.md) for the first steps. This reference preserves the current interface details and operational limits. Run command examples from the repository root, after preparing the exact declared dependencies and registered configuration.

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

## Local setup

All paths are absolute. Each database belongs to exactly one pre-registered service. Token
generation is finite, uses the OS cryptographic random source, creates a new mode-0600 file,
and refuses an existing file or any symbolic-link path component. Runtime tokens are exactly
64 lowercase hexadecimal characters.

```bash
cargo run --locked --offline -- initialize --service coela-control \
  --expected-runtime continuous --db /absolute/state/services/coela-control/telemetry.sqlite3
cargo run --locked --offline -- token generate --output /absolute/state/read.token
cargo run --locked --offline -- token generate --output /absolute/state/ingest.token
cargo run --locked --offline -- serve --bind 127.0.0.1:4212 \
  --read-token-file /absolute/state/read.token \
  --ingest-token-file /absolute/state/ingest.token \
  --service-db coela-control=/absolute/state/services/coela-control/telemetry.sqlite3
```

Repeat `--service-db ID=ABSOLUTE_PATH` for every explicitly registered service, up to 512.
Unknown IDs are rejected and never auto-created. Read and ingest tokens must be different.

## HTTP and SSE

- `GET /health` — loopback launcher readiness; contains no service details.
- `POST /v1/telemetry/signals` — one strict health/event/metric or diagnostic event; ingest token.
- `GET /v1/telemetry/projection` — current redacted aggregate; read token.
- `GET /v1/telemetry/stream` — initial and changed projections; read token.

The SSE stream emits `event: projection`, `id: <runtimeInstanceId>:<revision>`, and one-line JSON
`data`. The 32-character instance ID is stable for one loaded runtime and changes after reload;
revision is monotonic within that generation. Polling every 500 ms makes writes, retention,
and freshness transitions visible. EventSource reconnection receives the current full snapshot;
`retry` is 1500 ms and comments provide a 15-second heartbeat. The runtime permits at most 32
concurrent HTTP/SSE connections and returns a bounded 503 response when capacity is exhausted.

An example health body is available at [`examples/health.json`](../examples/health.json). Record
a reviewed finite input without starting the runtime:

```bash
cargo run --locked --offline -- record --db /absolute/telemetry.sqlite3 \
  --input /absolute/health.json
cargo run --locked --offline -- project --db /absolute/telemetry.sqlite3
```

The local toolchain can verify the fixed authenticated HTTP/SSE boundary without opening a
database, reading a token, starting a listener, or writing state:

```bash
cargo run --locked --offline -- contract-check
```
