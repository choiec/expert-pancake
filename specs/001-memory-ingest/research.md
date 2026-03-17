# Research: Memory Ingest Vertical Slice

**Status**: IMPLEMENT-READY

These research decisions are ratified for implementation by `/workspaces/rust/specs/001-memory-ingest/adr/0001-direct-standard-ingest.md`.

## Decision 1: Layer the slice as handler -> application/service -> repository/indexing ports

- **Decision**: Keep Axum handlers in `app_server` as thin HTTP adapters and move all ingest, retrieval, and search orchestration into a dedicated `mod_memory` application layer backed by repository and indexing traits.
- **Rationale**: This directly satisfies the constitution's non-negotiable handler/service/repository boundary, keeps business rules testable without HTTP context, and fits the current workspace's intended crate layout.
- **Alternatives considered**: Putting business logic in handlers was rejected because it violates the constitution and would couple validation, normalization, and persistence too tightly.

## Decision 2: Validate canonical, Open Badges, and CLR payloads at the HTTP boundary

- **Decision**: Accept `application/json` bodies as one of three boundary DTO families: canonical request, Open Badges request, or CLR request. Open Badges and CLR requests are validated against repository-pinned JSON Schema snapshots for the supported credential envelope profiles in this slice, with strict validation of the required envelope and canonical mapping fields (`@context`, `type`, `id`, `name`) and permissive handling of standard extension fields allowed by those pinned schemas. Valid inputs are then converted into the same canonical registration command, stored with `document_type = json`, and normalized into exactly one `json_document` memory item whose content is the accepted UTF-8 request body.
- **Rationale**: The spec requires first-class ingest support for Open Badges and CLR while keeping authoritative storage protocol-neutral. Adapter validation at the boundary prevents 1EdTech concepts from leaking into the canonical domain model and removes ambiguity about what constitutes a supported standard payload.
- **Canonical mapping rule**: canonical `external_id` derives from the trimmed standard payload `id`, canonical `title` derives from trimmed `name`, authoritative `document_type` is `json`, accepted canonical content preserves the original validated UTF-8 request body exactly as submitted, and idempotency hashing uses a deterministic normalized JSON form separate from stored content.
- **Failure policy**: payloads that pass the supported standard envelope schema but cannot satisfy canonical mapping are rejected with `400 INVALID_STANDARD_PAYLOAD` and leave no partial authoritative state. Unmappable means they fail to produce one deterministic non-empty `id`/`name` pair or cannot be classified to exactly one supported profile.
- **Alternatives considered**: Persisting standard-specific shapes directly was rejected because it would break protocol neutrality. A single untyped JSON endpoint was rejected because it weakens validation and obscures error semantics.

## Decision 3: Use SurrealDB as the sole authoritative write and retrieval store

- **Decision**: Persist `source`, `memory_item`, and durable indexing work in SurrealDB transactions and serve authoritative retrieval endpoints only from SurrealDB.
- **Rationale**: The constitution explicitly assigns authoritative storage to SurrealDB and separates correctness from read-path optimization. This also satisfies the spec's rollback and consistency requirements.
- **Alternatives considered**: Reading retrieval data from Meilisearch was rejected because indexing is eventual and non-authoritative. Adding FalkorDB in this slice was rejected because graph work is out of scope.

## Decision 4: Enforce idempotency with unique `external_id` plus canonical payload hash

- **Decision**: Add a unique SurrealDB constraint on `external_id` and store a canonical payload hash in reserved system metadata. On duplicate `external_id`, compare hashes to return existing identifiers for a true replay or `409 Conflict` for conflicting payloads. Standard payload replay hashes are computed from deterministic normalized JSON so raw-body formatting differences do not create conflicts.
- **Rationale**: This resolves concurrent duplicate registration safely at the database layer while honoring the product requirement that identical replays reuse authoritative identifiers.
- **Alternatives considered**: Handler-level check-then-create logic was rejected because it is race-prone. Requiring an extra idempotency header was rejected because `external_id` already serves as the client identity key.

## Decision 5: Generate deterministic immutable URNs with UUID v5 and content bounds

- **Decision**: Build each memory-item URN from UUID v5 using `source_id` as namespace and `sequence`, offsets, and `content_hash` as the stable name seed.
- **Rationale**: The URN stays deterministic across idempotent replay, remains globally unique, and preserves immutability without needing a separate central ID allocator.
- **Alternatives considered**: Random UUID v4 and sortable UUID v7 were rejected because they break deterministic replay semantics. Content-only hashes were rejected because repeated content fragments could collide inside a source.

## Decision 6: Normalize using canonical document-type rules only

- **Decision**: Normalize canonical `text` by blank-line paragraphs and canonical `markdown` by heading section boundaries, with explicit placeholder handling for empty content. Normalize accepted Open Badges and CLR payloads as a single `json_document` item that spans the full preserved UTF-8 request body.
- **Rationale**: These rules come directly from the spec and are simple, deterministic, and testable without external services.
- **Alternatives considered**: Token, sentence, or embedding-based chunking was rejected because it changes product intent and adds unnecessary complexity to the first vertical slice.

## Decision 7: Persist durable indexing work and process Meilisearch asynchronously

- **Decision**: Commit an indexing outbox row in the same SurrealDB transaction as authoritative data, then let a background worker translate that durable work into Meilisearch bulk indexing operations.
- **Rationale**: This keeps writes correct even when search is down, avoids lost indexing work across process restarts, and preserves the constitution's write-path versus read-path separation.
- **Public status rule**: API responses expose only `queued`, `indexed`, and `deferred`. Internal outbox states (`pending`, `processing`, `retryable`, `completed`, `dead_letter`) remain implementation-only and are summarized into the public vocabulary.
- **Alternatives considered**: Synchronous indexing inside the request path was rejected because it would make write availability depend on Meilisearch. An in-memory retry queue was rejected because it can lose work on process crash.

## Decision 8: Search remains a projection with explicit degraded behavior

- **Decision**: `GET /search/memory-items` reads only from Meilisearch, returns projection hits rather than authoritative memory-item records, and returns `503` when search is unavailable, while registration and authoritative retrieval continue working.
- **Rationale**: The spec explicitly allows search degradation without blocking ingest or retrieval. Returning projection hits keeps SurrealDB and Meilisearch responsibilities separate and avoids pretending that search results are authoritative reads.
- **Alternatives considered**: Falling back to SurrealDB full scans for search was rejected because it obscures operational state and scales poorly.

## Decision 9: Reserve a graph projection boundary without adding FalkorDB runtime coupling

- **Decision**: Define `GraphProjectionPort` and canonical projection event shapes now, and include a `NoopGraphProjectionAdapter` task in this slice so the future FalkorDB boundary is executable rather than aspirational.
- **Rationale**: This keeps model naming and service boundaries ready for future graph expansion while respecting the current slice's scope, and it avoids leaving unimplemented abstraction claims in the plan.
- **Alternatives considered**: Adding FalkorDB immediately was rejected as scope creep. Omitting the boundary entirely was rejected because it would make later graph introduction more invasive.

## Decision 10: Treat observability as part of the core contract

- **Decision**: Require request IDs, W3C trace context propagation, structured JSON errors, local-only `/health`, dependency-aware `/ready`, and endpoint latency metrics from the first slice.
- **Rationale**: The constitution makes observability and auditability first-class non-functional requirements, and the feature spec includes them as explicit functional requirements. Separating local-only liveness from dependency-aware readiness removes probe ambiguity and keeps `/health` fast.
- **Alternatives considered**: Adding tracing and metrics later was rejected because it would weaken validation of the first vertical slice and make failure analysis harder.

## Decision 11: Use machine-readable API contracts plus adapter contract tests

- **Decision**: Capture the initial API in an OpenAPI contract under `contracts/`, verify implementation against it with contract tests for every published endpoint and status code, and pair that suite with explicit SurrealDB and Meilisearch adapter contract tests.
- **Rationale**: The constitution requires machine-readable public contracts and storage-adapter verification, and the repository is still skeletal enough that the contracts should lead implementation rather than trail it.
- **Alternatives considered**: Markdown-only endpoint documentation was rejected because it is less precise and less testable.