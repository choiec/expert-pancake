# Feature Specification: Canonical Source External ID

**Feature Branch**: `002-canonical-source-external-id`  
**Created**: 2026-03-18  
**Updated**: 2026-03-19  
**Status**: Draft  
**Input**: Introduce canonical external source identifiers so duplicate sources are normalized and resolved consistently across ingest and retrieval paths. Equivalent external source IDs must map to the same logical source, avoid duplicate registration, and remain deterministic and testable across repeated inputs.

## Context

`001-memory-ingest` established the first authoritative ingest path, but this repository still needs one repository-wide rule for how external source identity is represented, replayed, and exposed. Without that rule, the same logical source can be expressed differently across manual ingest, direct-standard ingest, persistence, and retrieval, which creates duplicate registrations, ambiguous provenance, and inconsistent conflict behavior.

This feature defines the canonical external identity model for sources in the modular monolith:

- one project-owned canonical URI grammar for public source identity
- one deterministic internal `source_id` derived from canonical identity
- one replay/conflict rule based on semantic equality rather than raw formatting
- one provenance shape returned consistently across registration and retrieval
- no transition-only behavior because the repository is still operating under a pre-production assumption

The expected outcome is that clients can submit equivalent sources repeatedly and always get one authoritative identity, while operators can still audit original upstream identifiers and decision reasons.

## Goals

- Define one canonical `external_id` format under the project-owned namespace for all governed source registrations.
- Ensure equivalent source registrations resolve to the same authoritative source across ingest and retrieval flows.
- Preserve original direct-standard identifiers as provenance without allowing them to replace canonical identity.
- Keep replay and conflict behavior deterministic and explainable.
- Keep module ownership explicit across domain, application wiring, infrastructure adapters, and public contracts.
- Remove any expectation of migration, alias, remap, or mixed-population support for this feature.

## Non-goals

- Supporting migration from legacy source identifiers or mixed legacy/canonical populations.
- Expanding the set of ingest families beyond the source registration modes already in scope.
- Changing memory-item URN generation or downstream memory-item normalization behavior.
- Moving domain rules into `app_server` or cross-cutting utility code into `core_shared` unless the behavior is truly stable and domain-agnostic.
- Making search projection state the source of truth for identity or replay decisions.

## Users / Actors

- **Source Producer**: Submits a source through manual or direct-standard ingest and expects deterministic identity and replay behavior.
- **API Consumer**: Reads source and memory-item data and expects one stable public identity plus auditable provenance.
- **System Operator**: Diagnoses duplicate, replay, and conflict outcomes through logs, traces, and contract-consistent responses.

## User Scenarios & Testing

### User Story 1 - Register a source with one canonical identity (Priority: P1)

A source producer submits a source through any supported ingest path and receives one canonical public identifier for that logical source.

**Why this priority**: Without a single canonical identity, duplicate prevention, retrieval consistency, and auditability all remain unreliable.

**Independent Test**: Submit equivalent source registrations through the supported ingest path variants and verify that the same authoritative identifiers and provenance are returned.

**Acceptance Scenarios**:

1. **Given** a manual registration request containing a canonical project-owned source URI, **When** registration succeeds, **Then** the response returns that canonical `external_id` and the deterministic internal `source_id` derived from it.
2. **Given** a direct-standard registration request containing a valid upstream identifier and trusted source-domain context, **When** registration succeeds, **Then** the system returns one canonical `external_id` and preserves the original upstream identifier separately as provenance.

---

### User Story 2 - Replay equivalent submissions without duplicates (Priority: P1)

A source producer resubmits the same logical source and receives the existing authoritative result instead of creating a duplicate source.

**Why this priority**: Idempotent replay is a core outcome for safe source registration and is the clearest business value of canonical external identity.

**Independent Test**: Submit the same logical source multiple times, including formatting-only variations where applicable, and verify that only one authoritative source exists.

**Acceptance Scenarios**:

1. **Given** an already registered canonical source, **When** the same logical payload is submitted again, **Then** the system returns the existing authoritative identifiers and does not create duplicate source or memory-item records.
2. **Given** an already registered canonical source, **When** the same canonical identity is submitted with different semantic content, **Then** the system rejects the request as a conflict without mutating authoritative state.

---

### User Story 3 - Retrieve canonical identity with provenance (Priority: P2)

An API consumer retrieves a source or related memory items and can distinguish canonical platform identity from original upstream provenance.

**Why this priority**: Retrieval must reflect the same identity model established during registration; otherwise registration correctness is not observable to callers.

**Independent Test**: Register a source through manual and direct-standard flows, retrieve the source and memory items, and verify the public provenance shape is consistent.

**Acceptance Scenarios**:

1. **Given** a source that originated from direct-standard ingest, **When** the source is retrieved, **Then** the response includes the canonical `external_id` plus the preserved original upstream identifier in the documented provenance location.
2. **Given** a source that was replayed idempotently, **When** the source is retrieved later, **Then** the returned canonical identity and provenance fields remain unchanged from the authoritative registration result.

## Edge Cases

- Manual registration submits a non-canonical or non-project-owned identifier.
- Direct-standard ingest provides an original upstream identifier that is present but cannot be safely normalized into the governed canonical URI grammar.
- Two equivalent requests arrive concurrently and must not create duplicate authoritative rows.
- A replay request changes only formatting or transport-level representation while preserving semantic payload meaning.
- A retrieval call encounters older or malformed provenance fields that do not satisfy the canonical identity model.
- Search projection or other non-authoritative read models lag behind authoritative persistence and must not redefine source identity.

## Impacted Crates

### Domain behavior ownership

- **`crates/mod_memory`**: Owns canonical external identity rules, deterministic source identity derivation, provenance semantics, and replay/conflict behavior for source registration.
- **`crates/core_shared`**: May only host genuinely cross-cutting, stable, domain-agnostic types or error primitives needed by multiple crates. It MUST NOT absorb feature-specific canonicalization logic, source provenance rules, or replay policy.

### Application / service orchestration ownership

- **`crates/app_server`**: Owns routing, handler validation, request/response mapping, and wiring of application services. It MUST NOT contain source identity domain rules.

### Infrastructure ownership

- **`crates/core_infra`**: Owns persistence and projection adapters for authoritative source state and non-authoritative search/index views. It MUST implement the domain-owned identity rules through repository contracts rather than redefining them.

### External contract implications

- Public HTTP contracts exposed through `app_server` are affected where request validation, replay responses, provenance fields, and error semantics reflect the canonical identity model.
- Downstream contract artifacts and repository-level tests are affected anywhere they assert source registration, retrieval, or replay behavior.

## Functional Requirements

- **FR-001**: The system MUST persist governed source `external_id` values only as canonical URIs under the project-owned namespace.
- **FR-002**: Manual source registration MUST accept only caller-supplied `external_id` values that already satisfy the governed canonical URI grammar.
- **FR-003**: Direct-standard ingest MUST derive the canonical `external_id` from trusted source-domain context plus the original upstream source identifier.
- **FR-004**: The system MUST preserve the original direct-standard identifier separately from the canonical `external_id` and MUST expose it only as provenance metadata.
- **FR-005**: The system MUST derive one deterministic internal `source_id` from the canonical `external_id` and MUST keep `source_id` distinct from `external_id` in all layers.
- **FR-006**: Replay and conflict decisions MUST use canonical `external_id` plus semantic payload equivalence, not raw formatting differences or transport metadata.
- **FR-007**: When the same canonical `external_id` is submitted with the same semantic payload, the system MUST return the existing authoritative source and MUST NOT create duplicate authoritative rows.
- **FR-008**: When the same canonical `external_id` is submitted with different semantic payload content, the system MUST reject the request as a conflict and MUST NOT overwrite existing authoritative state.
- **FR-009**: Registration and retrieval responses MUST expose the same public provenance shape for canonical identity version, ingest kind, semantic replay context, and original upstream identifier when present.
- **FR-010**: Public contracts MUST NOT expose migration-only aliases, alternate identifier paths, or compatibility-only fields.
- **FR-011**: The authoritative source record MUST retain the canonical identity version needed to explain how the canonical `external_id` was produced.
- **FR-012**: Search or projection records MAY reflect canonical identity for querying, but they MUST remain non-authoritative and MUST NOT govern replay or conflict outcomes.
- **FR-013**: Domain behavior for canonical identity, replay, and provenance MUST remain owned by `mod_memory`; application orchestration MUST remain in `app_server`; infrastructure adapters MUST remain in `core_infra`; and feature-specific logic MUST NOT be moved into `core_shared`.

## Data Model Implications

- **Source**:
	- Continues to represent the authoritative registered source.
	- Requires a canonical `external_id` stored under the governed URI namespace.
	- Requires a deterministic internal `source_id` that is distinct from `external_id`.
	- Requires server-managed canonical identity version metadata.
	- Requires provenance metadata for original upstream identifiers when ingest originates from a direct-standard source.
	- Requires semantic replay metadata sufficient to explain why a submission replayed or conflicted.
- **Memory Item**:
	- Keeps its existing URN and normalization semantics.
	- Inherits source identity through the authoritative `source_id` association only.
	- Does not require a new public identity model for this feature.
- **Search Projection / Read Models**:
	- May need the canonical `external_id` and related provenance fields if exposed for query or diagnostics.
	- Must remain rebuildable from authoritative source state.

## Contract / API Implications

- Manual registration contracts become stricter by rejecting non-canonical `external_id` values instead of accepting multiple equivalent representations.
- Direct-standard registration contracts must return canonical `external_id` values while preserving original upstream identifiers as provenance rather than public identity.
- Source retrieval contracts must surface the same provenance envelope returned during registration.
- Conflict and replay responses must distinguish idempotent replay from semantic conflict using stable, documented response behavior.
- Contract artifacts must not include migration-only fields, remap states, or backward-compatibility aliases.

## Operational and Performance Constraints

- Canonicalization and validation MUST complete before authoritative writes are committed.
- Concurrent equivalent registrations MUST converge on one authoritative source without leaving duplicate rows.
- Authoritative write success MUST remain independent from non-authoritative search/index availability.
- Logs, traces, and metrics MUST preserve request correlation plus domain-relevant decision reasons for canonicalization, replay, and conflict outcomes.
- This feature MUST NOT introduce unbounded retries, unbounded memory growth, or new blocking dependencies on projection systems for the authoritative write path.

## Acceptance Criteria

- **AC-001**: A manual registration request succeeds only when the submitted `external_id` already matches the governed canonical URI grammar.
- **AC-002**: A manual registration request with a non-canonical identifier is rejected before authoritative state is created.
- **AC-003**: A direct-standard registration request stores and returns a canonical `external_id` while preserving the original upstream identifier separately as provenance.
- **AC-004**: Repeating an equivalent source registration returns the same authoritative `source_id` and canonical `external_id` without creating duplicate source records.
- **AC-005**: Submitting a semantically different payload for the same canonical `external_id` returns a conflict result and leaves the existing authoritative source unchanged.
- **AC-006**: Source retrieval returns the same public provenance shape as registration for canonical identity version, ingest kind, and original upstream identifier when present.
- **AC-007**: No public contract, repository adapter, or runtime behavior requires migration-only alias lookup, mixed-population reads, or backward-compatibility remap behavior.
- **AC-008**: Search or projection lag does not change the authoritative replay or conflict outcome for a source registration.

## Success Criteria

- **SC-001**: 100% of newly accepted source registrations expose one canonical public identity across registration and retrieval responses.
- **SC-002**: 100% of semantically equivalent replay submissions resolve to the existing authoritative source instead of creating duplicates.
- **SC-003**: 100% of semantic conflicts for the same canonical identity are rejected without overwriting authoritative state.
- **SC-004**: Operators can determine why a request was accepted, replayed, or rejected for every canonical identity decision through structured diagnostics tied to the request.
- **SC-005**: Typical single-source registration requests receive a final success, replay, or conflict decision within 5 seconds under normal operating conditions.

## Test Strategy

- **Crate-local tests**:
	- `crates/mod_memory`: canonicalization rules, deterministic source identity derivation, provenance modeling, replay/conflict decisions, and normalization edge cases owned by the module.
	- `crates/core_infra`: adapter-focused tests for authoritative persistence and projection mapping where the behavior is local to the adapter contract.
	- `crates/app_server`: request validation and response mapping tests where the behavior is handler-local.
- **Repository contract tests**:
	- `tests/contract`: registration request/response shape, retrieval provenance shape, conflict semantics, and storage/projection contract assertions that cross crate boundaries.
- **Repository integration tests**:
	- `tests/integration`: cross-module flows covering manual registration, direct-standard registration, replay detection, conflict handling, retrieval consistency, and multi-instance convergence.
- **Performance verification**:
	- `tests/perf`: verify registration and replay decision timing against the published success criteria where SLO enforcement is required.
	- `benches/`: benchmark canonicalization and replay-heavy workloads only when micro-benchmarking is needed to isolate hot paths.

The test layout MUST keep behavior tests as close as possible to the owning crate. Repository-level integration tests are reserved for genuinely cross-module or end-to-end flows.

## Risks / Open Questions / Assumptions

### Risks

- Existing records or fixtures may still assume non-canonical or mixed-identity behavior and require synchronized updates.
- Direct-standard ingest can become ambiguous if trusted source-domain context is incomplete or inconsistent.
- Projection contracts can drift from authoritative persistence if canonical identity fields are updated in one layer but not the other.

### Open Questions

- None blocking for this specification. If the pre-production assumption changes and legacy source rows must be preserved, this spec will need a follow-up clarification before planning or implementation proceeds.

### Assumptions

- The repository remains pre-production, so no legacy migration or compatibility behavior is required.
- `001-memory-ingest` remains the baseline authoritative ingest slice and this feature refines its source identity model rather than replacing the ingest workflow.
- Memory-item URN generation remains unchanged.
- Trusted source-domain context is available wherever direct-standard ingest is supported.

## Known Unknowns

- Whether future ingest families beyond the currently supported set will require additional canonical identity grammar variants.
- Whether any external consumers depend on non-canonical identifiers in fixtures or unpublished integration environments.

## Recommended Next Command

- `/speckit.plan` if the pre-production and no-migration assumptions remain accepted.
- `/speckit.clarify` if legacy compatibility, additional ingest families, or trusted source-domain derivation rules need to be revisited.
