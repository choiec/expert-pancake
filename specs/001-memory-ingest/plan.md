# Implementation Plan: Source Document Ingestion and Memory Item Normalization

**Branch**: `001-memory-ingest` | **Date**: 2026-03-17 | **Spec**: `specs/001-memory-ingest/spec.md`
**Status**: IMPLEMENT-READY
**Input**: Feature specification from `/workspaces/rust/specs/001-memory-ingest/spec.md`
**Related ADR**: `/workspaces/rust/specs/001-memory-ingest/adr/0001-direct-standard-ingest.md`

## Merge Note

- This plan now carries the canonical source identity concerns that were split into `002-canonical-source-external-id` on `main`.
- No separate `specs/002-*` folder is created in this branch; the merged plan remains under `001-memory-ingest`.

## Technical Summary

This vertical slice adds the first production-shaped ingest pipeline for canonical memory storage and canonical source identity governance: Axum handlers accept canonical/manual, Open Badges, or CLR JSON at the HTTP boundary; adapter validators translate supported payloads into a protocol-neutral canonical `Source` plus derived `MemoryItem` values; an application service normalizes content with a preserve-or-reject policy, persists authoritative state transactionally in SurrealDB, records durable indexing work, and exposes retrieval APIs from authoritative storage. Canonical/manual requests must already carry a project-owned canonical `external_id`; direct-standard requests derive canonical `external_id`, preserve the original upstream identifier as provenance, and compute replay semantics from semantic payload equality rather than raw formatting. Meilisearch is used only as a search projection and never as a source of truth.

## Technical Context

**Language/Version**: Rust stable, edition 2024  
**Primary Dependencies**: `axum`, `tokio`, `tower`, `tower-http`, `serde`, `serde_json`, `validator`, `uuid`, `sha2`, `tracing`, `tracing-subscriber`, `thiserror`, `surrealdb`, `meilisearch-sdk`  
**Storage**: SurrealDB for authoritative source and memory-item persistence plus durable indexing outbox; Meilisearch for search projection only  
**Testing**: `cargo test` with unit, integration, API contract, and storage adapter contract coverage  
**Target Platform**: Linux containerized web service behind an API gateway  
**Project Type**: Multi-crate Rust web service  
**Performance Goals**: registration under 5 seconds p95 for typical payloads, retrieval under 200 ms p95, search under 500 ms p95  
**Constraints**: thin handlers only, canonical domain model first, SurrealDB writes must be transactional, Meilisearch failure must not block writes, single-document ingest only, 10 MB max request body, 30 second normalization timeout  
**Scale/Scope**: single-tenant first slice, horizontally scalable stateless app instances, up to 1M indexed memory items, sources with up to 10k derived items

## Constitution Alignment Check

*Gate status before Phase 0 research: PASS*

- **Layered handler/service/repository separation**: Pass. Handlers stay in `app_server` as HTTP adapters only. Use-case orchestration lives in `mod_memory::application`, and SurrealDB/Meilisearch logic is hidden behind repository and indexing ports.
- **Canonical domain model first**: Pass. Canonical identifiers and shared invariants are defined before transport or storage DTOs. Protocol-specific Open Badges and CLR concerns stay at the ingest adapter boundary.
- **Storage responsibility separation**: Pass. SurrealDB remains authoritative, Meilisearch is explicitly non-authoritative, and FalkorDB is not introduced as a runtime dependency in this slice.
- **Observability and explicit errors**: Pass. Request IDs, W3C trace context propagation, structured JSON errors, and endpoint latency metrics are first-class design requirements.
- **Testing discipline**: Pass with follow-through required. The plan includes unit, integration, API contract, and storage adapter contract tests per constitution.
- **Security and privacy**: Pass. Auth remains gateway-managed, logs avoid raw payload leakage, and only canonical minimum fields plus caller-supplied metadata are persisted.

*Post-design re-check after Phase 1 artifacts: PASS*

- No constitution violations were introduced by the design artifacts.
- The only deferred area is durable retry implementation detail for indexing jobs, which is captured as an explicit open decision rather than hidden complexity.

## Implementation Readiness Assessment

- Artifact-wise, this feature is ready for implementation. No blocking artifact issue or unresolved cross-document ambiguity remains.
- Residual risk is limited to implementation verification. Each remaining risk is tied below to the exact implementation activity and evidence required to close it.

### Remaining Implementation Risks

1. **Standard-payload validation**
  - Spec anchors: FR-001, FR-002, FR-014, AC-F7, AC-V1, NC-012
  - Implementation activity: keep Open Badges and CLR validation at the HTTP boundary, execute pinned-schema checks before canonical mapping, and reject any payload that cannot produce one deterministic non-empty trimmed `id` and `name` pair.
  - Verification evidence: a validation matrix that proves accepted, schema-invalid, and shape-valid-but-unmappable payloads produce the documented allow or reject outcomes and never leave partial authoritative state.
2. **Replay hashing**
  - Spec anchors: FR-002, FR-005, AC-F2, AC-F3, AC-V2
  - Implementation activity: compute a deterministic normalized JSON hash for supported standard payloads independently from the preserved raw-body content, compare that hash on replay, and preserve first-commit retrieval content unchanged.
  - Verification evidence: deterministic hash fixtures plus replay and conflict scenarios showing same-hash idempotency, preserved-content retrieval, and conflict detection for semantic changes.
3. **Outbox mapping**
  - Spec anchors: FR-006, FR-008, FR-009, AC-F1, AC-R2, AC-V3, NC-007
  - Implementation activity: commit authoritative rows and `memory_index_job` in one transaction, rehydrate projection documents from authoritative storage, and map internal outbox states to public `indexing_status` without leaking internal vocabulary.
  - Verification evidence: contract and integration coverage proving that the outbox record contains the authoritative keys needed to reconstruct projection inputs and that external responses summarize indexing state only as `queued`, `indexed`, or `deferred`.
4. **Performance gates**
  - Spec anchors: AC-P1, AC-P2, AC-P3, AC-V4, NC-001, NC-002, NC-003, NC-004, NC-009
  - Implementation activity: instrument endpoint latency and error-rate metrics, execute benchmark and load suites over representative canonical and direct-standard fixtures, and treat threshold assertions as release gates.
  - Verification evidence: reproducible benchmark or load reports showing p95 or p99 latency, throughput, and error-rate measurements against the published thresholds.

## Architecture / Components

### Request Flow

1. Axum handler validates headers, body size, and payload shape.
2. Ingest adapter converts canonical, Open Badges, or CLR input into a protocol-neutral registration command.
3. Application service enforces idempotency, normalizes content into memory items, and commits authoritative records plus indexing work in one SurrealDB transaction.
4. Retrieval endpoints read only from SurrealDB authoritative records.
5. Search endpoint reads only from Meilisearch projections.
6. Background indexing worker consumes durable projection work and updates Meilisearch asynchronously.

### Handler

- Location: `crates/app_server/src/` with route modules such as `handlers/memory_ingest.rs` and `handlers/health.rs`.
- Responsibilities:
  - Apply request body limit and content-type guards.
  - Deserialize request DTOs and run schema validation.
  - Extract or generate request IDs and propagate W3C trace context.
  - Convert service results into HTTP status codes and JSON responses.
- Non-responsibilities:
  - No normalization logic.
  - No direct SurrealDB or Meilisearch calls.
  - No cross-endpoint orchestration beyond request adaptation.

### Application / Service

- Proposed crate: `crates/mod_memory/src/application/`.
- Core services:
  - `RegisterSourceService`: validate canonical command, compute canonical payload hash, enforce idempotency, normalize, persist, enqueue indexing.
  - `GetMemoryItemService`: retrieve a memory item and source summary from authoritative storage.
  - `GetSourceService`: retrieve source metadata plus ordered memory items.
  - `SearchMemoryItemsService`: query Meilisearch projection and map projection hits to API responses without hydrating hidden SurrealDB fallbacks.
- Supporting application ports:
  - `ClockPort`
  - `IdGeneratorPort`
  - `IndexingPort`
  - `GraphProjectionPort` as a no-op boundary implemented in this slice for future FalkorDB expansion.

### Domain Model

- Shared canonical primitives belong in `crates/core_shared/src/`:
  - `SourceId`
  - `MemoryItemUrn`
  - `DocumentType`
  - `MemoryUnitType`
  - shared error types
- Memory-slice aggregate logic belongs in `crates/mod_memory/src/domain/`:
  - `Source`
  - `MemoryItem`
  - `NormalizationPlan`
  - `SourceRegistration`
  - `ProjectionEvent`
- Domain invariants:
  - `external_id` is globally unique.
  - `sequence` is stable and unique within a source.
  - memory-item URNs are deterministic and immutable.
  - indexing projections are derived views and may lag authoritative state.

### Repository

- Proposed repository ports in `crates/mod_memory/src/infra/repo.rs`:
  - `SourceRepository`
  - `MemoryItemRepository`
  - `MemoryQueryRepository`
  - `IndexingOutboxRepository`
- Proposed SurrealDB adapter responsibilities in `crates/core_infra/src/surrealdb.rs`:
  - bootstrap namespaces, tables, and uniqueness constraints
  - execute transactional create-or-return-existing flow
  - query source plus ordered items efficiently
  - persist durable indexing outbox rows in the same transaction as authoritative data

### Indexing Adapter

- Proposed adapter in `crates/mod_memory/src/infra/indexer.rs` backed by `crates/core_infra/src/meilisearch.rs`.
- Responsibilities:
  - translate canonical memory items to Meilisearch projection documents
  - create or update index settings idempotently on startup
  - bulk index all items for a source after persistence commits
  - update outbox job status for success, retry, and dead-letter conditions
- Constraint: indexing never replaces or bypasses authoritative storage reads.

### Future Graph Boundary

- Introduce a `GraphProjectionPort` and canonical projection event names now, but ship only a `NoopGraphProjectionAdapter` in this slice.
- Naming and boundaries use canonical identifiers (`source_id`, `urn`, `sequence`) so a future FalkorDB adapter can project graph nodes and edges without changing the `Source` or `MemoryItem` meaning.

## Data Model Implications

### Source

- Authoritative record stored in SurrealDB table `source`.
- Public fields: `source_id`, `external_id`, `title`, `summary`, `document_type`, `created_at`, `updated_at`, `source_metadata`.
- Derived internal fields:
  - `canonical_payload_hash` stored in reserved system metadata for idempotency comparison.
  - `ingest_kind` to record whether the request originated as canonical, Open Badges, or CLR without polluting canonical APIs.
  - `payload_preservation` metadata describing whether content came from canonical `content` or the preserved raw standard-body string.
- Constraints:
  - unique `external_id`
  - immutable `source_id`
  - `document_type` is `json` for accepted Open Badges and CLR ingest even though the original boundary payload family is tracked only in reserved metadata
  - source exists before at least one memory item is visible

### Memory Item

- Authoritative record stored in SurrealDB table `memory_item`.
- Fields: `urn`, `source_id`, `sequence`, `unit_type`, `start_offset`, `end_offset`, `version`, `content`, `content_hash`, `created_at`, `updated_at`, `item_metadata`.
- Derived metadata:
  - `content_preview` computed for projections only, not needed in authoritative storage.
  - optional operational warnings stored under `item_metadata.system.warnings` without mutating accepted content.
- Constraints:
  - unique `urn`
  - unique (`source_id`, `sequence`)
  - offsets are UTF-8 byte offsets into authoritative canonical content
  - accepted direct-standard ingest produces exactly one `json_document` item with offsets `[0, canonical_content_byte_length)`
  - immutable after creation for this slice

### Retrieval View / Projection

- Retrieval view for `GET /sources/{source-id}` is a joined read model built at query time from SurrealDB authoritative rows:
  - source summary fields
  - ordered memory-item collection
  - required `indexing_status` summary for operational visibility using the public vocabulary `queued`, `indexed`, or `deferred`
- Search projection stored in Meilisearch index `memory_items_v1`:
  - `urn`
  - `source_id`
  - `sequence`
  - `document_type`
  - `content_preview`
  - `content_hash`
  - `created_at`
  - `updated_at`
- Supporting internal entity: `memory_index_job` outbox row containing job id, source id, batch payload reference, status, retry count, and timestamps.

## Interface / Contract Considerations

### REST Endpoints

- `POST /sources/register`
  - accepts one of canonical JSON, Open Badges JSON, or CLR JSON
  - returns `201 Created` for new authoritative ingest
  - returns `200 OK` for idempotent replay of the same canonical payload
  - returns `409 Conflict` for duplicate `external_id` with conflicting canonical payload
- `GET /memory-items/{urn}`
  - authoritative retrieval by immutable URN
- `GET /sources/{source-id}`
  - authoritative source retrieval with associated items ordered by ascending `sequence`
- `GET /search/memory-items`
  - Meilisearch-backed projection query with `q`, `source-id`, `document-type`, `limit`, `offset`
  - returns projection hits only; authoritative content remains available through retrieval endpoints
- `GET /health`
  - local-only liveness probe with service-local status only
- `GET /ready`
  - dependency-aware readiness gated on SurrealDB write-path availability; search can be degraded

### Request / Response Shape

- Canonical register request fields remain `title`, `summary`, `external-id`, `document-type`, `content`, `metadata`.
- Open Badges and CLR inputs are accepted as boundary-specific DTOs and validated against repository-pinned JSON Schema snapshots derived from the supported standard credential envelope profiles for this slice.
- Validation strictness is strict for the supported envelope fields and canonical mapping prerequisites (`id`, `name`, and the required context/type markers), while remaining permissive for extension fields allowed by the pinned schemas.
- A payload that matches a supported standard envelope but cannot be deterministically mapped into canonical `title`, `external_id`, and canonical content is rejected with `400 INVALID_STANDARD_PAYLOAD`. Unmappable means the payload cannot yield one non-empty trimmed `id` and `name` pair or cannot be deterministically classified to one supported family after the pinned-schema pass.
- Register response includes:
  - `source_id`
  - `external_id`
  - `document_type`
  - `memory_items` with `urn`, `sequence`, `unit_type`
  - `indexing_status` with values `indexed`, `queued`, or `deferred`
- Retrieval response includes canonical source and memory metadata only; no standard-specific payload fields leak past the ingest boundary. For direct-standard ingest, the authoritative `json_document` memory item content is the preserved raw UTF-8 request body from the first successful registration.
- Search response includes projection hits only: `urn`, `source_id`, `sequence`, `document_type`, `content_preview`, and optional score.

### Error Contract

- Uniform JSON error object:
  - `error_code`
  - `message`
  - `details` optional structured object or array
  - `timestamp`
  - `request_id`
- Mapping examples:
  - `INVALID_INPUT` -> 400
  - `PAYLOAD_TOO_LARGE` -> 413
  - `EXTERNAL_ID_CONFLICT` -> 409
  - `NOT_FOUND` -> 404
  - `STORAGE_UNAVAILABLE` -> 503
  - `NORMALIZATION_TIMEOUT` -> 408
  - `SEARCH_UNAVAILABLE` -> 503 for search-only requests

## Storage / State / API Decisions

- **Authoritative state**: SurrealDB only. All source and memory-item reads for ingest and retrieval use SurrealDB.
- **Projection state**: Meilisearch only. Search reads never infer authoritative truth and can be rebuilt from SurrealDB plus outbox data.
- **Standard-payload canonicalization**:
  - Open Badges AchievementCredential-style and CLR credential-style requests are validated against pinned boundary schemas before canonicalization.
  - canonical `external_id` is derived from the standard payload `id`.
  - canonical `title` is derived from the standard payload `name`.
  - authoritative `document_type` is set to `json` for accepted standard payloads.
  - canonical content preserves the original validated UTF-8 request body exactly as submitted; in implementation terms this is the accepted request body string before any pretty-printing or reparsing.
  - accepted direct-standard ingest produces exactly one `json_document` memory item whose content equals that preserved raw body string.
  - canonical payload hashing for idempotency uses a deterministic normalized JSON form separate from stored content so whitespace and object-key-order differences replay cleanly.
  - shape-valid but unmappable payloads fail with `400 INVALID_STANDARD_PAYLOAD`.
- **Idempotency**:
  - enforce unique `external_id` in SurrealDB
  - compute canonical payload hash after request normalization
  - for standard payloads, compare a deterministic normalized JSON hash while leaving the preserved raw content untouched
  - on duplicate `external_id`, compare stored canonical hash to decide `200 OK` replay vs `409 Conflict`
- **Transactions**:
  - single SurrealDB transaction writes `source`, `memory_item[]`, and `memory_index_job`
  - any normalization or persistence failure aborts the transaction
- **Normalization rules**:
  - `text` -> blank-line separated paragraphs
  - `markdown` -> heading sections, with fallback to a single item if no headings exist
  - direct-standard `json` -> exactly one `json_document` item spanning the full preserved UTF-8 payload body; no semantic splitting by assertion, achievement, subject, or claim occurs in this slice
  - empty content -> single placeholder memory item derived from metadata
  - accepted content is never sanitized, truncated, or split beyond the declared natural boundaries in this slice
- **Indexing status vocabulary**:
  - external API responses use only `queued`, `indexed`, and `deferred`
  - outbox rows use internal job statuses `pending`, `processing`, `retryable`, `completed`, and `dead_letter`
  - external `queued` means authoritative persistence succeeded and an outbox job is durably pending or processing
  - external `indexed` means the projection is already confirmed in Meilisearch
  - external `deferred` means indexing could not yet be confirmed because search is degraded or retryable backlog exists
- **URN generation**:
  - deterministic UUID v5 with namespace `source_id`
  - name seed combines `sequence`, `start_offset`, `end_offset`, and `content_hash`
- **Outbox payload policy**:
  - `memory_index_job` stores source identifiers and status metadata only; the worker rehydrates projection documents from authoritative SurrealDB records
- **API body limits**:
  - 10 MB hard limit enforced in the HTTP layer before full parsing
- **Timeouts**:
  - service-level 30 second timeout for normalization plus transaction phase
  - external dependency clients use bounded connection and request timeouts
- **Readiness semantics**:
  - `/health` performs only local liveness checks and always returns service-local status with no dependency probe
  - `/ready` performs dependency-aware checks and returns 503 only if SurrealDB cannot complete a lightweight write-path probe
  - `/ready` reports search as `degraded` when Meilisearch is unavailable while the write path remains ready

## Failure Modes / Edge Cases

- Oversized payloads are rejected before normalization with `413`.
- Missing required fields, invalid enum values, malformed UTF-8, or supported-standard payloads that fail pinned-schema validation or canonical mapping return `400` with structured validation details.
- Formatting-only variants of the same validated Open Badges or CLR payload replay successfully because idempotency uses normalized JSON rather than preserved raw-body bytes.
- Duplicate `external_id` races are resolved by database uniqueness, not handler-level locks.
- Conflicting replay for the same `external_id` returns `409` after canonical hash comparison.
- Empty content creates one placeholder memory item with `unit_type = metadata_placeholder` or equivalent canonical unit.
- Long normalization cancels with `408`, rolls back the transaction, and emits a timeout metric.
- SurrealDB outage returns `503` and leaves no partial authoritative state.
- Meilisearch outage marks indexing job as retryable and still returns successful ingest with `indexing_status = deferred`.
- Search requests when Meilisearch is unavailable return `503` with a message indicating authoritative retrieval remains available.
- Partial indexing failure in a source batch updates outbox status and retains authoritative items untouched.

## Security / Privacy Notes

- Authentication and authorization remain upstream concerns at the API gateway, but handlers still fail closed when gateway identity headers are missing or malformed.
- Logs include request IDs, trace IDs, status, duration, and error categories but never raw content or arbitrary metadata payloads.
- Standard-specific request bodies are validated against repository-pinned boundary schemas. Their raw UTF-8 body may be preserved as canonical content in this slice, but protocol-specific fields are not introduced into the authoritative source or memory-item schema.
- `source_metadata` and `item_metadata` reserve a system namespace for internal fields so user-supplied metadata cannot overwrite integrity or operational markers.
- Configuration for SurrealDB and Meilisearch credentials is environment-driven only.

## Performance / Scalability Notes

- Handlers remain stateless, so app instances scale horizontally behind a load balancer.
- Normalization is linear over content length and does not call external AI services.
- SurrealDB writes are batched in one transaction per source registration.
- Source retrieval should use ordered queries on `source_id` and `sequence` with no post-query sorting in handlers.
- Search projections store previews instead of full content to keep Meilisearch documents compact.
- Indexing uses bulk document submission per source to reduce request overhead.
- Durable outbox processing allows backpressure and retry without blocking the write path.

## Testing / Validation Strategy

- **Unit tests**:
  - paragraph and markdown section normalization
  - direct-standard `json_document` normalization and UTF-8 byte-offset calculation
  - deterministic URN generation
  - canonical payload hashing and conflict detection
  - normalized JSON hashing for standard-payload replay despite raw-body formatting differences
  - standard-payload validation allow or reject matrix coverage for accepted, schema-invalid, and shape-valid-but-unmappable inputs
  - error mapping from domain/application errors to HTTP contract
- **Integration tests**:
  - full ingest -> persist -> retrieve flow against disposable SurrealDB and Meilisearch instances
  - Open Badges happy path, CLR happy path, standard replay, standard conflict, invalid-standard-schema, and shape-valid-but-unmappable flows
  - preserved-content retrieval after formatting-only replay to prove raw-body preservation and replay-hash determinism remain aligned
  - search degraded mode where Meilisearch is unavailable but registration succeeds
  - readiness behavior with database down vs search down
- **API contract tests**:
  - validate request/response examples against the published OpenAPI contract
  - assert every published status code explicitly, including `200/201/400/408/409/413/503` for `POST /sources/register`, `200/404/503` for retrieval routes, `200/503` for `/ready`, and `200` for `/health`
  - assert the distinct liveness versus readiness schemas
- **Storage adapter contract tests**:
  - SurrealDB uniqueness, transaction rollback behavior, no-TTL retention baseline, and write-path readiness probe behavior
  - outbox durability, authoritative-to-projection mapping correctness, and retry status transitions
  - Meilisearch projection schema, filter settings, and sort behavior
- **Concurrency tests**:
  - duplicate registration races for identical and conflicting payloads
  - multiple concurrent source ingests across stateless app instances
  - multi-instance consistency under a shared SurrealDB backend
- **Performance validation**:
  - benchmark registration latency with representative canonical markdown payloads under 100 KB and direct-standard JSON payloads under 100 KB
  - benchmark retrieval latency for single-item and 10k-item source scenarios
  - benchmark search latency against a representative 1M-document projection corpus or replayable synthetic fixture
  - capture p95/p99 latency, throughput, and error rate from Prometheus-compatible metrics and fail the gate when AC-P1, AC-P2, AC-P3, AC-V4, NC-001, NC-002, NC-003, or NC-004 are exceeded

## Rollout / Migration Notes

- This is the first vertical slice, so there is no existing production data to migrate.
- Startup should bootstrap SurrealDB schema, uniqueness constraints, and Meilisearch index settings idempotently.
- Route registration can be gated behind a dedicated `memory_ingest` config flag until the slice is validated in staging.
- Because search is non-authoritative, a simple re-index worker can rebuild Meilisearch from SurrealDB plus outbox state if search data is lost.
- Release readiness for this slice requires explicit verification evidence for standard-payload validation, replay hashing determinism, authoritative outbox mapping, and published performance gates.
- Future FalkorDB rollout should consume the same canonical projection events instead of changing the current write or retrieval contracts.

## Residual Implementation Risk Statement

- No blocking artifact issue remains.
- Remaining risk is implementation-only and is explicitly limited to standard-payload validation, replay hashing, outbox mapping, and performance-gate verification.
- Implementation should not be considered complete until the verification evidence listed in this plan exists for each risk area.

## Project Structure

### Documentation (this feature)

```text
specs/001-memory-ingest/
├── adr/
│   └── 0001-direct-standard-ingest.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── README.md
│   └── memory-ingest.openapi.yaml
└── tasks.md
```

### Source Code (repository root)

```text
crates/
├── app_server/
│   └── src/
│       ├── config.rs
│       ├── main.rs
│       ├── middleware.rs
│       ├── router.rs
│       ├── state.rs
│       └── handlers/
│           ├── health.rs
│           └── memory_ingest.rs
├── core_infra/
│   └── src/
│       ├── lib.rs
│       ├── setup.rs
│       ├── surrealdb.rs
│       ├── meilisearch.rs
│       └── falkordb.rs
├── core_shared/
│   └── src/
│       ├── error.rs
│       ├── id_gen.rs
│       ├── lib.rs
│       ├── memory.rs
│       └── urn.rs
└── mod_memory/
    └── src/
        ├── lib.rs
        ├── application/
        │   ├── mod.rs
        │   ├── command.rs
        │   ├── query.rs
        │   └── service.rs
        ├── domain/
        │   ├── mod.rs
        │   ├── normalizer.rs
        │   ├── source.rs
        │   ├── memory_item.rs
        │   └── event.rs
        └── infra/
            ├── mod.rs
            ├── repo.rs
            ├── indexer.rs
            └── outbox.rs

tests/
├── contract/
│   └── memory_ingest_api.rs
├── integration/
│   ├── memory_ingest_flow.rs
│   ├── memory_search_projection.rs
│   └── readiness.rs
└── unit/
    └── memory_normalizer.rs
```

**Structure Decision**: Use the existing multi-crate workspace and add a dedicated `mod_memory` crate for memory-specific application/domain/infra code. Keep HTTP adapter code in `app_server`, shared primitives in `core_shared`, and concrete SurrealDB/Meilisearch connectivity in `core_infra`.

## Complexity Tracking

No constitution violations require justification in this plan.
