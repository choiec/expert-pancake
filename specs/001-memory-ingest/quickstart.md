# Quickstart: Schema-Native Standard Credential Registry

**Status**: IMPLEMENT-READY

## Purpose

Document the local validation flow for the schema-native credential redesign.

## Prerequisites

- Rust stable toolchain with edition 2024 support
- Docker and Docker Compose or equivalent local containers
- `curl` for HTTP checks
- `cargo-nextest`, `cargo-llvm-cov`, and `cargo-mutants` installed locally

## Local RED -> GREEN -> REFACTOR -> VERIFY Loop

1. Start from the next unchecked `RED` task in `tasks.md` and write or tighten the failing test.
2. Implement the smallest change that turns that one failing proof green.
3. Refactor only after the failing proof is green.
4. Run the story gate with `just test-fast` during the inner loop and `just verify-story 001-memory-ingest` before moving the task to done.

## Environment

```bash
export APP_LISTEN_ADDR=127.0.0.1:3000
export SURREALDB_URL=ws://127.0.0.1:8000/rpc
export SURREALDB_NAMESPACE=memory
export SURREALDB_DATABASE=memory
export SURREALDB_USERNAME=root
export SURREALDB_PASSWORD=root
export MEILI_HTTP_ADDR=http://127.0.0.1:7700
export MEILI_MASTER_KEY=local-dev-key
```

## Run the Service

```bash
cargo run -p app_server
```

## Standard Validation Commands

```bash
just fmt
just lint
just test-fast
just test-full
just verify-story 001-memory-ingest
just mutants
just coverage
```

## Smoke Test: Register an Open Badges Credential

```bash
curl -i http://127.0.0.1:3000/credentials/register \
  -H 'content-type: application/json' \
  --data '{
    "@context": ["https://www.w3.org/ns/credentials/v2"],
    "type": ["VerifiableCredential", "OpenBadgeCredential"],
    "id": "urn:example:badge:001",
    "name": "Rust Badge",
    "issuer": {"id": "https://issuer.example.org"},
    "credentialSubject": {"achievement": {"name": "Rust Badge"}},
    "proof": {"type": "DataIntegrityProof"},
    "validFrom": "2026-01-01T00:00:00Z"
  }'
```

Expected behavior:

- `201 Created` for first registration
- response body is the authoritative credential document itself
- no `source_id`, `external_id`, `memory_items`, or `source_metadata` fields appear

## Smoke Test: Replay the Same Credential

Resubmit the same credential with harmless JSON formatting changes.

Expected behavior:

- `200 OK` for semantic replay
- no duplicate authoritative row is created

## Smoke Test: Retrieve an Authoritative Credential

```bash
curl -i 'http://127.0.0.1:3000/credentials/urn%3Aexample%3Abadge%3A001'
```

Expected behavior:

- retrieval succeeds by official credential `id`
- response body is the stored schema-exact credential document
- no wrapper-specific fields are returned

## Search Check

```bash
curl -i 'http://127.0.0.1:3000/credentials/search?q=rust&limit=10'
```

Expected behavior:

- search returns projection hits derived from credential data
- search degradation may return `503` without changing authoritative registration or retrieval behavior
