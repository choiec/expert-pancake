# Quickstart: Memory Ingest Vertical Slice

**Status**: IMPLEMENT-READY

## Purpose

This document describes the local development flow for the Rust + Axum + SurrealDB + Meilisearch memory-ingest slice.

## Prerequisites

- Rust stable toolchain with edition 2024 support
- Docker and Docker Compose or equivalent local containers
- `curl` for HTTP checks

## Proposed Environment Variables

```bash
export APP_LISTEN_ADDR=127.0.0.1:3000
export SURREALDB_URL=ws://127.0.0.1:8000/rpc
export SURREALDB_NAMESPACE=memory
export SURREALDB_DATABASE=memory
export SURREALDB_USERNAME=root
export SURREALDB_PASSWORD=root
export MEILI_HTTP_ADDR=http://127.0.0.1:7700
export MEILI_MASTER_KEY=local-dev-key
export MEMORY_INGEST_ENABLED=true
```

## Start Infrastructure

If `docker-compose.yaml` is updated for this slice, the intended command is:

```bash
docker compose up -d surrealdb meilisearch
```

Until that file is populated, equivalent local containers are:

```bash
docker run -d --name surrealdb -p 8000:8000 surrealdb/surrealdb:latest start --log trace --user root --pass root memory
docker run -d --name meilisearch -p 7700:7700 -e MEILI_MASTER_KEY=local-dev-key getmeili/meilisearch:v1.13
```

## Run the Service

```bash
cargo run -p app_server
```

Expected behavior after implementation:

- `GET /health` returns `200` from a local-only liveness check without calling SurrealDB or Meilisearch
- `GET /ready` returns `200` only when SurrealDB write-path checks succeed and returns `503` when the authoritative write path is unavailable; search degradation appears in the response body without failing readiness when the database remains ready

## Smoke Test: Canonical Ingest

```bash
curl -i http://127.0.0.1:3000/sources/register \
  -H 'content-type: application/json' \
  -H 'traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01' \
  --data '{
    "title": "Axum Plan",
    "summary": "Planning notes",
    "external-id": "demo-source-001",
    "document-type": "markdown",
    "content": "# Intro\n\nHello world\n\n# Next\n\nMore text",
    "metadata": {"topic": "planning"}
  }'
```

Expected response shape:

- `201 Created` for a first ingest
- `200 OK` for an idempotent replay with the same canonical payload
- `409 Conflict` for the same `external-id` with conflicting canonical content
- `indexing_status` returns only `queued`, `indexed`, or `deferred`

## Direct Standard Ingest Semantics

- Accepted Open Badges and CLR payloads are stored with authoritative `document_type = json`.
- The accepted UTF-8 request body is preserved exactly as stored content and emitted through one derived `json_document` memory item.
- Formatting-only replay of the same validated standard payload reuses the first authoritative record because replay detection uses normalized JSON hashing rather than raw-body bytes.

## Implementation Verification Checkpoints

- **Standard-payload validation**: verify that accepted, schema-invalid, and shape-valid-but-unmappable Open Badges and CLR fixtures produce the documented `201` or `200` versus `400` outcomes and leave no authoritative state on rejection.
- **Replay hashing**: verify that formatting-only variants of the same validated standard payload return the same `source_id` and URNs while retrieval still returns the preserved first-commit request body exactly as stored.
- **Outbox mapping**: verify that a successful registration creates durable indexing work tied to authoritative identifiers and that external responses summarize indexing progress only as `queued`, `indexed`, or `deferred`.
- **Performance gates**: verify that the benchmark or load suite asserts the published latency and throughput thresholds before staging or rollout.

## Smoke Test: Open Badges / CLR Direct Ingest

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

Expected behavior:

- `201 Created` with `document_type: json`
- exactly one returned memory item with `unit_type: json_document`
- `indexing_status: queued` when the outbox accepts work but search confirmation is still pending
- `indexing_status: deferred` when Meilisearch is unavailable but authoritative persistence succeeds

## Smoke Test: Retrieval

```bash
curl -i http://127.0.0.1:3000/sources/{source_id}
curl -i http://127.0.0.1:3000/memory-items/{urn}
```

## Smoke Test: Search

```bash
curl -i 'http://127.0.0.1:3000/search/memory-items?q=hello&limit=10'
```

Expected behavior:

- `200 OK` with search projection hits if Meilisearch is healthy and indexing completed
- `503 Service Unavailable` with structured error if search is unavailable while retrieval remains operational

Use a document-type filter to validate direct-standard projection behavior explicitly:

```bash
curl -i 'http://127.0.0.1:3000/search/memory-items?q=Rust&document-type=json&limit=10&offset=0'
```

Expected behavior:

- only projection hits are returned
- each hit contains `urn`, `source_id`, `sequence`, `document_type`, `content_preview`, and optional `score`
- no authoritative `content` field is returned from the search endpoint

## Operational Validation Notes

- Indexing backlog inspection is performed against the authoritative `memory_index_job` outbox records in SurrealDB. Operators should be able to distinguish `pending`, `retryable`, and `dead_letter` jobs.
- Public API responses summarize those internal job states as `queued`, `indexed`, or `deferred`; internal outbox vocabulary is never returned directly to clients.
- Retry exhaustion must promote jobs to `dead_letter` without affecting authoritative source or memory-item availability.
- Manual recovery must support re-indexing from authoritative SurrealDB data by replaying source identifiers from the outbox rather than depending on stale Meilisearch state.
- Performance validation is part of operational readiness for this slice: emitted metrics and benchmark or load reports must be retained as evidence that AC-P1, AC-P2, AC-P3, NC-001, NC-002, NC-003, and NC-004 were checked.

## Operator Queries

Inspect the authoritative outbox backlog:

```sql
SELECT source_id, status, retry_count, last_error, available_at, created_at, updated_at
FROM memory_index_job
ORDER BY available_at ASC, created_at ASC;
```

Inspect only exhausted jobs:

```sql
SELECT source_id, status, retry_count, last_error, updated_at
FROM memory_index_job
WHERE status = 'dead_letter'
ORDER BY updated_at DESC;
```

Requeue dead-letter jobs for manual recovery after search remediation:

```sql
UPDATE memory_index_job
SET status = 'retryable', retry_count = 0, last_error = NONE, available_at = time::now(), updated_at = time::now()
WHERE status = 'dead_letter';
```

## Benchmarks And Performance Gates

Build and run the owned performance validation:

```bash
cargo test --test memory_ingest_slo
cargo bench --bench memory_ingest_latency
```

The performance test asserts p95 and p99 thresholds for registration, retrieval, and search. The benchmark prints a reproducible report and Prometheus-compatible histogram output from the in-process metrics pipeline.

## Suggested Local Validation Sequence

1. Start SurrealDB and Meilisearch.
2. Run the Axum service.
3. Confirm `/health` and `/ready` behavior.
4. Register a canonical markdown source.
5. Retrieve the new source and one derived memory item.
6. Confirm Meilisearch receives the projection and search returns the item.
7. Replay the same ingest request and verify idempotent identifiers are returned.
8. Submit a conflicting payload with the same `external-id` and verify `409 Conflict`.
9. Register one Open Badges payload and one CLR payload, then verify each returns `document_type = json`, one `json_document` item, and exact-content retrieval.
10. Submit one schema-invalid standard payload and one shape-valid-but-unmappable standard payload and verify both return `400` without persistence.
11. Replay a formatting-only variant of a successful direct-standard payload and verify the same authoritative identifiers are returned while retrieval still exposes the first committed body.
12. Inspect the corresponding authoritative outbox state and verify public `indexing_status` remains limited to `queued`, `indexed`, or `deferred`.
13. Confirm `/health` stays `200` when dependencies are down while `/ready` reflects authoritative write-path availability.
14. Run the benchmark/load suite and confirm the emitted p95/p99 metrics satisfy AC-P1, AC-P2, AC-P3, NC-001, NC-002, NC-003, and NC-004 before staging rollout.