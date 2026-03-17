# ADR 0001: Direct Standard Ingest Canonicalization and Indexing Semantics

**Status**: Accepted
**Date**: 2026-03-17
**Feature**: `001-memory-ingest`

## Context

The first memory-ingest vertical slice accepts canonical text/markdown requests and also accepts Open Badges and CLR payloads directly at the HTTP boundary. Before implementation, the artifact set needed a durable governance record for four linked architectural decisions:

- how supported 1EdTech JSON payloads cross the ingest boundary without polluting canonical storage
- what exact-content retrieval guarantees mean for direct standard ingest
- how idempotent replay works when authoritative raw-body preservation and semantic replay detection must coexist
- how asynchronous indexing and future graph projection boundaries remain explicit without weakening authoritative write/read guarantees

The constitution requires significant architectural decisions to be captured in ADRs.

## Decision

### 1. Direct 1EdTech JSON Ingest Boundary

- The HTTP boundary accepts three request families only: canonical request, Open Badges request, and CLR request.
- Open Badges and CLR validation happens at the HTTP adapter boundary against repository-pinned JSON Schema snapshots for the supported envelope profiles.
- Protocol-specific fields remain boundary concerns only and do not appear in the canonical retrieval schema.

### 2. Canonicalization and Retrieval Guarantees

- Accepted Open Badges and CLR requests are stored as canonical `Source` records with `document_type = json`.
- Canonical `external_id` is derived from trimmed `id`; canonical `title` is derived from trimmed `name`.
- The accepted UTF-8 request body is preserved exactly as authoritative content. In concrete terms, the service stores the accepted request body string without reparsing, pretty-printing, key reordering, whitespace normalization, or silent mutation.
- Direct-standard ingest produces exactly one derived memory item with `unit_type = json_document`.
- That memory item's `content` is the preserved request body, and its offsets are `[0, canonical_content_byte_length)` using UTF-8 byte offsets.
- Retrieval guarantees apply to the authoritative stored content from the first successful registration. Replay requests do not overwrite that preserved content.

### 3. Idempotent Replay and Conflict Semantics

- Authoritative uniqueness is still keyed by `external_id`.
- Replay detection compares a deterministic normalized JSON hash of the validated standard payload rather than the preserved raw-body bytes.
- Formatting-only changes such as whitespace or object-key-order differences replay successfully to the original authoritative record.
- Semantic differences that change the normalized payload hash produce `409 Conflict`.
- Shape-valid but unmappable payloads fail with `400 INVALID_STANDARD_PAYLOAD` before any authoritative persistence.

### 4. Indexing Outbox and Deferred Indexing Semantics

- SurrealDB remains the authoritative store for sources and memory items.
- The same authoritative transaction writes a durable indexing outbox record.
- Public API responses expose indexing progress only through `queued`, `indexed`, and `deferred`.
- Internal outbox job statuses are `pending`, `processing`, `retryable`, `completed`, and `dead_letter`; they are never leaked directly to external contracts.
- `queued` means authoritative persistence succeeded and indexing work is durably accepted but not yet confirmed in Meilisearch.
- `indexed` means the projection is confirmed in Meilisearch.
- `deferred` means authoritative persistence succeeded but search confirmation is blocked by degraded dependency health or retry backlog.

### 5. Graph Projection Boundary

- The slice keeps a `GraphProjectionPort` and a no-op adapter boundary now.
- Future graph work must derive from canonical identifiers only: `source_id`, `urn`, `sequence`, `document_type`, and canonical metadata.
- No FalkorDB-specific identifiers or behaviors are introduced into authoritative storage or public retrieval contracts in this slice.

## Consequences

- The implementation can proceed without re-opening direct standard ingest semantics.
- OpenAPI, tasks, plan, quickstart, and data-model artifacts must use `document_type = json` and `unit_type = json_document` consistently for supported standard ingest.
- Performance and contract validation must cover canonical requests and direct-standard requests separately where their behavior differs.
- Future graph expansion remains additive and cannot redefine current source or retrieval semantics.