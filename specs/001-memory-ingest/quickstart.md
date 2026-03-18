# Quickstart: Memory Ingest Vertical Slice

**Status**: IMPLEMENTED

## Purpose

This document describes the current local development and operator workflow for the Rust + Axum + SurrealDB + Meilisearch memory-ingest slice.

## Prerequisites

- Rust stable toolchain with edition 2024 support
- Docker and Docker Compose
- `curl` for HTTP checks

## Environment Variables

Required:

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

Optional:

```bash
export SURREALDB_CONNECT_TIMEOUT_MS=5000
export SURREALDB_READY_TIMEOUT_MS=1000
export MEILI_CONNECT_TIMEOUT_MS=5000
export MEILI_READY_TIMEOUT_MS=1000
export MEMORY_MAX_REQUEST_BODY_BYTES=10485760
export MEMORY_NORMALIZATION_TIMEOUT_SECS=30
```

## Start Infrastructure

```bash
docker compose up -d surrealdb meilisearch
```

## Run the Service

```bash
cargo run -p app_server
```

## Smoke Test: Health And Readiness

```bash
curl -i http://127.0.0.1:3000/health
curl -i http://127.0.0.1:3000/ready
```

Expected behavior:

- `/health` returns `200` without probing SurrealDB or Meilisearch.
- `/ready` returns `200` only when the SurrealDB write path is ready.
- `/ready` returns `503` when SurrealDB is unavailable.
- Search degradation is reported in the `/ready` body while authoritative write readiness remains `200`.

## Smoke Test: Canonical Registration

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

Expected registration semantics:

- First ingest returns `201 Created`.
- Idempotent replay returns `200 OK` with the same `source_id` and `memory_items`.
- Conflicting replay returns `409 Conflict`.
- Public `indexing_status` is only `queued`, `indexed`, or `deferred`.

## Smoke Test: Direct Open Badges Or CLR Registration

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

Direct-standard contract guarantees:

- Accepted payloads are persisted as canonical `document_type = json`.
- Exactly one `json_document` memory item is created.
- The first committed UTF-8 request body is the authoritative retrieval payload.
- Formatting-only replay reuses the same authoritative identifiers because replay hashing uses normalized JSON, not raw-body bytes.

## Smoke Test: Authoritative Retrieval

```bash
curl -i http://127.0.0.1:3000/sources/{source_id}
curl -i http://127.0.0.1:3000/memory-items/{urn}
```

The source response returns memory items ordered by ascending `sequence`. Retrieval is authoritative and should be used to validate direct-standard preserved content and source ordering.

## Smoke Test: Search

```bash
curl -i 'http://127.0.0.1:3000/search/memory-items?q=hello&limit=10&offset=0'
curl -i 'http://127.0.0.1:3000/search/memory-items?q=Rust&document-type=json&limit=10&offset=0'
```

Expected behavior:

- `200 OK` with projection hits if Meilisearch is healthy and indexing completed.
- `503 SEARCH_UNAVAILABLE` if Meilisearch is unavailable.
- Search responses never include authoritative `content`.

## Degraded Search Exercise

1. Stop or block Meilisearch while keeping SurrealDB available.
2. Register a new source.
3. Confirm registration still succeeds and `indexing_status` is `deferred`.
4. Confirm `GET /search/memory-items` returns `503`.
5. Confirm `GET /sources/{source_id}` and `GET /memory-items/{urn}` still succeed.

## Verification Checkpoints

- **Standard-payload validation**: accepted, schema-invalid, and shape-valid-but-unmappable fixtures must match the published `201` or `200` versus `400` outcomes.
- **Replay hashing**: formatting-only variants must reuse authoritative identifiers and preserve first-commit retrieval content.
- **Outbox mapping**: committed outbox rows must rehydrate projection documents from `memory_source` and `memory_item` without semantic loss.
- **Public status translation**: only `queued`, `indexed`, or `deferred` can appear through the public API even when internal outbox state is `pending`, `processing`, `retryable`, `completed`, or `dead_letter`.
- **Performance gates**: the benchmark and SLO suite must emit p95/p99 latency, throughput, and error-rate evidence for canonical registration, direct-standard registration, 10k-item source retrieval, and search projection queries.

## Operator Queries

Inspect the authoritative outbox backlog:

```sql
SELECT job_id, source_id, status, retry_count, last_error, available_at, created_at, updated_at
FROM memory_index_job
ORDER BY available_at ASC, created_at ASC;
```

Inspect authoritative rows for a single source:

```sql
SELECT * FROM memory_source WHERE source_id = <uuid>;
SELECT * FROM memory_item WHERE source_id = <uuid> ORDER BY sequence ASC;
SELECT * FROM memory_index_job WHERE source_id = <uuid> ORDER BY created_at ASC;
```

Inspect dead-letter rows:

```sql
SELECT job_id, source_id, retry_count, last_error, updated_at
FROM memory_index_job
WHERE status = 'dead_letter'
ORDER BY updated_at DESC;
```

Requeue retryable or dead-letter rows after Meilisearch remediation:

```sql
UPDATE memory_index_job
SET status = 'retryable', retry_count = 0, last_error = NONE, available_at = time::now(), updated_at = time::now()
WHERE status IN ['retryable', 'dead_letter'];
```

Supported recovery guidance for this slice:

1. Treat `memory_source` and `memory_item` as authoritative.
2. Validate those rows before taking any recovery action.
3. Rebuild `memory_items_v1` only by requeueing durable `memory_index_job` rows keyed by authoritative `source_id`.
4. Never use stale Meilisearch documents as recovery input.

## Benchmarks And Performance Gates

Release-shaped closeout verification:

```bash
cargo test --tests
cargo test --test memory_ingest_slo -- --nocapture
cargo bench --bench memory_ingest_latency --no-run
```

This sequence covers the full contract, integration, and unit surface, executes the published SLO gate, and confirms the benchmark target is buildable before rollout.

```bash
cargo test --test memory_ingest_slo -- --nocapture
cargo bench --bench memory_ingest_latency
```

The SLO suite asserts the published pass/fail latency gates. The benchmark prints p95/p99 latency, throughput, error rate, and the Prometheus histogram snapshot for the same scenarios.

## Suggested Local Validation Sequence

1. Start SurrealDB and Meilisearch.
2. Run the Axum service.
3. Confirm `/health` and `/ready` behavior.
4. Register a canonical markdown source.
5. Retrieve the source and one memory item.
6. Confirm ordered source retrieval and authoritative preserved content.
7. Confirm Meilisearch receives the projection and search returns the item.
8. Replay the same ingest request and verify idempotent identifiers.
9. Submit a conflicting payload with the same `external-id` and verify `409 Conflict`.
10. Register Open Badges and CLR payloads and verify each returns `document_type = json` plus one `json_document` item.
11. Replay a formatting-only standard payload and verify retrieval still returns the first committed body.
12. Inspect the authoritative outbox row for the same `source_id` and confirm public `indexing_status` stays limited to `queued`, `indexed`, or `deferred`.
13. Exercise degraded search behavior.
14. Run the SLO suite and benchmark before staging rollout.