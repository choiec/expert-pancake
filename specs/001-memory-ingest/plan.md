# Implementation Plan: Schema-Native Standard Credential Registry

**Branch**: `001-memory-ingest` | **Date**: 2026-03-21 | **Spec**: `/workspaces/debian/expert-pancake-ai/specs/001-memory-ingest/spec.md`
**Status**: IMPLEMENT-READY
**Input**: Feature specification from `/workspaces/debian/expert-pancake-ai/specs/001-memory-ingest/spec.md`
**Related ADRs**:
- `/workspaces/debian/expert-pancake-ai/specs/001-memory-ingest/adr/0001-direct-standard-ingest.md`
- `/workspaces/debian/expert-pancake-ai/specs/001-memory-ingest/adr/0002-schema-native-api.md`

## Summary

This redesign removes the public canonical `Source` / `MemoryItem` model and replaces it with a schema-native credential API. Axum handlers accept supported Open Badges 3.0 and CLR 2.0 credential payloads at the HTTP boundary, validate them against pinned schemas, classify the family, derive an authoritative semantic hash, persist an authoritative credential record keyed by the official standard `id`, and return the stored schema-exact credential document directly. Retrieval uses the official `id`, search remains a rebuildable projection, and operational probes continue to distinguish local liveness from dependency-aware readiness.

## Technical Context

**Language/Version**: Rust stable, edition 2024  
**Primary Dependencies**: `axum`, `tokio`, `tower`, `tower-http`, `serde`, `serde_json`, `validator`, `uuid`, `sha2`, `tracing`, `tracing-subscriber`, `thiserror`, `surrealdb`, `meilisearch-sdk`  
**Storage**: SurrealDB for authoritative credential persistence plus durable indexing outbox; Meilisearch for search projection only  
**Testing**: `cargo nextest`, contract tests, integration tests, adapter contract tests, `cargo-llvm-cov`, `cargo-mutants`  
**Target Platform**: Linux containerized web service behind an API gateway  
**Project Type**: Multi-crate Rust web service  
**Performance Goals**: registration under 5 seconds p95, retrieval under 200 ms p95, search under 500 ms p95 for representative fixtures  
**Constraints**: thin handlers only, canonical domain model first, no service-owned public wrapper fields, 10 MB max request body, 30 second normalization or validation timeout, transactional authoritative writes, search failure must not block writes  
**Scale/Scope**: single-tenant first slice, horizontally scalable stateless app instances, up to 1M indexed credentials, Open Badges 3.0 and CLR 2.0 only

## Constitution Check

*Gate status before Phase 0 research: PASS*

- **Layered handler/service/repository separation**: Pass. Handlers stay thin and delegate credential registration, retrieval, and projection work into application services and repository ports.
- **Canonical domain model first**: Pass. The new canonical model is now a schema-native standard credential aggregate rather than a protocol-neutral `Source`.
- **Storage responsibility separation**: Pass. SurrealDB remains authoritative and Meilisearch remains projection-only.
- **Explicit errors and observability**: Pass. The redesign retains structured error mapping, request correlation, traces, and readiness semantics.
- **Testing discipline**: Pass with follow-through required. The task plan keeps RED -> GREEN -> REFACTOR -> VERIFY for contract, integration, and adapter coverage.
- **Security and privacy**: Pass. The public redesign removes wrapper metadata leakage and continues to forbid logging whole credential bodies.

*Post-design re-check after Phase 1 artifacts: PASS*

- No constitution violations were introduced by the schema-native redesign.
- The primary scope change is public contract replacement, not architectural shortcutting.

## Implementation Readiness Assessment

- Artifact-wise, this feature is ready for implementation.
- The previous canonical identity artifacts are intentionally superseded and must not be treated as compatibility requirements.
- Residual risk is limited to implementation verification and is captured below as explicit gates.

### Remaining Implementation Risks

1. **Strict schema-native boundary validation**
  - Spec anchors: FR-001, FR-002, FR-004, AC-001, AC-002, NC-007
  - Implementation activity: reject unsupported families and unsupported top-level fields before persistence, while preserving nested official content under supported top-level keys.
  - Verification evidence: contract and integration coverage for supported payloads, unsupported top-level keys, unsupported families, and schema-invalid requests.
2. **Replay hashing**
  - Spec anchors: FR-006, FR-007, FR-008, AC-003, AC-004
  - Implementation activity: compute a deterministic normalized JSON hash from the authoritative schema-exact credential value and use it for replay and conflict comparison.
  - Verification evidence: deterministic hash fixtures plus replay and conflict scenarios across formatting-only and semantic differences.
3. **Authoritative retrieval shape**
  - Spec anchors: FR-004, FR-005, FR-009, FR-010, AC-002, AC-005
  - Implementation activity: ensure authoritative GET and POST responses return only schema-native credential documents and never leak wrapper-era fields.
  - Verification evidence: OpenAPI-backed contract tests and end-to-end retrieval checks.
4. **Projection decoupling**
  - Spec anchors: FR-011, AC-006, NC-006
  - Implementation activity: persist durable search work transactionally, rehydrate projection documents from authoritative credential rows, and keep search failure isolated from authoritative writes and reads.
  - Verification evidence: outbox contract tests plus degraded-search integration coverage.

## Project Structure

### Documentation

```text
specs/001-memory-ingest/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── README.md
│   ├── memory-ingest.openapi.yaml
│   └── 1edtech/
└── tasks.md
```

### Source Code

```text
crates/
├── app_server/
│   ├── src/
│   │   ├── handlers/
│   │   ├── middleware.rs
│   │   ├── router.rs
│   │   └── state.rs
│   └── tests/
├── core_infra/
│   └── src/
├── core_shared/
│   └── src/
└── mod_memory/
    ├── src/
    │   ├── application/
    │   ├── domain/
    │   ├── infra/
    │   └── bootstrap.rs
    └── tests/
```

**Structure Decision**: Keep the existing multi-crate service layout. Replace the `Source` / `MemoryItem` application flow inside `mod_memory` and `app_server` with a schema-native credential flow instead of creating a parallel module.

## Architecture / Components

### Request Flow

1. Axum handler validates headers, body size, and request JSON against the supported credential contract.
2. A boundary classifier determines whether the request is Open Badges 3.0 or CLR 2.0.
3. The application service filters the payload to the authoritative schema-exact top-level credential value, computes the semantic payload hash, and checks authoritative replay or conflict rules keyed by credential `id`.
4. The authoritative repository writes the credential record and an indexing outbox row in one SurrealDB transaction.
5. Retrieval endpoints read the authoritative credential record from SurrealDB by official credential `id`.
6. Search reads only from Meilisearch projection documents derived from authoritative credential data.

### Handler Layer

- Location: `crates/app_server/src/handlers/`
- Responsibilities:
  - enforce request limits and content-type guards
  - deserialize credential requests
  - map application outcomes to HTTP status codes and JSON bodies
  - keep `/health` local-only and `/ready` dependency-aware
- Non-responsibilities:
  - no SurrealDB or Meilisearch calls
  - no replay logic
  - no schema filtering or persistence logic

### Application Layer

- Location: `crates/mod_memory/src/application/` and `crates/mod_memory/src/bootstrap.rs`
- Core services:
  - `RegisterCredentialService`
  - `GetCredentialService`
  - `SearchCredentialsService`
- Responsibilities:
  - classify supported families
  - build authoritative credential aggregates
  - compute semantic hashes
  - enforce replay and conflict rules
  - orchestrate persistence and outbox writes

### Domain Model

- Shared primitives in `crates/core_shared/src/`:
  - structured error types
  - identifier encoding helpers for credential `id` path handling
- Memory-slice domain types in `crates/mod_memory/src/domain/`:
  - `StandardCredential`
  - `CredentialFamily`
  - `CredentialRegistration`
  - `CredentialSearchProjection`
- Domain invariants:
  - authoritative public identity is the official credential `id`
  - authoritative credential document contains only official top-level keys for the detected family
  - semantic replay compares normalized authoritative credential content, not raw formatting
  - search documents are rebuildable from authoritative state

### Repository Layer

- Repository ports in `crates/mod_memory/src/infra/`
  - `CredentialRepository`
  - `CredentialQueryRepository`
  - `IndexingOutboxRepository`
- SurrealDB adapter responsibilities in `crates/core_infra/src/surrealdb.rs`
  - bootstrap authoritative credential and outbox tables
  - enforce uniqueness on credential `id`
  - implement transactional create-or-replay-or-conflict flow
  - query credentials by official `id`

### Search Adapter

- `crates/core_infra/src/meilisearch.rs` remains a projection adapter.
- Projection documents derive from authoritative credential rows and may expose search-only summaries such as score or preview context.
- Search failure must not change authoritative API results.

## Data Model Implications

### Standard Credential Record

- Authoritative SurrealDB table: `standard_credential`
- Fields:
  - `credential_id`
  - `family`
  - `credential`
  - `semantic_payload_hash`
  - `created_at`
  - `updated_at`
- Constraints:
  - unique `credential_id`
  - `credential` stores only official top-level keys for the detected supported family
  - no wrapper-era authoritative fields such as `source_id`, `external_id`, or `source_metadata`

### Search Projection

- Meilisearch index: `credentials_v1`
- Projection fields:
  - `credential_id`
  - `family`
  - `name`
  - `issuer`
  - `type`
  - `valid_from`
  - optional search-only preview text
- Constraint:
  - projection is non-authoritative and fully rebuildable

### Credential Index Job

- Supporting internal table: `credential_index_job`
- Fields:
  - `job_id`
  - `credential_id`
  - `status`
  - `retry_count`
  - `created_at`
  - `updated_at`
- Constraint:
  - committed in the same transaction as the authoritative credential record

## Interface / Contract Considerations

### REST Endpoints

- `POST /credentials/register`
  - accepts Open Badges 3.0 or CLR 2.0 credential JSON
  - returns `201 Created` for a new authoritative record
  - returns `200 OK` for an idempotent replay
  - returns `409 Conflict` for a semantic mismatch on the same credential `id`
- `GET /credentials/{credential-id}`
  - authoritative retrieval by official credential `id`
  - returns the stored schema-exact credential document directly
- `GET /credentials/search`
  - Meilisearch-backed projection query
  - returns projection hits only
- `GET /health`
  - local-only liveness
- `GET /ready`
  - dependency-aware readiness with separate authoritative and search status

### Request / Response Shape

- Registration request is one of the supported standard credential schemas only.
- Successful authoritative registration and retrieval responses are schema-native credential documents, not wrapper envelopes.
- Error responses remain uniform JSON objects containing:
  - `error_code`
  - `message`
  - `details` optional
  - `timestamp`
  - `request_id`
- Search response is a projection envelope with pagination metadata and search hits derived from credential data.

### Error Contract

- `INVALID_INPUT` -> 400
- `PAYLOAD_TOO_LARGE` -> 413
- `CREDENTIAL_CONFLICT` -> 409
- `NOT_FOUND` -> 404
- `STORAGE_UNAVAILABLE` -> 503
- `VALIDATION_TIMEOUT` -> 408
- `SEARCH_UNAVAILABLE` -> 503 for search-only requests

## Storage / State / API Decisions

- **Authoritative state**: SurrealDB only.
- **Authoritative document shape**: schema-native credential JSON with official top-level keys only.
- **Authoritative identity**: official credential `id`.
- **Replay comparator**: normalized authoritative credential JSON hash.
- **Removed public concepts**: `Source`, `MemoryItem`, canonical `external_id`, internal `source_id`, provenance wrappers, raw memory-item retrieval.
- **Projection state**: Meilisearch only, rebuildable from authoritative credential rows plus outbox state.

## Validation Strategy

- Contract tests pin all published status codes and authoritative response bodies for `POST /credentials/register`, `GET /credentials/{credential-id}`, `GET /credentials/search`, `/health`, and `/ready`.
- Integration tests cover successful Open Badges and CLR registration, replay, conflict, retrieval, and degraded-search scenarios.
- Storage adapter contract tests prove uniqueness, rollback, schema-exact persistence, and no-TTL retention behavior.
- Story-level verification continues to use `cargo nextest`, `cargo-mutants`, and `cargo-llvm-cov`.
