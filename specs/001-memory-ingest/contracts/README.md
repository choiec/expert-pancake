# API Contract

**Status**: IMPLEMENT-READY

This directory contains the public HTTP contract for the first memory-ingest vertical slice.

## Contract Scope

- `POST /sources/register`
- `GET /sources/{source-id}`
- `GET /memory-items/{urn}`
- `GET /search/memory-items`
- `GET /health`
- `GET /ready`

## Contract Principles

- Authoritative retrieval is always served from SurrealDB-backed canonical records.
- Search is a Meilisearch projection and may be degraded independently. Search responses are projection hits, not authoritative memory-item payloads.
- `/health` is a local-only liveness probe. `/ready` is the dependency-aware probe for SurrealDB write-path readiness and search degradation reporting.
- Open Badges and CLR are accepted only at the ingest boundary and do not change the canonical retrieval schema. Validation uses repository-pinned JSON Schema snapshots for the supported credential envelope profiles in this slice; payloads that are shape-valid but cannot map into canonical title and external_id are rejected with HTTP 400, while accepted request bodies are preserved as authoritative canonical content exactly as submitted.
- Accepted direct-standard ingest is surfaced through `document_type = json` plus one `json_document` memory item. No semantic splitting by credential substructure occurs in this slice.
- Public indexing progress uses only `queued`, `indexed`, and `deferred`. Internal outbox job states remain implementation-only.
- Error responses use one JSON envelope with stable `error_code`, `message`, `details`, `timestamp`, and `request_id` fields.

## Coverage Expectations

- HTTP contract tests must assert every published endpoint and every published status code, including `408` for `POST /sources/register`, `200` for `/health`, and `200/503` for `/ready`.
- HTTP contract and integration coverage must explicitly include Open Badges success, CLR success, schema-invalid standard payload `400`, shape-valid-but-unmappable standard payload `400`, replay, and conflict behavior.
- Storage-adapter contract tests must complement the public HTTP contract by verifying SurrealDB authoritative guarantees and Meilisearch projection guarantees required by the constitution.
- Performance validation must use the instrumented metrics pipeline to measure p95/p99 latency and error rate against the published performance criteria.

## Contract File

- `memory-ingest.openapi.yaml`: machine-readable OpenAPI 3.1 contract for the current slice.