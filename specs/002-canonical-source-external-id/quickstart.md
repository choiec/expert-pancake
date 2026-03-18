# Quickstart: Canonical Source External ID Rollout

## Purpose

Describe the validation workflow for implementing and verifying canonical `external_id` governance across manual and direct-standard ingest.

## Prerequisites

- Rust stable toolchain with edition 2024 support
- Docker and Docker Compose
- Local SurrealDB and Meilisearch instances as already used by the memory-ingest slice

## Validation Sequence After Implementation

1. Start infrastructure.

```bash
docker compose up -d surrealdb meilisearch
```

2. Run the service.

```bash
cargo run -p app_server
```

3. Verify canonical/manual ingest accepts only canonical URIs.

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
- response `external_id` exactly matches the submitted canonical URI
- response `source_id` is a deterministic UUID v5 derived from the canonical source seed
- `source_metadata.system.canonical_id_version = "v1"`

4. Verify malformed or non-canonical manual ids are rejected.

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
- structured validation error indicating invalid canonical external id

5. Verify direct-standard ingest derives canonical `external_id` and preserves original payload id.

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
- response `external_id` uses the project-owned namespace
- response `source_id` is deterministic for the same canonical source identity
- retrieval shows `source_metadata.system.original_standard_id = "urn:example:badge:001"`
- retrieval still returns one `json_document` memory item with the first committed raw body

6. Verify replay semantics ignore raw-formatting and raw-id spelling noise after canonicalization.

- Submit two direct-standard payloads that normalize to the same canonical URI and same semantic projection.
- Expect second request to return `200 OK` with the same `source_id` and `memory_items`.
- Confirm the first committed raw body remains authoritative through `GET /memory-items/{urn}`.

7. Run regression and performance gates.

```bash
cargo test --tests
cargo test --test memory_ingest_slo -- --nocapture
cargo bench --bench memory_ingest_latency --no-run
```

## Migration Checks

- Run the source-id migration and confirm no persisted `source_id` remains outside the UUID v5 rule.
- Confirm migrated rows remain retrievable through `/sources/{source-id}` and `/memory-items/{urn}`.
- Confirm new writes always record `canonical_id_version = "v1"`.
- Confirm no API example or contract test still treats direct-standard payload `id` as canonical `external_id` or assumes UUID v4 source ids.