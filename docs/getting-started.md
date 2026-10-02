# Using crowsi-service-telemetry

Retain service-owned operational signals and publish only an explicitly allowed projection.

## Before you start

The service owns its raw records and retention policy. Public projections must not expose private payloads or credentials.

## First steps

Run from the repository root:

```sh
cargo test --locked
```

## How to assess the result

- Write bounded signals into a local SQLite store.
- Expose selected, sanitized operational summaries.

A passing source-level check establishes only what that check observes. Keep missing configuration, unavailable services and unverified deployment paths visible.

## Continue reading

[Repository overview](../README.md)
