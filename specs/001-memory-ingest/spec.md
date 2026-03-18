# Feature Specification: Source Document Ingestion and Memory Item Normalization

**Feature Branch**: `001-memory-ingest`  
**Created**: 2026-03-17  
**Status**: IMPLEMENT-READY  
**Scope**: First greenfield vertical slice (bootstrap)  

## Problem & Context

AI memory systems need a reliable pipeline to accept external source documents, normalize them into queryable memory units, and expose them through basic retrieval and search. Currently, there is no entry point for ingesting documents or a canonical way to transform raw sources into standardized memory items. This feature establishes the first end-to-end vertical slice: ingest → normalize → persist → retrieve.

The **ingest-persist-retrieve consistency** is the primary success criterion for this phase. Advanced capabilities (UI, broad 1EdTech coverage, graph traversal, LLM enrichment) are deferred.

This slice includes first-class ingest support for Open Badges and CLR JSON payloads at the HTTP boundary. Those payloads are validated against their originating standard shape before normalization into the internal source and memory-item model. CASE and other 1EdTech standards remain out of scope for this slice.

## Clarifications

### Session 2026-03-17

- Q: Should this slice accept 1EdTech standard JSON payloads directly at the ingest boundary? → A: Option A (confirmed) - the ingest API accepts supported 1EdTech standard JSON payloads as first-class request bodies in this slice.
- Q: Which 1EdTech JSON standard families are in scope for direct ingest in this slice? → A: Option B (confirmed) - support Open Badges and CLR direct ingest; CASE and other 1EdTech standards are deferred.
- Q: How are accepted Open Badges and CLR payloads canonicalized for authoritative storage? → A: They are stored as canonical `Source` records with `document_type = json`; the accepted UTF-8 request body is preserved exactly as received and emitted as a single derived `json_document` memory item spanning the full payload.
- Q: How do exact-content guarantees interact with idempotent replay for direct standard ingest? → A: Retrieval returns the first committed authoritative content exactly as stored; replay detection uses a deterministic normalized JSON hash of the validated payload so formatting-only changes do not create duplicate authoritative records.

## Readiness Assessment

- Artifact-wise, this feature is implement-ready: no artifact-level ambiguity or blocking issue remains across spec, plan, tasks, data model, contracts, research, or quickstart.
- The only remaining risk is implementation risk. That risk is intentionally narrowed to documented verification work for the four areas below.

### Remaining Implementation Risks

1. **Standard-payload validation**
  - Requirement anchors: FR-001, FR-002, FR-014, AC-F7, NC-012
  - Invariant: only payloads that satisfy the pinned Open Badges or CLR boundary schema and deterministically map to canonical `external_id` and `title` may create authoritative state.
  - Verification target: implementation must prove that documented allow and reject criteria match contract behavior for accepted, schema-invalid, and shape-valid-but-unmappable payloads.
2. **Replay hashing**
  - Requirement anchors: FR-002, FR-005, AC-F2, AC-F3
  - Invariant: idempotency for supported standard payloads is decided from a deterministic normalized JSON hash of the validated payload, while retrieval preserves the first committed UTF-8 request body exactly as stored.
  - Verification target: implementation must prove that formatting-only replays resolve to the same authoritative identifiers and preserved content, while semantically different payloads with the same `external_id` produce HTTP 409.
3. **Outbox mapping**
  - Requirement anchors: FR-006, FR-008, FR-009, AC-F1, AC-R2, NC-007
  - Invariant: authoritative `Source` and `MemoryItem` writes remain the source of truth, and each committed indexing job must map to projection inputs without semantic loss while external `indexing_status` exposes only `queued`, `indexed`, or `deferred`.
  - Verification target: implementation must prove that outbox records can rehydrate the intended search projection from authoritative rows and that internal job states never leak through public API responses.
4. **Performance gates**
  - Requirement anchors: AC-P1, AC-P2, AC-P3, NC-001, NC-002, NC-003, NC-004, NC-009
  - Invariant: published latency and throughput targets are release gates for this slice, not informal aspirations.
  - Verification target: implementation must prove, with instrumented measurements and threshold assertions, that representative canonical and direct-standard workloads satisfy the documented performance criteria.

## Goals

- **G1**: Establish a reliable HTTP API for registering source documents
- **G2**: Implement internal normalization logic that splits sources into memory items (minimal but standardized)
- **G3**: Persist source metadata and memory items durably to SurrealDB
- **G4**: Provide basic retrieval API to fetch memory items and their source context
- **G5**: Create minimal search projection for future ranking/filtering
- **G6**: Ensure system is operational and observable (health checks, request tracing, error handling)
- **G7**: Validate end-to-end consistency: ingest → normalize → store → retrieve pipeline works without data loss or corruption

## Non-Goals

- **UI/frontend**: This feature provides HTTP APIs only; no web interface or client UI.
- **1EdTech broad specification coverage**: Only Open Badges and CLR direct JSON ingest are in scope. CASE, QTI, full LTI/Caliper import-export, and broader 1EdTech compliance coverage are deferred; basic LTI claims support is handled in `mod_lti`, not here.
- **Advanced graph traversal**: Knowledge graph relationships and multi-hop queries are deferred to a future phase.
- **LLM-based enrichment**: AI-driven content analysis, summarization, or quality scoring is out of scope.
- **Batch operations**: Only single-document registration is in scope; bulk import/export is deferred.
- **Access control/authorization**: This phase assumes a single default tenant and no per-document permissions; auth is handled at the API gateway level.
- **Schema migrations/versioning**: No support for evolving memory item schema across deployments; initial schema is fixed.
- **Binary or media ingest**: Only inline UTF-8 `text`, `markdown`, and supported JSON/JSON-LD request bodies are in scope; file uploads, PDFs, images, audio, and binary attachments are deferred.
- **Retention automation and delete/redaction workflows**: TTL policies, purge jobs, legal hold, delete APIs, and redaction flows are out of scope for this slice.
- **Protocol-specific canonical persistence**: Even though Open Badges and CLR payloads are accepted directly at ingest, protocol-specific required fields remain out of scope for the authoritative source and memory-item storage and retrieval contracts; those payloads are normalized into the internal model after validation.

## Users & Actors

- **Source Producer**: An external system (learning platform, content management system, or API client) that registers source documents. Sends HTTP requests to register documents; expects structured responses with memory item references.
- **Downstream Consumer**: Applications or services that query memory items (search, retrieval, embedding systems). Does not appear in this feature scope but informs API contract.
- **System Operator**: Monitors health, readiness, and operational metrics; troubleshoots failures via logs and traces. Needs observability and clear error messages.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Source Document Registration (Priority: P1)

A Source Producer (e.g., learning platform backend) sends a document with metadata (title, summary, external ID, document type) to the ingestion API. The system validates input, normalizes the content, generates memory items, stores them durably, and returns a normalized response with memory item IDs and URNs for future retrieval.

**Why this priority**: This is the entry point to the entire pipeline. Without a working registration API, no memory items are created. It is the foundation for all downstream operations.

**Independent Test**: Can be fully tested by calling the registration endpoint with valid canonical, Open Badges, and CLR payloads, validating the response contains valid memory item identifiers plus an `indexing_status`, and querying the database to confirm persistence.

**Acceptance Scenarios**:

1. **Given** a valid canonical source document with title, summary, and external ID, **When** a POST request is sent to `/sources/register`, **Then** the system returns HTTP 201 with a list of normalized memory item URNs, an `indexing_status`, and source metadata echoed back with a deterministic UUID v5 source ID.
2. **Given** a markdown source document with multiple heading sections, **When** registration completes, **Then** each section is normalized into a separate memory item with a relationship back to the source.
3. **Given** a valid Open Badges or CLR payload, **When** registration completes, **Then** the authoritative source is stored with `document_type = json`, exactly one `json_document` memory item is created, and that memory item content matches the accepted UTF-8 request body exactly as stored.
4. **Given** duplicate external IDs submitted with the same canonical payload semantics, **When** both registrations are processed, **Then** the second request receives the same authoritative identifiers and no duplicate memory items are created; if the normalized payload conflicts, the request fails with HTTP 409.

---

### User Story 2 - Memory Item Retrieval (Priority: P1)

A Downstream Consumer queries for memory items by source ID, external ID, or memory item URN. The system returns the memory item content, metadata, and a reference to its source document.

**Why this priority**: Without retrieval, stored memory items are useless. This validates that the normalization and storage pipeline works end-to-end.

**Independent Test**: Can be fully tested by registering a source, extracting a memory item URN from the response, calling a GET endpoint with that URN, and validating that the returned item matches the expected content and metadata.

**Acceptance Scenarios**:

1. **Given** a registered source with one or more memory items, **When** a GET request is sent to `/memory-items/{urn}`, **Then** the system returns HTTP 200 with the memory item content, metadata (created timestamp, version), and a link to the source.
2. **Given** a request for a memory item that exists, **When** the response is received, **Then** the returned content is identical to what was stored during normalization (byte-accurate for `text`, `markdown`, and direct-standard `json_document` items relative to the authoritative stored content).
3. **Given** a request for a non-existent memory item URN, **When** a GET request is sent, **Then** the system returns HTTP 404 with a structured error message.

---

### User Story 3 - Source Metadata and Relationship Access (Priority: P1)

A Downstream Consumer queries for source metadata (registration timestamp, source type, external ID) and retrieves all memory items belonging to a source in a single request.

**Why this priority**: Memory items exist in context of their source. This validates that source-to-memory-item relationships are correctly maintained.

**Independent Test**: Can be fully tested by registering a source, knowing its source ID, calling a GET endpoint for source metadata, and validating that the returned list of memory items matches what was created during registration.

**Acceptance Scenarios**:

1. **Given** a registered source with a deterministic UUID v5 source ID, **When** a GET request is sent to `/sources/{source-id}`, **Then** the system returns HTTP 200 with source metadata and an array of associated memory item URNs.
2. **Given** a source with multiple memory items, **When** the source endpoint is queried, **Then** all memory items are present and ordered by ascending `sequence`.

---

### User Story 4 - Search Projection for Basic Filtering (Priority: P2)

The system maintains a search-friendly projection of memory items in Meilisearch, allowing basic full-text and filtering queries. This projection is automatically updated whenever a memory item is indexed.

**Why this priority**: While not strictly necessary for the core pipeline, the search projection validates indexing consistency and provides a foundation for future ranking features. This is P2 because the core retrieval pipeline (Story 2) works without it.

**Independent Test**: Can be fully tested by registering a source with known keywords, waiting for indexing, calling a search API with those keywords, and validating that the correct projection hits are returned.

**Acceptance Scenarios**:

1. **Given** a registered memory item with searchable content, **When** a GET request with a query term is sent to `/search/memory-items?q=keyword`, **Then** the system returns matching search projection hits with `urn`, `source_id`, `sequence`, `document_type`, `content_preview`, and relevance score.
2. **Given** a source with multiple memory items, **When** filtering by source ID with `/search/memory-items?source-id=X`, **Then** only projection hits from that source are returned.

---

### Edge Cases

- **Oversized documents**: What happens when a source document exceeds 10 MB? System MUST reject with HTTP 413 (Payload Too Large) before attempting normalization.
- **Empty content**: What if a source has no content or only metadata? System MUST accept and create at least one memory item representing the metadata itself; empty documents do not cause failures.
- **Invalid source**: If the payload is missing required fields, uses an unsupported `document_type`, or violates UTF-8 / size constraints, the system MUST return HTTP 400 or 413 and MUST NOT persist any source or memory-item records.
- **Concurrent registration of same external ID**: When two requests register the same external ID within milliseconds, the system MUST use database-level uniqueness constraints to ensure only one source is created; a retry with the same canonical payload returns the existing `source_id` and URNs, while a conflicting payload returns HTTP 409.
- **Malformed UTF-8**: If source content contains invalid UTF-8 sequences, the system MUST reject the request with HTTP 400 and a clear error message. This slice does not sanitize or normalize invalid text.
- **Shape-valid but unmappable standard payload**: If an Open Badges or CLR payload passes the pinned boundary schema but does not yield a non-empty canonical `external_id` and `title`, or cannot be deterministically classified to one supported payload family, the system MUST reject the request with HTTP 400 and MUST NOT persist any authoritative state.
- **Partial ingest failure**: If normalization or persistence fails after ingest begins, the operation MUST roll back the entire source and all derived memory items; no partial authoritative state may remain visible.
- **Normalization timeout**: If normalization (splitting into memory items) takes longer than 30 seconds, the operation MUST be canceled, rolled back, and HTTP 408 (Request Timeout) returned with a message indicating the source was too complex to process.
- **Database unavailable**: If SurrealDB is unreachable during registration, the system MUST return HTTP 503 (Service Unavailable) with a message indicating the storage backend is offline; the registration MUST NOT be partially committed.
- **Search projection failures**: If Meilisearch is unavailable or indexing fails, registration MUST still succeed after authoritative persistence completes; the response and logs MUST indicate degraded indexing status, and the item MUST be eligible for asynchronous re-index retry.
- **Formatting-only replay of standard JSON**: If the same Open Badges or CLR payload is retried with different whitespace or object key ordering but the deterministic normalized JSON hash is unchanged, the system MUST treat it as an idempotent replay and return the first authoritative identifiers and preserved content.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST expose an HTTP POST endpoint `/sources/register` that accepts either: (a) the canonical JSON payload containing title (required, non-empty string), external-id (required, unique string), document-type (required, enum: "text", "markdown"), content (required, non-empty string), and optional metadata (key-value pairs as JSON object); or (b) an Open Badges AchievementCredential-style JSON payload or a CLR credential-style JSON payload as a first-class request body. `summary` is optional in the canonical payload and accepted when provided but is not part of the minimum required schema. For Open Badges and CLR ingest, boundary validation MUST use repository-pinned JSON Schema snapshots derived from the supported credential envelope profiles for this slice; payloads that pass envelope validation but cannot be deterministically mapped into canonical `title` and `external_id` MUST be rejected with HTTP 400. Accepted standard payload bodies MUST be preserved as the authoritative canonical content exactly as submitted, meaning the accepted UTF-8 request body string is stored and later retrieved without reparsing, reformatting, key reordering, whitespace normalization, or silent mutation. Direct-standard ingest stores the authoritative `Source` with `document_type = json` and derives its preserved content into a single `json_document` memory item.

- **FR-002**: System MUST validate input on `/sources/register`: reject requests with missing required fields (HTTP 400), reject oversized payloads (>10 MB, HTTP 413), reject invalid UTF-8 (HTTP 400), reject supported-standard payloads that fail pinned-schema validation or canonical mapping (HTTP 400), and enforce idempotency on `external_id`. Canonical payload hashing for idempotency MUST be computed after request normalization: canonical `text` and `markdown` requests hash their canonical request shape, while supported standard payloads hash a deterministic normalized JSON form of the validated payload so whitespace or object-key-order differences do not create conflicts. If the same `external_id` is retried with the same canonical payload hash, the system MUST return the existing `source_id` and memory-item URNs without creating duplicates. If the same `external_id` is submitted with a different canonical payload hash, the system MUST return HTTP 409.

- **FR-003**: System MUST implement a normalization service that accepts a source document and produces a list of memory items. Normalization logic MUST:
  - Split content by document-type natural boundaries: `text` by paragraph (blank-line separated), `markdown` by heading section, and direct-standard `json` by a single full-document `json_document` item with no semantic splitting in this slice
  - Generate unique URNs for each memory item using a deterministic, collision-free scheme (e.g., UUID v5 with source ID + content hash)
  - Associate each memory item with its source via a source_id foreign key
  - Assign a stable `sequence` (0-based integer) per item within the same source so retrieval order is deterministic
  - Persist minimum item metadata for each memory item: `sequence`, `unit_type`, `start_offset`, `end_offset`, `version`
  - Use 0-based UTF-8 byte offsets into the authoritative canonical content for `start_offset` and `end_offset`; for direct-standard `json_document` items the offsets MUST be `[0, canonical_content_byte_length)`
  - Handle edge cases with a preserve-or-reject policy: empty content → create placeholders; requests over 10 MB → reject before normalization; invalid UTF-8 → reject; accepted `text`, `markdown`, and direct-standard `json` content is split only by the declared natural boundaries and is never sanitized, truncated, reparsed, or otherwise silently mutated

- **FR-004**: System MUST persist normalized memory items to SurrealDB with the following schema (implementation-neutral description):
  - **source table**: source_id (PK), external_id (unique), title, summary, document_type (`text`, `markdown`, or `json`), created_at, updated_at, source_metadata (JSON)
  - **memory_item table**: urn (PK), source_id (FK), sequence (integer, required), unit_type, start_offset, end_offset, version, content, content_hash, created_at, updated_at, item_metadata (JSON, optional extension fields)
  - Constraints: source_id exists in source table; each source has one or more memory_item rows; (`source_id`, `sequence`) is unique; external_id is globally unique; urns are immutable

- **FR-005**: System MUST expose an HTTP GET endpoint `/memory-items/{urn}` that retrieves a memory item by URN. Response MUST include at minimum: `urn`, `source_id`, `sequence`, `content`, `item_metadata` with `unit_type`, `start_offset`, `end_offset`, `version`, and `created_at`. The response MAY additionally include `updated_at` and parent `source_metadata`. Retrieval MUST return the authoritative stored content exactly as committed for that memory item; for direct-standard ingest this means the single `json_document` memory item returns the preserved UTF-8 request body from the first successful registration for that `external_id`.

- **FR-006**: System MUST expose an HTTP GET endpoint `/sources/{source-id}` that retrieves source metadata and a list of all associated memory items. Response MUST include: source_id, external_id, title, summary, document_type, created_at, `indexing_status`, and an array of memory_item objects (or URNs) ordered by ascending `sequence`. `indexing_status` is a derived external contract field that MUST use only `queued`, `indexed`, or `deferred`.

- **FR-007**: System MUST handle GET requests for non-existent resources gracefully: return HTTP 404 with a structured error response (error code, message, timestamp).

- **FR-008**: System MUST automatically index memory items in Meilisearch upon successful persistence. The search index MUST include: `urn`, `source_id`, `sequence`, `document_type`, `content_preview` (first 500 chars), `content_hash`, `created_at`, and `updated_at`. The index MUST support:
  - Full-text search on content
  - Filtering by source_id
  - Filtering by document_type
  - Sorting by created_at
  - Registration and source-retrieval responses MUST summarize indexing progress with `indexing_status = queued` when authoritative persistence succeeded and an index job was durably accepted but not yet confirmed in search, `indexing_status = indexed` when the projection is already confirmed available, and `indexing_status = deferred` when search is degraded or indexing must retry asynchronously. Internal outbox states such as `pending`, `processing`, `retryable`, `completed`, and `dead_letter` MUST NOT leak through the public API.

- **FR-009**: System MUST expose an HTTP GET endpoint `/search/memory-items` that accepts query parameters: q (search term), source-id (filter), document-type (filter: `text`, `markdown`, or `json`), limit (default 20, max 100), offset (for pagination). Response MUST return an array of search projection hits, each containing at minimum `urn`, `source_id`, `sequence`, `document_type`, and `content_preview`, scored by relevance when applicable. Search responses are projection results from Meilisearch and MUST NOT be presented as authoritative retrieval records.

- **FR-010**: System MUST emit structured logs for every significant operation: source registration (success/failure), memory item creation, persistence completion, and search indexing. Logs MUST include: request_id (correlation ID), timestamp, operation, status, duration_ms, and error details (if applicable).

- **FR-011**: System MUST implement HTTP probes at `/health` and `/ready` using a single probe model across artifacts. `/health` is a local-only liveness check: it MUST return HTTP 200 if the process and router are running, MUST respond in under 100 ms, and MUST NOT call external dependencies. Its response includes service-local status only. `/ready` is dependency-aware readiness: it MUST return HTTP 200 only when the authoritative SurrealDB write path is available, MUST return HTTP 503 when the write path is unavailable, and MUST include component status for `service`, `database`, and `search`. Search degradation MAY appear in `/ready` without changing the status code as long as the write path remains ready.

- **FR-012**: System MUST implement graceful error handling: all endpoints MUST return valid JSON responses with structured error objects including: error_code (string, e.g., "INVALID_INPUT"), message (human-readable string), details (optional, additional context), and timestamp.

- **FR-013**: System MUST support request tracing via W3C Trace Context headers (traceparent, tracestate). All logs and outgoing requests MUST propagate trace IDs. Traces MUST include: source registration duration, normalization duration, database write latency, and search indexing latency.

- **FR-014**: Canonical `Source` and `Memory Item` contracts MUST remain protocol-neutral after ingest normalization. Open Badges and CLR payloads MAY be accepted directly at the HTTP boundary in this slice, but the authoritative storage and retrieval schema MUST NOT require 1EdTech-, LTI-, QTI-, Caliper-, or CASE-specific fields, enums, or identifiers.

- **FR-015**: Future graph expansion MUST be additive. Any later graph projection or edge generation MUST derive from existing canonical identifiers and metadata (`source_id`, `urn`, `sequence`, `document_type`, minimum `item_metadata`) without changing their meaning or breaking retrieval/search contracts defined in this spec.

### Key Entities *(include if feature involves data)*

- **Source**: Represents the external document being registered. Contains: source_id (UUID v5, deterministically derived from the canonical source seed), external_id (string, unique, provided by client), title, summary (optional), document_type (`text`, `markdown`, or `json`), metadata (JSON, optional), created_at, updated_at. Minimum canonical ingest contract requires title, external_id, document_type, and content. Direct Open Badges and CLR ingest derive `document_type = json`, preserve the accepted UTF-8 request body exactly as authoritative content, and record ingest provenance in reserved system metadata. Every source in this slice links to one or more memory items via source_id.

- **Memory Item**: Represents a normalized unit of content from a source. Contains: urn (URN, server-assigned, globally unique), source_id (FK to source), sequence (0-based integer, stable within source), unit_type (`paragraph`, `section`, `json_document`, or `metadata_placeholder`), start_offset (integer), end_offset (integer), version (string), content (string), content_hash (SHA-256 hash for deduplication), item_metadata (JSON, optional extension fields), created_at, updated_at. Offsets are 0-based UTF-8 byte offsets into the authoritative canonical content. Direct-standard ingest always produces exactly one `json_document` memory item whose content is the preserved accepted request body. Memory items are immutable after creation.

- **Search Index Entry** (Meilisearch projection): Denormalized copy of memory item metadata and preview for search. Contains: urn, source_id, sequence, document_type, content_preview, content_hash, created_at, updated_at. `document_type` may be `text`, `markdown`, or `json`. Not authoritative; can be rebuilt from authoritative sources.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **AC-F1**: A valid source document sent to `/sources/register` returns HTTP 201 with memory item URNs, source_id, and an `indexing_status` in the public vocabulary (`queued`, `indexed`, or `deferred`). Memory items are retrievable via `/memory-items/{urn}` and reflect the authoritative stored content exactly.

- **AC-F2**: Submitting the same `external_id` with the same canonical payload hash returns the same `source_id` and URNs without creating duplicate authoritative records; submitting the same `external_id` with a conflicting canonical payload hash returns HTTP 409.

- **AC-F3**: Retrieving a memory item via `/memory-items/{urn}` returns the correct content and metadata matching the registered source. For direct-standard ingest, the single `json_document` memory item returns the preserved authoritative request body exactly as first committed.

- **AC-F4**: Querying `/sources/{source-id}` returns the full list of memory items associated with the source in a consistent order.

- **AC-F5**: Searching via `/search/memory-items?q=keyword` returns search projection hits matching the query; ranking is consistent across calls.

- **AC-F6**: All edge cases (oversized documents, empty content, malformed UTF-8, concurrent registrations) are handled without crashes or data corruption.

- **AC-F7**: Valid Open Badges and CLR payloads are accepted as direct ingest requests, produce `document_type = json` plus one `json_document` memory item, and invalid or shape-valid-but-unmappable standard payloads return HTTP 400 without partial persistence.

- **AC-V1**: Standard-payload validation correctness is verified when the documented allow and reject criteria for Open Badges and CLR requests are shown to match boundary behavior across accepted payloads, schema-invalid payloads, and shape-valid-but-unmappable payloads.

- **AC-V2**: Replay hashing determinism and idempotency are verified when semantically equivalent validated standard payloads produce the same canonical payload hash, `source_id`, and memory-item URNs across replays, while semantically different payloads with the same `external_id` return HTTP 409.

- **AC-V3**: Outbox mapping correctness is verified when each committed source registration yields durable indexing work that can rehydrate the intended projection inputs from authoritative `Source` and `MemoryItem` rows without semantic loss, and public `indexing_status` values remain limited to `queued`, `indexed`, and `deferred`.

- **AC-V4**: Performance gates are verified when representative registration, retrieval, and search workloads are executed against the instrumented metrics pipeline and fail the release gate if the published latency, throughput, or error-rate thresholds are exceeded.

- **AC-P1**: Registration (including normalization and persistence) completes in under 5 seconds p95 for typical documents (< 100 KB).

- **AC-P2**: Memory item retrieval completes in under 200 ms p95.

- **AC-P3**: Search queries complete in under 500 ms p95.

- **AC-R1**: If SurrealDB becomes unavailable, registration returns HTTP 503 and stores no incomplete data; when database comes back online, subsequent registrations work without manual intervention.

- **AC-R2**: If Meilisearch becomes unavailable, registration and retrieval still work; search queries return HTTP 503 with a helpful message.

- **AC-R3**: The system can be deployed with multiple instances behind a load balancer; all instances serve consistent results for the same queries.

- **AC-O1**: Every request generates a request_id and logs all operations (start, normalization, database, search, finish) with timestamps and latencies.

- **AC-O2**: `/health` reflects local service liveness and `/ready` reflects authoritative write-path readiness plus dependency degradation state.

## Non-Functional Constraints

### Performance

- **NC-001**: Registration latency MUST NOT exceed 5 seconds p95 (including normalization and persistence). If normalization exceeds 30 seconds, the operation MUST timeout and rollback.
- **NC-002**: Memory item retrieval latency (GET `/memory-items/{urn}`) MUST NOT exceed 200 ms p95 from SurrealDB (excluding network latency).
- **NC-003**: Search queries (GET `/search/memory-items`) MUST return results in under 500 ms p95 for queries on up to 1 million memory items.
- **NC-004**: Source metadata retrieval (GET `/sources/{source-id}`) MUST NOT exceed 200 ms p95 even if the source has 10,000 associated memory items.

### Reliability & Availability

- **NC-005**: System MUST be horizontally scalable: registration and retrieval APIs MUST support multiple concurrent instances without shared mutable state (store session state in SurrealDB, not in-memory).
- **NC-006**: Registration failures (e.g., database unavailable) MUST NOT result in partial commits. Either all memory items from a source are persisted together, or the entire operation is rolled back.
- **NC-007**: Search indexing failures (Meilisearch down) MUST NOT block registration or retrieval. Dependency-aware readiness reporting MUST distinguish authoritative write availability from degraded search availability.

### Observability

- **NC-008**: Every request MUST be assigned a unique request_id (correlation ID) and logged with timestamp, duration, status, and error details (if applicable). Logs MUST be structured (JSON) and include trace context.
- **NC-009**: P95 and P99 latencies for all endpoints MUST be measurable via metrics (e.g., Prometheus-compatible format). Histogram buckets: 50ms, 100ms, 200ms, 500ms, 1s, 5s.
- **NC-010**: The `/health` liveness endpoint MUST be available and respond in under 100 ms; it MUST check only local service readiness and MUST NOT rely on external services. Dependency-aware checks belong only to `/ready`.

### Security & Data Integrity

- **NC-011**: Requests and responses MUST NOT include sensitive data (passwords, API keys, PII) in logs. If validation fails due to sensitive input, log only the validation rule that failed, not the input itself.
- **NC-012**: For this slice, content handling is limited to structural validation and deterministic partitioning. Unsupported document types and invalid UTF-8 are rejected; accepted `text` and `markdown` content is stored exactly as submitted, and accepted direct-standard JSON content is stored as the exact UTF-8 request body string that passed boundary validation. Active sanitization, truncation, reparsing, pretty-printing, script stripping, or other silent mutation is deferred to a later phase and MUST NOT occur in this slice.
- **NC-013**: Memory items MUST be immutable after creation (no updates, only create/delete). If correction is needed, a new source must be registered; old memory items remain queryable for audit.
- **NC-014**: All data MUST be persisted to SurrealDB (authoritative) before returning a 201/200 response to the client. Write operations MUST use transactions to ensure consistency.
- **NC-015**: Privacy baseline for this slice is data minimization: clients MUST NOT send secrets or regulated personal data unless separately approved; the service stores only the minimum canonical fields defined in this spec plus optional metadata supplied by the client.
- **NC-016**: Retention baseline for this slice is indefinite authoritative retention until an explicit delete or retention feature is specified in a later phase; no automatic TTL, purge, or archival behavior is assumed.

## Assumptions & Open Questions

### Assumptions

- **A1**: Single tenant, single deployment: Authorization (multi-tenant isolation) is handled at the API gateway level; this feature assumes all authenticated requests belong to the same logical tenant.

- **A2**: Normalization logic is simple: For the bootstrap phase, normalization splits `markdown` by heading sections, `text` by blank-line paragraphs, and supported direct-standard `json` by a single full-document `json_document` item. No semantic analysis or LLM-based insights.

- **A3**: URN scheme is deterministic: URNs are generated using UUID v5 or similar deterministic scheme based on source_id + content hash, ensuring reproducibility (same source + content always produces the same URN).

- **A4**: Meilisearch is read-biased: Search indexing failures do NOT block registration/retrieval. Write operations to SurrealDB are authoritative; search is eventual-consistency.

- **A5**: Idempotency is keyed by `external_id` plus the canonical payload hash. For supported standard payloads, that hash is computed from a deterministic normalized JSON form of the validated payload, while the raw authoritative content is preserved separately for retrieval.

- **A6**: Content preservation: Accepted content is stored exactly as the accepted UTF-8 string without transformation. For canonical `text` and `markdown`, this is the submitted `content` field. For supported standard payloads, this is the raw request body string that passed validation. Invalid UTF-8 is rejected rather than normalized.

- **A7**: Error recovery: If normalization times out (> 30 seconds), the entire operation is rolled back; no partial data persists.

- **A8**: Privacy baseline is conservative by default: the slice is intended for non-secret content, and optional metadata is expected to exclude secrets, credentials, and unnecessary personal data.

- **A9**: Retention baseline is conservative operationally but simple functionally: authoritative source and memory-item records persist until a future delete/retention feature defines a different lifecycle.

- **A10**: Direct 1EdTech ingest in this slice is limited to Open Badges and CLR credential-style JSON documents. CASE package-style payloads and other standard families are deferred because they require materially different entity and relationship handling.

### Open Questions & Clarifications

- No blocking open questions remain for bootstrap implementation planning after this artifact alignment pass.

## Known Unknowns

- **KU-1**: Additional optional `item_metadata` extension fields beyond the minimum required set (`sequence`, `unit_type`, `start_offset`, `end_offset`, `version`) are not yet specified and will be refined during planning.

- **KU-2**: Scaling limits are not yet validated: performance targets assume documents < 100 KB and up to 1 million memory items. Actual scaling limits with FalkorDB and graph relationships are deferred to later phases.

- **KU-3**: Audit and compliance requirements beyond the default retention/privacy baseline (jurisdiction-specific retention, legal hold, redaction, immutability proofs) are not yet specified and remain out of scope for this slice.

- **KU-4**: Internationalization (i18n): UTF-8 content is supported; error messages and field names are English. Multi-language support is deferred.

## Recommended Next Command

1. **For implementation**: Run `/speckit.implement` against the approved tasks for this feature.

2. **For post-implementation verification**: Run `/speckit.checklist` or `/speckit.analyze` to validate implementation against these implement-ready artifacts.

---

**Status**: IMPLEMENT-READY  
**Quality Gate**: Artifact-wise ready; no blocking artifact issue remains. Residual risk is implementation-only and is explicitly tracked as standard-payload validation, replay hashing, outbox mapping, and performance-gate verification.
