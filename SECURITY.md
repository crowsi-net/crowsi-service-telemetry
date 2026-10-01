# Security policy

## Boundary

The runtime binds only an IPv4 loopback address. Projection/SSE and ingestion use distinct
Bearer tokens loaded from absolute, owner-owned mode-0600 files with no symbolic-link ancestor.
Tokens are never accepted in
URLs, environment values, SQLite, event attributes, logs, or responses. The unauthenticated
health route returns only `{ "status": "ready" }`.

Each SQLite database is bound to one service identifier. The aggregate runtime opens only
explicit `ID=path` bindings, verifies that the registered owner matches, and rejects unknown
services. It never discovers databases or creates them during ingestion.

## Data minimization

Do not submit credentials, authentication payloads, user identifiers, repository paths,
network addresses, packet content, prompts, or customer data. Attributes use a closed
allow-list, duplicate and unknown keys are dropped, values are bounded, and secret-shaped
values, URLs, queries, and values outside the key-specific syntax are dropped before persistence.
A diagnostic event declaring
`contains_sensitive_values: true` is rejected.

Raw WebAuthn authenticator data, credential IDs, challenges, signatures, origins, and user
handles are outside every public type. A diagnostic reason code is not authentication proof
and does not authorize an operation.

## Availability and trust

Missing telemetry is `unknown`; an expired observation transitions through degraded to stale.
Neither state alone is proof that a service is unavailable or healthy.
Telemetry is operational context, not independent security evidence. SQLite uses WAL,
foreign-key checks, full synchronization, a busy timeout, bounded count retention, and bounded
age retention. Projection reads perform a cheap retention check and write only when physical
pruning and its revision increment are required. Equal or older health observations cannot
replace the first accepted observation. The SSE stream publishes only minimized projections and
keeps no browser credential.

Deploying this runtime beyond one owner-controlled local session requires a new transport and
threat-model review. Do not expose the loopback endpoint through a reverse proxy.

## Private vulnerability reporting

Report vulnerabilities through this repository's GitHub private vulnerability reporting form. Do not put credentials, personal or customer data, or production certificate material in public issues or pull requests.
