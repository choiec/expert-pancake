# Quickstart: Canonical Source External ID

## Purpose

Validate the pre-production `002` end state: one canonical external identity model, deterministic `source_id`, semantic replay or conflict rules, and one active canonical-only public surface.

## Pre-production note

This repository uses reset-and-retest workflows only. Development and test data are expected to be recreated directly in the canonical `002` model.

## Prerequisites

- Rust stable toolchain with edition 2024 support
- Docker and Docker Compose
- Local SurrealDB and Meilisearch instances used by the memory-ingest slice

## Start the stack

1. Start infrastructure.

```bash
docker compose up -d surrealdb meilisearch
```

2. Run the service.

```bash
cargo run -p app_server
```

## Validate surviving 002 behavior

1. Verify canonical/manual ingest accepts only canonical URIs.

```bash
curl -i http://127.0.0.1:3000/sources/register \
  -H 'content-type: application/json' \
  --data '{
    "title": "Canonical Source",
    "external-id": "https://api.cherry-pick.net/cc/v1p3/nebooks.co.kr:eng3-ch01",
    "document-type": "markdown",
    "content": "# Heading\n\nBody"
  }'
```

Expected result:

- `201 Created`
- top-level `external_id` exactly matches the submitted canonical URI
- `source_metadata.system.canonical_id_version = "v1"`
- `source_metadata.system.ingest_kind = "canonical"`
- `source_metadata.system.semantic_payload_hash` is present
- `x-request-id` response header is present

2. Verify malformed or non-canonical manual identifiers are rejected.

```bash
curl -i http://127.0.0.1:3000/sources/register \
  -H 'content-type: application/json' \
  --data '{
    "title": "Bad Canonical Source",
    "external-id": "urn:badge:001",
    "document-type": "markdown",
    "content": "Body"
  }'
```

Expected result:

- `400 Bad Request`
- structured error body contains `request_id`
- no authoritative rows are created

3. Verify direct-standard ingest derives canonical `external_id` and preserves the original payload `id` separately.

```bash
curl -i http://127.0.0.1:3000/sources/register \
  -H 'content-type: application/json' \
  --data '{
    "@context": ["https://www.w3.org/ns/credentials/v2"],
    "type": ["VerifiableCredential", "OpenBadgeCredential"],
    "id": "urn:example:badge:001",
    "name": "Rust Badge",
    "issuer": {"id": "https://issuer.example.org"}
  }'
```

Expected result:

- `201 Created`
- top-level `external_id = "https://api.cherry-pick.net/ob/v2p0/issuer.example.org:urn%3Aexample%3Abadge%3A001"`
- `source_metadata.system.ingest_kind = "direct_standard"`
- `source_metadata.system.original_standard_id = "urn:example:badge:001"`
- `source_metadata.system.semantic_payload_hash` is present
- public response omits `raw_body_hash`

4. Verify deterministic `source_id` behavior.

- Submit the same canonical request twice.
- Expect the first response to return `201 Created` and the second to return `200 OK`.
- Confirm both responses return the same `source_id`.

5. Verify replay semantics are based only on canonical identity plus semantic payload hash.

- Submit two payloads that normalize to the same canonical URI and the same semantic projection.
- Expect the second request to return `200 OK` with the same `source_id`.
- Confirm the first stored authoritative body remains unchanged.

6. Verify conflict semantics reject semantic divergence.

- Submit a payload that resolves to the same canonical URI and a different semantic projection.
- Expect `409 Conflict`.

7. Verify provenance parity.

- Call `GET /sources/{source-id}` for a canonical/manual row and for a direct-standard row.
- Confirm `source_metadata.system` matches the registration response shape.
- Confirm only direct-standard rows expose `original_standard_id`.

8. Verify public endpoints remain limited to the active 002 surface.

- `POST /sources/register`
- `GET /sources/{source-id}`
- `GET /memory-items/{urn}`
- `GET /search/memory-items`
- `GET /health`
- `GET /ready`

## Validation commands

```bash
cargo test --tests
cargo test --test memory_ingest_slo -- --nocapture
cargo bench --bench memory_ingest_latency --no-run
```

## Expected non-features

The following behavior should not exist in this repository:

- transition-management commands or write-path state machines
- alternate lookup paths by non-canonical identifiers
- public response fields for transitional state
- superseded authoritative hash field names
- nondeterministic `source_id` generation
