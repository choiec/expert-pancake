# Feature Specification: Source Document Ingestion and Memory Item Normalization

**Feature Branch**: `001-memory-ingest`  
**Created**: 2026-03-17  
**Status**: Draft  
**Scope**: First greenfield vertical slice (bootstrap)  

## Problem & Context

AI memory systems need a reliable pipeline to accept external source documents, normalize them into queryable memory units, and expose them through basic retrieval and search. Currently, there is no entry point for ingesting documents or a canonical way to transform raw sources into standardized memory items. This feature establishes the first end-to-end vertical slice: ingest → normalize → persist → retrieve.

The **ingest-persist-retrieve consistency** is the primary success criterion for this phase. Advanced capabilities (UI, broad 1EdTech coverage, graph traversal, LLM enrichment) are deferred.

This slice includes first-class ingest support for Open Badges and CLR JSON payloads at the HTTP boundary. Those payloads are validated against their originating standard shape before normalization into the internal source and memory-item model. CASE and other 1EdTech standards remain out of scope for this slice.

## Clarifications

### Session 2026-03-17

- Q: Should this slice accept 1EdTech standard JSON payloads directly at the ingest boundary? → A: Option A (confirmed) - the ingest API accepts supported 1EdTech standard JSON payloads as first-class request bodies in this slice.
- Q: Which 1EdTech JSON standard families are in scope for direct ingest in this slice? → A: Option B (confirmed) - support Open Badges and CLR direct ingest; CASE and other 1EdTech standards are deferred.

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
- **Binary or media ingest**: Only inline UTF-8 `text` and `markdown` content is in scope; file uploads, PDFs, images, audio, and binary attachments are deferred.
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

**Independent Test**: Can be fully tested by calling the registration endpoint with a valid source document, validating the response contains valid memory item identifiers, and querying the database to confirm persistence.

**Acceptance Scenarios**:

1. **Given** a valid source document with title, summary, and external ID, **When** a POST request is sent to `/sources/register`, **Then** the system returns HTTP 201 with a list of normalized memory item URNs and source metadata echoed back with a server-assigned source ID.
2. **Given** a markdown source document with multiple heading sections, **When** registration completes, **Then** each section is normalized into a separate memory item with a relationship back to the source.
3. **Given** duplicate external IDs submitted with the same canonical payload, **When** both registrations are processed, **Then** the second request receives the same authoritative identifiers and no duplicate memory items are created; if the payload conflicts, the request fails with HTTP 409.

---

### User Story 2 - Memory Item Retrieval (Priority: P1)

A Downstream Consumer queries for memory items by source ID, external ID, or memory item URN. The system returns the memory item content, metadata, and a reference to its source document.

**Why this priority**: Without retrieval, stored memory items are useless. This validates that the normalization and storage pipeline works end-to-end.

**Independent Test**: Can be fully tested by registering a source, extracting a memory item URN from the response, calling a GET endpoint with that URN, and validating that the returned item matches the expected content and metadata.

**Acceptance Scenarios**:

1. **Given** a registered source with one or more memory items, **When** a GET request is sent to `/memory-items/{urn}`, **Then** the system returns HTTP 200 with the memory item content, metadata (created timestamp, version), and a link to the source.
2. **Given** a request for a memory item that exists, **When** the response is received, **Then** the returned content is identical to what was stored during normalization (byte-accurate for text, consistent for structured metadata).
3. **Given** a request for a non-existent memory item URN, **When** a GET request is sent, **Then** the system returns HTTP 404 with a structured error message.

---

### User Story 3 - Source Metadata and Relationship Access (Priority: P1)

A Downstream Consumer queries for source metadata (registration timestamp, source type, external ID) and retrieves all memory items belonging to a source in a single request.

**Why this priority**: Memory items exist in context of their source. This validates that source-to-memory-item relationships are correctly maintained.

**Independent Test**: Can be fully tested by registering a source, knowing its source ID, calling a GET endpoint for source metadata, and validating that the returned list of memory items matches what was created during registration.

**Acceptance Scenarios**:

1. **Given** a registered source with a server-assigned source ID, **When** a GET request is sent to `/sources/{source-id}`, **Then** the system returns HTTP 200 with source metadata and an array of associated memory item URNs.
2. **Given** a source with multiple memory items, **When** the source endpoint is queried, **Then** all memory items are present and ordered by ascending `sequence`.

---

### User Story 4 - Search Projection for Basic Filtering (Priority: P2)

The system maintains a search-friendly projection of memory items in Meilisearch, allowing basic full-text and filtering queries. This projection is automatically updated whenever a memory item is indexed.

**Why this priority**: While not strictly necessary for the core pipeline, the search projection validates indexing consistency and provides a foundation for future ranking features. This is P2 because the core retrieval pipeline (Story 2) works without it.

**Independent Test**: Can be fully tested by registering a source with known keywords, waiting for indexing, calling a search API with those keywords, and validating that the correct memory items are returned.

**Acceptance Scenarios**:

1. **Given** a registered memory item with searchable content, **When** a GET request with a query term is sent to `/search/memory-items?q=keyword`, **Then** the system returns matching memory items scored by relevance.
2. **Given** a source with multiple memory items, **When** filtering by source ID with `/search/memory-items?source-id=X`, **Then** only items from that source are returned.

---

### Edge Cases

- **Oversized documents**: What happens when a source document exceeds 10 MB? System MUST reject with HTTP 413 (Payload Too Large) before attempting normalization.
- **Empty content**: What if a source has no content or only metadata? System MUST accept and create at least one memory item representing the metadata itself; empty documents do not cause failures.
- **Invalid source**: If the payload is missing required fields, uses an unsupported `document_type`, or violates UTF-8 / size constraints, the system MUST return HTTP 400 or 413 and MUST NOT persist any source or memory-item records.
- **Concurrent registration of same external ID**: When two requests register the same external ID within milliseconds, the system MUST use database-level uniqueness constraints to ensure only one source is created; a retry with the same canonical payload returns the existing `source_id` and URNs, while a conflicting payload returns HTTP 409.
- **Malformed UTF-8**: If source content contains invalid UTF-8 sequences, the system MUST either reject the request with HTTP 400 and a clear error message, or sanitize the content by replacing invalid sequences with the Unicode replacement character (U+FFFD).
- **Partial ingest failure**: If normalization or persistence fails after ingest begins, the operation MUST roll back the entire source and all derived memory items; no partial authoritative state may remain visible.
- **Normalization timeout**: If normalization (splitting into memory items) takes longer than 30 seconds, the operation MUST be canceled, rolled back, and HTTP 408 (Request Timeout) returned with a message indicating the source was too complex to process.
- **Database unavailable**: If SurrealDB is unreachable during registration, the system MUST return HTTP 503 (Service Unavailable) with a message indicating the storage backend is offline; the registration MUST NOT be partially committed.
- **Search projection failures**: If Meilisearch is unavailable or indexing fails, registration MUST still succeed after authoritative persistence completes; the response and logs MUST indicate degraded indexing status, and the item MUST be eligible for asynchronous re-index retry.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST expose an HTTP POST endpoint `/sources/register` that accepts either: (a) the canonical JSON payload containing title (required, non-empty string), external-id (required, unique string), document-type (required, enum: "text", "markdown"), content (required, non-empty string), and optional metadata (key-value pairs as JSON object); or (b) an Open Badges or CLR JSON payload as a first-class request body. `summary` is optional in the canonical payload and accepted when provided but is not part of the minimum required schema.

- **FR-002**: System MUST validate input on `/sources/register`: reject requests with missing required fields (HTTP 400), reject oversized payloads (>10 MB, HTTP 413), reject invalid UTF-8 (HTTP 400), and enforce idempotency on `external_id`. If the same `external_id` is retried with the same canonical payload, the system MUST return the existing `source_id` and memory-item URNs without creating duplicates. If the same `external_id` is submitted with materially different payload fields, the system MUST return HTTP 409.

- **FR-003**: System MUST implement a normalization service that accepts a source document and produces a list of memory items. Normalization logic MUST:
  - Split content by document-type natural boundaries: `text` by paragraph (blank-line separated), `markdown` by heading section
  - Generate unique URNs for each memory item using a deterministic, collision-free scheme (e.g., UUID v5 with source ID + content hash)
  - Associate each memory item with its source via a source_id foreign key
  - Assign a stable `sequence` (0-based integer) per item within the same source so retrieval order is deterministic
  - Persist minimum item metadata for each memory item: `sequence`, `unit_type`, `start_offset`, `end_offset`, `version`
  - Handle edge cases: empty content → create placeholders; oversized memory items → split or truncate with warnings; malformed content → sanitize or reject

- **FR-004**: System MUST persist normalized memory items to SurrealDB with the following schema (implementation-neutral description):
  - **source table**: source_id (PK), external_id (unique), title, summary, document_type, created_at, updated_at, source_metadata (JSON)
  - **memory_item table**: urn (PK), source_id (FK), sequence (integer, required), unit_type, start_offset, end_offset, version, content, content_hash, created_at, updated_at, item_metadata (JSON, optional extension fields)
  - Constraints: source_id exists in source table; each source has one or more memory_item rows; (`source_id`, `sequence`) is unique; external_id is globally unique; urns are immutable

- **FR-005**: System MUST expose an HTTP GET endpoint `/memory-items/{urn}` that retrieves a memory item by URN. Response MUST include at minimum: `urn`, `source_id`, `sequence`, `content`, `item_metadata` with `unit_type`, `start_offset`, `end_offset`, `version`, and `created_at`. The response MAY additionally include `updated_at` and parent `source_metadata`.

- **FR-006**: System MUST expose an HTTP GET endpoint `/sources/{source-id}` that retrieves source metadata and a list of all associated memory items. Response MUST include: source_id, external_id, title, summary, document_type, created_at, and an array of memory_item objects (or URNs) ordered by ascending `sequence`.

- **FR-007**: System MUST handle GET requests for non-existent resources gracefully: return HTTP 404 with a structured error response (error code, message, timestamp).

- **FR-008**: System MUST automatically index memory items in Meilisearch upon successful persistence. The search index MUST include: `urn`, `source_id`, `sequence`, `document_type`, `content_preview` (first 500 chars), `content_hash`, `created_at`, and `updated_at`. The index MUST support:
  - Full-text search on content
  - Filtering by source_id
  - Filtering by document_type
  - Sorting by created_at

- **FR-009**: System MUST expose an HTTP GET endpoint `/search/memory-items` that accepts query parameters: q (search term), source-id (filter), document-type (filter), limit (default 20, max 100), offset (for pagination). Response MUST return an array of memory items matching the query, scored by relevance (if applicable).

- **FR-010**: System MUST emit structured logs for every significant operation: source registration (success/failure), memory item creation, persistence completion, and search indexing. Logs MUST include: request_id (correlation ID), timestamp, operation, status, duration_ms, and error details (if applicable).

- **FR-011**: System MUST implement HTTP health checks at `/health` and `/ready`. The `/health` endpoint MUST return 200 if the service is running; the `/ready` endpoint MUST return 200 only if SurrealDB is reachable and responding. Both endpoints MUST include component status: { status, components: { database: ready|degraded|down, search: ready|degraded|down } }.

- **FR-012**: System MUST implement graceful error handling: all endpoints MUST return valid JSON responses with structured error objects including: error_code (string, e.g., "INVALID_INPUT"), message (human-readable string), details (optional, additional context), and timestamp.

- **FR-013**: System MUST support request tracing via W3C Trace Context headers (traceparent, tracestate). All logs and outgoing requests MUST propagate trace IDs. Traces MUST include: source registration duration, normalization duration, database write latency, and search indexing latency.

- **FR-014**: Canonical `Source` and `Memory Item` contracts MUST remain protocol-neutral after ingest normalization. Open Badges and CLR payloads MAY be accepted directly at the HTTP boundary in this slice, but the authoritative storage and retrieval schema MUST NOT require 1EdTech-, LTI-, QTI-, Caliper-, or CASE-specific fields, enums, or identifiers.

- **FR-015**: Future graph expansion MUST be additive. Any later graph projection or edge generation MUST derive from existing canonical identifiers and metadata (`source_id`, `urn`, `sequence`, `document_type`, minimum `item_metadata`) without changing their meaning or breaking retrieval/search contracts defined in this spec.

### Key Entities *(include if feature involves data)*

- **Source**: Represents the external document being registered. Contains: source_id (UUID, server-assigned), external_id (string, unique, provided by client), title, summary (optional), document_type (`text` or `markdown`), metadata (JSON, optional), created_at, updated_at. Minimum ingest contract requires title, external_id, document_type, and content. Links to zero or more memory items via source_id.

- **Memory Item**: Represents a normalized unit of content from a source. Contains: urn (URN, server-assigned, globally unique), source_id (FK to source), sequence (0-based integer, stable within source), unit_type (`paragraph` or `section`), start_offset (integer), end_offset (integer), version (string), content (string), content_hash (SHA-256 hash for deduplication), item_metadata (JSON, optional extension fields), created_at, updated_at. Immutable after creation.

- **Search Index Entry** (Meilisearch projection): Denormalized copy of memory item metadata and preview for search. Contains: urn, source_id, sequence, document_type, content_preview, content_hash, created_at, updated_at. Not authoritative; can be rebuilt from authoritative sources.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **AC-F1**: A valid source document sent to `/sources/register` returns HTTP 201 with memory item URNs and source_id. Memory items are retrievable via `/memory-items/{urn}` and reflect the exact content submitted (byte-accurate validation).

- **AC-F2**: Submitting the same `external_id` with the same canonical payload returns the same `source_id` and URNs without creating duplicate authoritative records; submitting the same `external_id` with a conflicting payload returns HTTP 409.

- **AC-F3**: Retrieving a memory item via `/memory-items/{urn}` returns the correct content and metadata matching the registered source.

- **AC-F4**: Querying `/sources/{source-id}` returns the full list of memory items associated with the source in a consistent order.

- **AC-F5**: Searching via `/search/memory-items?q=keyword` returns memory items matching the query; ranking is consistent across calls.

- **AC-F6**: All edge cases (oversized documents, empty content, malformed UTF-8, concurrent registrations) are handled without crashes or data corruption.

- **AC-P1**: Registration (including normalization and persistence) completes in under 5 seconds p95 for typical documents (< 100 KB).

- **AC-P2**: Memory item retrieval completes in under 200 ms p95.

- **AC-P3**: Search queries complete in under 500 ms p95.

- **AC-R1**: If SurrealDB becomes unavailable, registration returns HTTP 503 and stores no incomplete data; when database comes back online, subsequent registrations work without manual intervention.

- **AC-R2**: If Meilisearch becomes unavailable, registration and retrieval still work; search queries return HTTP 503 with a helpful message.

- **AC-R3**: The system can be deployed with multiple instances behind a load balancer; all instances serve consistent results for the same queries.

- **AC-O1**: Every request generates a request_id and logs all operations (start, normalization, database, search, finish) with timestamps and latencies.

- **AC-O2**: Health check endpoints (`/health`, `/ready`) respond correctly and reflect the availability of dependent services.

## Non-Functional Constraints

### Performance

- **NC-001**: Registration latency MUST NOT exceed 5 seconds p95 (including normalization and persistence). If normalization exceeds 30 seconds, the operation MUST timeout and rollback.
- **NC-002**: Memory item retrieval latency (GET `/memory-items/{urn}`) MUST NOT exceed 200 ms p95 from SurrealDB (excluding network latency).
- **NC-003**: Search queries (GET `/search/memory-items`) MUST return results in under 500 ms p95 for queries on up to 1 million memory items.
- **NC-004**: Source metadata retrieval (GET `/sources/{source-id}`) MUST NOT exceed 200 ms p95 even if the source has 10,000 associated memory items.

### Reliability & Availability

- **NC-005**: System MUST be horizontally scalable: registration and retrieval APIs MUST support multiple concurrent instances without shared mutable state (store session state in SurrealDB, not in-memory).
- **NC-006**: Registration failures (e.g., database unavailable) MUST NOT result in partial commits. Either all memory items from a source are persisted together, or the entire operation is rolled back.
- **NC-007**: Search indexing failures (Meilisearch down) MUST NOT block registration or retrieval. A separate health check endpoint MUST distinguish availability of write and search functionality.

### Observability

- **NC-008**: Every request MUST be assigned a unique request_id (correlation ID) and logged with timestamp, duration, status, and error details (if applicable). Logs MUST be structured (JSON) and include trace context.
- **NC-009**: P95 and P99 latencies for all endpoints MUST be measurable via metrics (e.g., Prometheus-compatible format). Histogram buckets: 50ms, 100ms, 200ms, 500ms, 1s, 5s.
- **NC-010**: Health check endpoint MUST be available and respond in under 100 ms; it MUST NOT rely on external services (only check local service readiness).

### Security & Data Integrity

- **NC-011**: Requests and responses MUST NOT include sensitive data (passwords, API keys, PII) in logs. If validation fails due to sensitive input, log only the validation rule that failed, not the input itself.
- **NC-012**: For this slice, content safety handling is limited to structural validation: unsupported document types and invalid UTF-8 are rejected, but valid text/markdown content is otherwise stored as submitted. Active sanitization or script stripping is deferred to a later phase and MUST NOT silently mutate accepted content in this slice.
- **NC-013**: Memory items MUST be immutable after creation (no updates, only create/delete). If correction is needed, a new source must be registered; old memory items remain queryable for audit.
- **NC-014**: All data MUST be persisted to SurrealDB (authoritative) before returning a 201/200 response to the client. Write operations MUST use transactions to ensure consistency.
- **NC-015**: Privacy baseline for this slice is data minimization: clients MUST NOT send secrets or regulated personal data unless separately approved; the service stores only the minimum canonical fields defined in this spec plus optional metadata supplied by the client.
- **NC-016**: Retention baseline for this slice is indefinite authoritative retention until an explicit delete or retention feature is specified in a later phase; no automatic TTL, purge, or archival behavior is assumed.

## Assumptions & Open Questions

### Assumptions

- **A1**: Single tenant, single deployment: Authorization (multi-tenant isolation) is handled at the API gateway level; this feature assumes all authenticated requests belong to the same logical tenant.

- **A2**: Normalization logic is simple: For the bootstrap phase, normalization simply splits content by document-type rules (markdown sections, plain-text paragraphs). No semantic analysis or LLM-based insights.

- **A3**: URN scheme is deterministic: URNs are generated using UUID v5 or similar deterministic scheme based on source_id + content hash, ensuring reproducibility (same source + content always produces the same URN).

- **A4**: Meilisearch is read-biased: Search indexing failures do NOT block registration/retrieval. Write operations to SurrealDB are authoritative; search is eventual-consistency.

- **A5**: Idempotency is keyed by `external_id` plus the canonical minimum payload shape. Retries with the same canonical payload return the same authoritative resource identity; conflicting payloads are rejected.

- **A6**: Content preservation: Registered content is stored bit-for-bit without transformation (except UTF-8 normalization). Formatting, metadata, and structure are preserved for later use.

- **A7**: Error recovery: If normalization times out (> 30 seconds), the entire operation is rolled back; no partial data persists.

- **A8**: Privacy baseline is conservative by default: the slice is intended for non-secret content, and optional metadata is expected to exclude secrets, credentials, and unnecessary personal data.

- **A9**: Retention baseline is conservative operationally but simple functionally: authoritative source and memory-item records persist until a future delete/retention feature defines a different lifecycle.

- **A10**: Direct 1EdTech ingest in this slice is limited to Open Badges and CLR credential-style JSON documents. CASE package-style payloads and other standard families are deferred because they require materially different entity and relationship handling.

### Open Questions & Clarifications

- **Pending 1**: Exact response code for successful idempotent replay (`200` vs `201`) is not material to planning but should be fixed in the API contract during `/speckit.plan`.

- **Pending 2**: Exact sanitization policy beyond structural validation (for example, whether embedded HTML/script-like text is stored verbatim or rejected) should be fixed in `/speckit.plan` together with API examples and contract tests.

## Known Unknowns

- **KU-1**: Additional optional `item_metadata` extension fields beyond the minimum required set (`sequence`, `unit_type`, `start_offset`, `end_offset`, `version`) are not yet specified and will be refined during planning.

- **KU-2**: Scaling limits are not yet validated: performance targets assume documents < 100 KB and up to 1 million memory items. Actual scaling limits with FalkorDB and graph relationships are deferred to later phases.

- **KU-3**: Audit and compliance requirements beyond the default retention/privacy baseline (jurisdiction-specific retention, legal hold, redaction, immutability proofs) are not yet specified and remain out of scope for this slice.

- **KU-4**: Internationalization (i18n): UTF-8 content is supported; error messages and field names are English. Multi-language support is deferred.

## Recommended Next Command

1. **For technical planning**: Run `/speckit.plan` to generate the implementation design for database schema, API contracts, idempotent write flow, indexing retry flow, and observability.

2. **For task generation**: After the plan is approved, run `/speckit.tasks` to break the design into executable backend, storage, and contract-test tasks.

---

**Status**: Ready for specification validation and clarification.  
**Quality Gate**: Awaiting specification validation checklist approval before proceeding.
