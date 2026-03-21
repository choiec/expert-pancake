# Feature Specification: Memory Ingest and Canonical Source Identity

**Feature Branch**: `001-memory-ingest`  
**Created**: 2026-03-17  
**Updated**: 2026-03-21  
**Status**: IMPLEMENT-READY

## Context

`001-memory-ingest` established the first authoritative ingest path for canonical/manual documents and direct-standard JSON payloads. The follow-up identity work that lived in `002-canonical-source-external-id` on `main` is now folded back into this feature branch and specification set instead of being carried as a separate spec folder.

This merged slice defines one authoritative vertical slice:

- canonical/manual ingest plus direct Open Badges and CLR ingest
- one canonical public `external_id` model
- one deterministic internal `source_id`
- one replay/conflict rule based on semantic payload equality
- one protocol-neutral `Source` plus `MemoryItem` storage model
- one consistent provenance envelope returned from registration and retrieval

The expected outcome is that equivalent requests converge on the same logical source, retrieval stays byte-accurate for authoritative memory-item content, and public contracts expose canonical identity plus provenance without leaking migration-only or adapter-only state.

## Goals

- **G1**: Accept canonical/manual and direct-standard ingest through `POST /sources/register`.
- **G2**: Normalize accepted content into deterministic `MemoryItem` records.
- **G3**: Persist authoritative `Source` and `MemoryItem` state before returning success.
- **G4**: Resolve replay, conflict, and authoritative retrieval decisions from canonical identity plus semantic payload hash.
- **G5**: Keep search explicitly non-authoritative while still exposing a usable projection API.
- **G6**: Expose consistent retrieval, search, health, readiness, and operator-facing documentation contracts.
- **G7**: Keep the vertical slice runnable and verifiable end-to-end through documented quickstart and smoke flows.

## Non-goals

- Legacy identifier migration, alias lookup, or mixed-population compatibility.
- Additional direct-standard families beyond Open Badges and CLR.
- Binary/media ingest, UI work, LLM enrichment, or batch ingest.
- Making search authoritative for replay, conflict, or retrieval decisions.

## Clarifications

### Session 2026-03-17

- Direct-standard ingest accepts Open Badges and CLR payloads as first-class request bodies.
- Accepted direct-standard payloads persist as `document_type = json` and derive one `json_document` memory item.
- Retrieval of a direct-standard memory item returns the preserved first-commit UTF-8 request body exactly as stored.
- Formatting-only JSON changes do not create duplicates because replay uses deterministic normalized JSON hashing.

### Session 2026-03-19

- The canonical public source identity model from `002-canonical-source-external-id` is merged into this slice rather than tracked in a separate spec folder.
- Public `external_id` values are canonical project-owned URIs under `https://api.cherry-pick.net/...`.
- Internal `source_id` remains distinct from `external_id` and is deterministically derived from canonical identity.
- Original direct-standard payload identifiers are preserved only as provenance metadata.

## User Stories

### User Story 1 - Register a source with canonical identity (P1)

A source producer submits a canonical/manual or direct-standard request and receives one authoritative source identity plus derived memory items.

**Acceptance scenarios**

1. Given a canonical/manual request with an already-canonical `external_id`, when registration succeeds, then the response returns the canonical `external_id`, deterministic `source_id`, `indexing_status`, and derived memory-item summaries.
2. Given a valid Open Badges or CLR payload, when registration succeeds, then the response returns `document_type = json`, one `json_document` memory item, and provenance metadata describing canonical identity version, ingest kind, semantic payload hash, and original standard identifier when present.

### User Story 2 - Replay equivalent submissions without duplicates (P1)

A source producer resubmits the same logical source and receives the existing authoritative source rather than creating duplicates.

**Acceptance scenarios**

1. Given an existing canonical identity and semantically equivalent payload, when the request is replayed, then the system returns the existing authoritative identifiers and does not create duplicate source or memory-item rows.
2. Given an existing canonical identity and semantically different payload, when the request is submitted, then the system returns `409 Conflict` and leaves authoritative state unchanged.

### User Story 3 - Retrieve canonical identity with provenance (P1)

A consumer retrieves a source or memory item and can distinguish canonical source identity from original upstream provenance.

**Acceptance scenarios**

1. Given a direct-standard source, when `GET /sources/{source-id}` succeeds, then the response includes canonical `external_id` plus provenance under `source_metadata.system`.
2. Given a stored memory item, when `GET /memory-items/{urn}` succeeds, then the response returns authoritative content and source metadata consistent with the registration result.

### User Story 4 - Search projection remains non-authoritative (P2)

A consumer searches memory-item projections while authoritative replay and retrieval continue to be governed by the authoritative store.

**Acceptance scenarios**

1. Given indexed content, when `GET /search/memory-items` is queried, then the response returns projection hits with preview-only fields.
2. Given search degradation, when registration and authoritative retrieval continue to work, then search may return `503` without changing authoritative replay or conflict behavior.

## Edge Cases

- Canonical/manual ingest provides a non-canonical `external_id`.
- Direct-standard ingest is shape-valid but cannot be classified or mapped into canonical identity.
- Formatting-only JSON replay keeps semantic equality and must replay successfully.
- Semantic payload changes for the same canonical identity must conflict.
- Concurrent equivalent requests must converge on one authoritative source.
- Search projection lag must not change authoritative replay or retrieval decisions.

## Functional Requirements

- **FR-001**: `POST /sources/register` MUST accept either canonical/manual JSON or supported direct-standard JSON (Open Badges or CLR).
- **FR-002**: Canonical/manual ingest MUST accept only already-canonical `external_id` values under the project-owned URI namespace.
- **FR-003**: Direct-standard ingest MUST derive canonical `external_id` from trusted source-domain context plus the original upstream identifier and MUST preserve the original upstream identifier as provenance.
- **FR-004**: The system MUST derive one deterministic internal `source_id` from canonical `external_id` and MUST keep `source_id` distinct from `external_id`.
- **FR-005**: Replay and conflict decisions MUST use canonical `external_id` plus semantic payload equivalence; raw formatting differences alone MUST NOT create conflicts.
- **FR-006**: A replay with the same canonical identity and same semantic payload MUST return the existing authoritative source and MUST NOT create duplicate rows.
- **FR-007**: A submission with the same canonical identity and different semantic payload MUST return `409 Conflict` and MUST NOT overwrite authoritative state.
- **FR-008**: Accepted direct-standard payload bodies MUST be preserved exactly as the first successful authoritative body and exposed through one derived `json_document` memory item.
- **FR-009**: Registration and retrieval responses MUST expose one public provenance shape under `source_metadata.system` containing `canonical_id_version`, `ingest_kind`, `semantic_payload_hash`, and `original_standard_id` when present.
- **FR-010**: Authoritative storage and retrieval contracts MUST remain protocol-neutral after ingest normalization.
- **FR-011**: `GET /sources/{source-id}` MUST return source metadata and all associated memory items ordered by ascending `sequence`.
- **FR-012**: `GET /memory-items/{urn}` MUST return authoritative content and item metadata exactly as committed.
- **FR-013**: `GET /search/memory-items` MUST return projection hits only and MUST remain non-authoritative.
- **FR-014**: `/health` MUST be local-only liveness and `/ready` MUST reflect authoritative write-path readiness plus search degradation.
- **FR-015**: Public contracts MUST NOT expose migration-only aliases, alternate identity paths, or compatibility-only fields.

## Non-Functional Requirements

- **NC-001**: Registration requests for representative canonical/manual and direct-standard payloads under 100 KB MUST meet a p95 latency target of 5 seconds or less in release validation.
- **NC-002**: Authoritative retrieval requests for representative stored sources and memory items MUST meet a p95 latency target of 200 ms or less in release validation.
- **NC-003**: Search projection queries against the representative benchmark corpus MUST meet a p95 latency target of 500 ms or less in release validation.
- **NC-004**: Performance-sensitive changes in this slice MUST produce reproducible benchmark or load evidence for latency, throughput, and error rate before release.
- **NC-005**: Concurrent duplicate registrations across stateless app instances MUST converge on one authoritative source outcome without data divergence.
- **NC-006**: Authoritative writes MUST be transactional and leave no partial authoritative state behind after timeout, conflict, or storage failure.
- **NC-007**: Search indexing and projection behavior MUST remain non-authoritative; search degradation or backlog MUST NOT block authoritative registration or retrieval.
- **NC-009**: All public endpoints MUST emit structured logs, request correlation, traces, and latency metrics sufficient for operational diagnosis.
- **NC-010**: `/health` and `/ready` MUST expose distinct liveness versus readiness semantics and response shapes.
- **NC-011**: Logs, metrics, and error payloads MUST avoid raw authoritative content and arbitrary caller metadata that could leak sensitive information.
- **NC-012**: Supported standard payloads MUST be validated at the HTTP boundary against pinned allow or reject rules before any authoritative persistence occurs.
- **NC-016**: Authoritative source, memory-item, and indexing records in this slice MUST default to a no-TTL, no-automatic-purge retention baseline unless a later spec changes that policy.

## Data Model Summary

### Source

- `source_id`: deterministic internal identifier derived from canonical identity
- `external_id`: canonical project-owned URI
- `title`, `summary`, `document_type`, `created_at`, `updated_at`
- `source_metadata.system`: canonical provenance metadata

### Memory Item

- `urn`, `source_id`, `sequence`, `unit_type`, `start_offset`, `end_offset`, `version`
- `content`, `content_hash`, `created_at`, `updated_at`, `item_metadata`
- direct-standard ingest always yields exactly one `json_document`

### Search Projection

- `urn`, `source_id`, `sequence`, `document_type`, `content_preview`, `content_hash`, timestamps
- rebuildable from authoritative source state

## Acceptance Criteria

- **AC-001**: Canonical/manual registration succeeds only for canonical project-owned `external_id` values.
- **AC-002**: Direct-standard registration returns canonical `external_id` and preserves original upstream identifier separately as provenance.
- **AC-003**: Equivalent replays return the same authoritative `source_id` and canonical `external_id` without duplicates.
- **AC-004**: Semantic conflicts for the same canonical identity return `409` without mutating authoritative state.
- **AC-005**: Retrieval returns the same canonical identity and provenance shape that registration established.
- **AC-006**: Direct-standard retrieval returns the preserved first-commit raw body through the single `json_document` item.
- **AC-007**: Search degradation does not change authoritative write, replay, or retrieval outcomes.

## Test Strategy

- Crate-local tests cover canonicalization, normalization, replay hashing, and response mapping close to the owning crates.
- Contract tests pin request and response schemas for registration, retrieval, search, health, and readiness.
- Integration tests cover canonical/manual ingest, direct-standard ingest, replay, conflict, retrieval consistency, and projection behavior.
- Performance verification remains a release gate for registration, retrieval, and search latency targets.
