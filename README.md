# Memory Ingest Workspace

This workspace implements the `001-memory-ingest` vertical slice: canonical and supported direct-standard registration, authoritative retrieval from SurrealDB, non-authoritative search projection in Meilisearch, readiness probes, request tracing, and histogram-backed latency metrics.

## Local Setup

Export the local development environment variables:

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

Start the local dependencies:

```bash
docker compose up -d surrealdb meilisearch
```

Run the service:

```bash
cargo run -p app_server
```

## Smoke Test

Probe the service:

```bash
curl -i http://127.0.0.1:3000/health
curl -i http://127.0.0.1:3000/ready
```

Register a canonical source:

```bash
curl -i http://127.0.0.1:3000/sources/register \
	-H 'content-type: application/json' \
	-H 'traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01' \
	--data '{
		"title": "Axum Plan",
		"summary": "Planning notes",
		"external-id": "demo-source-001",
		"document-type": "markdown",
		"content": "# Intro\n\nHello world\n\n# Next\n\nMore text"
	}'
```

Search the projection:

```bash
curl -i 'http://127.0.0.1:3000/search/memory-items?q=Hello&limit=10&offset=0'
```

Retrieve the authoritative records:

```bash
curl -i http://127.0.0.1:3000/sources/{source_id}
curl -i http://127.0.0.1:3000/memory-items/{urn}
```

## Indexing Operations

Backlog inspection uses the authoritative `memory_index_job` outbox records. A representative query is:

```sql
SELECT source_id, status, retry_count, last_error, available_at, created_at, updated_at
FROM memory_index_job
ORDER BY available_at ASC, created_at ASC;
```

Expected internal states are `pending`, `processing`, `retryable`, `completed`, and `dead_letter`. Public APIs never expose those values directly; they collapse to `queued`, `indexed`, or `deferred`.

Retry exhaustion is visible when a row moves to `dead_letter` and `last_error` is populated. Authoritative source and memory-item reads remain available in that state.

Manual recovery and re-indexing use authoritative SurrealDB rows plus the outbox only:

1. Stop traffic or stop the app instance that is processing the worker.
2. If you need a full projection rebuild, delete the `memory_items_v1` index contents in Meilisearch.
3. Requeue failed work by updating authoritative outbox rows, for example:

```sql
UPDATE memory_index_job
SET status = 'retryable', retry_count = 0, last_error = NONE, available_at = time::now(), updated_at = time::now()
WHERE status = 'dead_letter';
```

4. Start the app again. The background indexing worker rehydrates projection documents from `memory_source` and `memory_item`, not from stale Meilisearch state.

## Validation Commands

Contract and integration coverage for the search slice:

```bash
cargo test --test search_memory_items_contract --test meilisearch_projection_contract --test search_projection_flow --test indexing_status_mapping_flow --test observability_tracing_flow --test observability_metrics
```

Performance gate:

```bash
cargo test --test memory_ingest_slo
```

Lightweight benchmark report:

```bash
cargo bench --bench memory_ingest_latency
```