# Memory Ingest Workspace

This workspace ships the `001-memory-ingest` vertical slice:

- `POST /sources/register` accepts canonical JSON plus direct Open Badges and CLR JSON.
- `GET /sources/{source-id}` and `GET /memory-items/{urn}` read authoritative SurrealDB-backed records.
- `GET /search/memory-items` reads a non-authoritative Meilisearch projection.
- `GET /health` is local-only liveness.
- `GET /ready` is dependency-aware readiness for the authoritative write path.
- Request tracing and Prometheus-compatible latency histograms are emitted for every public endpoint.

## Local Setup

Required environment:

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

Optional runtime controls:

```bash
export SURREALDB_CONNECT_TIMEOUT_MS=5000
export SURREALDB_READY_TIMEOUT_MS=1000
export MEILI_CONNECT_TIMEOUT_MS=5000
export MEILI_READY_TIMEOUT_MS=1000
export MEMORY_MAX_REQUEST_BODY_BYTES=10485760
export MEMORY_NORMALIZATION_TIMEOUT_SECS=30
```

Start local dependencies:

```bash
docker compose up -d surrealdb meilisearch
```

Run the service:

```bash
cargo run -p app_server
```

## Smoke Test

Probe liveness and readiness:

```bash
curl -i http://127.0.0.1:3000/health
curl -i http://127.0.0.1:3000/ready
```

Expected behavior:

- `/health` always reflects only local service liveness.
- `/ready` returns `200` when SurrealDB write checks succeed.
- `/ready` returns `503` when the authoritative write path is unavailable.
- Search degradation can appear in the `/ready` body without changing a ready write-path to `503`.

Register a canonical markdown source:

```bash
curl -i http://127.0.0.1:3000/sources/register \
	-H 'content-type: application/json' \
	-H 'traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01' \
	--data '{
		"title": "Axum Plan",
		"summary": "Planning notes",
		"external-id": "https://api.cherry-pick.net/cc/v1p3/example.edu:demo-source-001",
		"document-type": "markdown",
		"content": "# Intro\n\nHello world\n\n# Next\n\nMore text",
		"metadata": {"topic": "planning"}
	}'
```

Project governance treats `external-id` as the canonical external URI under the
project-owned namespace. For direct-standard ingest, the payload's original
standard `id` remains provenance metadata and is not the canonical external ID.

Register a direct-standard Open Badges payload:

```bash
curl -i http://127.0.0.1:3000/sources/register \
	-H 'content-type: application/json' \
	--data '{
		"@context": ["https://www.w3.org/ns/credentials/v2"],
		"type": ["VerifiableCredential", "OpenBadgeCredential"],
		"id": "urn:badge:demo-001",
		"name": "Rust Badge",
		"issuer": {"id": "https://issuer.example.org"}
	}'
```

Expected registration contract:

- First successful ingest returns `201 Created`.
- Idempotent replay of the same semantic payload hash returns `200 OK` with the same `source_id` and `memory_items`.
- Conflicting replay for the same `external-id` returns `409 Conflict`.
- Public `indexing_status` is always one of `queued`, `indexed`, or `deferred`.
- Direct-standard success always returns `document_type: json` and exactly one `json_document` memory item.
- Public provenance is exposed under `source_metadata.system` with `canonical_id_version`, `ingest_kind`, `semantic_payload_hash`, and `original_standard_id` when present.

Retrieve authoritative records:

```bash
curl -i http://127.0.0.1:3000/sources/{source_id}
curl -i http://127.0.0.1:3000/memory-items/{urn}
```

The source response returns memory items ordered by ascending `sequence`. For direct-standard ingest, the single memory item returns the first committed UTF-8 request body exactly as stored.

Search the projection:

```bash
curl -i 'http://127.0.0.1:3000/search/memory-items?q=Hello&limit=10&offset=0'
curl -i 'http://127.0.0.1:3000/search/memory-items?q=Rust&document-type=json&limit=10&offset=0'
```

Search is projection-only. Results contain `urn`, `source_id`, `sequence`, `document_type`, `content_preview`, and optional `score`. The endpoint never returns authoritative `content`.

Exercise degraded search behavior:

1. Stop or firewall Meilisearch while leaving SurrealDB available.
2. Submit a new registration request.
3. Confirm the registration still succeeds and returns `indexing_status: deferred`.
4. Call `GET /search/memory-items` and confirm it returns `503 SEARCH_UNAVAILABLE`.
5. Call `GET /sources/{source_id}` or `GET /memory-items/{urn}` and confirm authoritative retrieval still works.

## Operator Runbook

### Inspect the authoritative indexing backlog

```sql
SELECT job_id, source_id, status, retry_count, last_error, available_at, created_at, updated_at
FROM memory_index_job
ORDER BY available_at ASC, created_at ASC;
```

Internal outbox states are `pending`, `processing`, `retryable`, `completed`, and `dead_letter`. Public APIs never expose those values directly; they translate them to `queued`, `indexed`, or `deferred`.

### Inspect authoritative source data for a queued or failed source

```sql
SELECT * FROM memory_source WHERE source_id = <uuid>;
SELECT * FROM memory_item WHERE source_id = <uuid> ORDER BY sequence ASC;
SELECT * FROM memory_index_job WHERE source_id = <uuid> ORDER BY created_at ASC;
```

Use the authoritative `memory_source` and `memory_item` rows to verify what should exist in Meilisearch. Do not use projection state as the source of truth during recovery.

### Retry exhaustion and dead-letter interpretation

- `retryable` means the worker will attempt the job again after `available_at`.
- `dead_letter` means retry exhaustion or a non-retryable indexing failure.
- `last_error` contains the sanitized failure summary that caused the latest transition.
- Authoritative retrieval remains available in both states.

Inspect only exhausted jobs:

```sql
SELECT job_id, source_id, retry_count, last_error, updated_at
FROM memory_index_job
WHERE status = 'dead_letter'
ORDER BY updated_at DESC;
```

### Manual re-index recovery

Supported recovery path for this slice:

1. Stop traffic or stop the app instance that is running the indexing worker.
2. Fix the Meilisearch dependency issue.
3. Validate the authoritative rows in `memory_source` and `memory_item` for the affected `source_id` values.
4. Requeue the durable outbox rows from SurrealDB.

```sql
UPDATE memory_index_job
SET status = 'retryable', retry_count = 0, last_error = NONE, available_at = time::now(), updated_at = time::now()
WHERE status IN ['retryable', 'dead_letter'];
```

5. Start the app again and allow the worker to drain the queue.
6. Verify the rebuilt projection with `GET /search/memory-items` and authoritative spot checks through `GET /sources/{source_id}`.

### Full projection rebuild from authoritative data

Only the following rebuild path is supported in this slice:

1. Stop the indexing worker.
2. Clear or recreate the `memory_items_v1` index in Meilisearch.
3. Requeue durable `memory_index_job` rows by `source_id` from SurrealDB.
4. Restart the worker.

Projection documents are always rehydrated from authoritative `memory_source` and `memory_item` rows via outbox `source_id`; stale Meilisearch documents are not used for rebuild.

## Validation Commands

Release-shaped closeout verification:

```bash
cargo test --tests
cargo test --test memory_ingest_slo -- --nocapture
cargo bench --bench memory_ingest_latency --no-run
```

This sequence validates the full contract, integration, and unit surface, runs the published SLO gate, and confirms the benchmark target remains executable.

Core verification suites:

```bash
cargo test --test register_source_replay_hashing --test indexing_outbox_mapping_contract --test indexing_status_mapping_flow --test observability_tracing_flow --test observability_metrics
```

Search and contract coverage:

```bash
cargo test --test search_memory_items_contract --test meilisearch_projection_contract --test search_projection_flow --test openapi_smoke
```

Performance gate:

```bash
cargo test --test memory_ingest_slo -- --nocapture
```

Benchmark report:

```bash
cargo bench --bench memory_ingest_latency
```