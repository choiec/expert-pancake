# Feature Specification: Schema-Native Standard Credential Registry

**Feature Branch**: `001-memory-ingest`  
**Created**: 2026-03-17  
**Updated**: 2026-03-21  
**Status**: IMPLEMENT-READY

## Purpose

Replace the current canonical `Source` / `MemoryItem` public model with a schema-native credential API. The authoritative write and read surface is now the supported standard credential document itself, not a protocol-neutral wrapper.

## Context

The current slice already accepts direct Open Badges and CLR payloads, but its public contract still exposes service-owned wrapper concepts such as canonical `external_id`, internal `source_id`, derived `memory_items`, and provenance envelopes. This redesign removes that compatibility layer and makes the supported credential schema the public contract.

The expected outcome is:

- one authoritative public identifier: the standard credential `id`
- one authoritative document shape: the supported credential envelope with official top-level schema keys only
- one replay rule: same `id` plus semantic payload equality replays successfully
- one conflict rule: same `id` plus semantic payload difference returns `409 Conflict`
- one authoritative retrieval shape: the stored schema-exact credential document

## Goals

- **G1**: Accept supported standard credential payloads as first-class authoritative write requests.
- **G2**: Persist authoritative credential records using only official top-level schema keys and official key names.
- **G3**: Make the standard credential `id` the public retrieval key and remove service-owned canonical identity wrappers from the public API.
- **G4**: Resolve replay and conflict decisions from standard credential identity plus semantic payload equality.
- **G5**: Keep search non-authoritative while aligning its projection vocabulary with schema-native credential concepts.
- **G6**: Preserve health and readiness contracts that reflect authoritative write-path health separately from search health.
- **G7**: Keep the slice runnable and verifiable end-to-end through updated machine-readable contracts, quickstart flows, and automated tests.

## Non-Goals

- Canonical/manual ingest for text or markdown documents.
- Public `source_id`, `external_id`, `urn`, `memory_item`, or `indexing_status` fields.
- Raw-body retrieval guarantees for authoritative read APIs.
- Compatibility aliases or migration shims for the previous `/sources/*` or `/memory-items/*` contract family.
- Standard families beyond the pinned Open Badges 3.0 and CLR 2.0 credential envelopes.

## User Scenarios & Testing

### User Story 1 - Register a standard credential (Priority: P1)

A producer submits an Open Badges or CLR credential and receives the authoritative schema-native credential representation back.

**Why this priority**: This is the core business value of the slice. Without authoritative registration, the rest of the system has nothing stable to retrieve or project.

**Independent Test**: Submit valid Open Badges and CLR credentials to the registration endpoint and verify that the response returns the authoritative credential document with only official top-level schema keys.

**Acceptance Scenarios**:

1. **Given** a valid Open Badges credential, **When** registration succeeds, **Then** the response is `201 Created` and the body is the authoritative credential document keyed only by official schema fields.
2. **Given** a valid CLR credential, **When** registration succeeds, **Then** the response is `201 Created` and the body is the authoritative credential document keyed only by official schema fields.
3. **Given** a credential payload that includes unsupported top-level fields, **When** registration is attempted, **Then** the request is rejected before persistence.

### User Story 2 - Replay equivalent submissions without duplicates (Priority: P1)

A producer resubmits the same logical credential and receives the existing authoritative record rather than creating duplicates.

**Why this priority**: Replay safety is required for reliable ingest and for concurrent producer behavior.

**Independent Test**: Register the same credential twice with formatting-only JSON differences and verify that the second request returns `200 OK` and does not create a second authoritative record.

**Acceptance Scenarios**:

1. **Given** an existing authoritative credential with the same standard `id`, **When** the producer resubmits a semantically equivalent payload, **Then** the system returns `200 OK` with the existing authoritative credential and does not create duplicate state.
2. **Given** an existing authoritative credential with the same standard `id`, **When** the producer submits a semantically different payload, **Then** the system returns `409 Conflict` and leaves authoritative state unchanged.

### User Story 3 - Retrieve authoritative credential documents (Priority: P1)

A consumer retrieves a stored credential using the official credential identifier and receives the authoritative schema-native document without wrapper-specific fields.

**Why this priority**: Registration is not useful unless consumers can later retrieve the same authoritative credential state directly.

**Independent Test**: Register a credential, retrieve it by its official `id`, and verify that the returned document matches the stored authoritative schema-exact record.

**Acceptance Scenarios**:

1. **Given** a stored supported credential, **When** retrieval by its official `id` succeeds, **Then** the response body contains the authoritative credential document and no service-owned wrapper fields.
2. **Given** an unknown credential `id`, **When** retrieval is attempted, **Then** the system returns `404 Not Found`.

### User Story 4 - Search credential projections without changing authority (Priority: P2)

A consumer searches credential summaries while authoritative registration and retrieval remain governed only by the authoritative credential store.

**Why this priority**: Search improves usability, but it must not redefine what the authoritative credential is.

**Independent Test**: Query the search endpoint and verify that it returns projection hits built from credential data, while authoritative registration and retrieval continue to work when search is degraded.

**Acceptance Scenarios**:

1. **Given** indexed credentials, **When** the credential search endpoint is queried, **Then** the response returns projection hits derived from stored credential data.
2. **Given** search degradation, **When** registration and authoritative retrieval are exercised, **Then** both continue to work and search may return `503 Service Unavailable`.

## Edge Cases

- A credential omits the required standard `id`.
- A credential uses the same `id` as an existing stored credential but belongs to a semantically different document.
- A credential includes non-standard top-level keys.
- A payload is schema-valid JSON but cannot be classified to exactly one supported family.
- Two concurrent requests register the same credential at nearly the same time.
- Search indexing is delayed or unavailable after a successful authoritative write.

## Requirements

### Functional Requirements

- **FR-001**: `POST /credentials/register` MUST accept supported Open Badges 3.0 and CLR 2.0 credential payloads as first-class authoritative requests.
- **FR-002**: The system MUST reject unsupported credential families and unsupported top-level fields before any authoritative persistence occurs.
- **FR-003**: The authoritative public identity for a stored credential MUST be the official standard `id` field from the accepted credential.
- **FR-004**: The system MUST persist authoritative credential documents using only official top-level keys defined by the pinned family schema and MUST preserve the official schema key names exactly.
- **FR-005**: The system MUST preserve the accepted nested content under official top-level keys without introducing derived top-level fields into the authoritative credential document.
- **FR-006**: Replay and conflict decisions MUST use the standard credential `id` plus semantic payload equivalence; formatting-only JSON differences MUST NOT create conflicts.
- **FR-007**: A replay with the same credential `id` and the same semantic payload MUST return the existing authoritative credential and MUST NOT create duplicate rows.
- **FR-008**: A submission with the same credential `id` and a different semantic payload MUST return `409 Conflict` and MUST NOT overwrite authoritative state.
- **FR-009**: `GET /credentials/{credential-id}` MUST return the authoritative stored credential document without service-owned wrapper fields.
- **FR-010**: The public authoritative API MUST NOT expose `source_id`, `external_id`, `urn`, `memory_items`, `source_metadata`, or compatibility-only aliases.
- **FR-011**: `GET /credentials/search` MUST return projection hits only and MUST remain non-authoritative.
- **FR-012**: `/health` MUST be local-only liveness and `/ready` MUST reflect authoritative write-path readiness while allowing search degradation to be reported separately.
- **FR-013**: Public contracts MUST remain machine-readable and must pin the schema-native request and response shapes used by registration, retrieval, search, and probes.

### Non-Functional Requirements

- **NC-001**: Registration requests for representative supported credential payloads under 100 KB MUST meet a p95 latency target of 5 seconds or less in release validation.
- **NC-002**: Authoritative retrieval requests for representative stored credentials MUST meet a p95 latency target of 200 ms or less in release validation.
- **NC-003**: Credential search projection queries against the representative benchmark corpus MUST meet a p95 latency target of 500 ms or less in release validation.
- **NC-004**: Concurrent duplicate registrations across stateless app instances MUST converge on one authoritative credential outcome without data divergence.
- **NC-005**: Authoritative writes MUST be transactional and leave no partial authoritative state behind after timeout, conflict, or storage failure.
- **NC-006**: Search indexing and projection behavior MUST remain non-authoritative; search degradation or backlog MUST NOT block authoritative registration or retrieval.
- **NC-007**: Supported credential payloads MUST be validated against pinned allow or reject rules before any authoritative persistence occurs.
- **NC-008**: Logs, metrics, and error payloads MUST avoid emitting full credential bodies or arbitrary caller metadata that could leak sensitive information.
- **NC-009**: Authoritative credential records in this slice MUST default to a no-TTL, no-automatic-purge retention baseline unless a later spec changes that policy.

### Key Entities

- **Standard Credential Record**: The authoritative stored credential document keyed by the official standard `id` and containing only official top-level schema keys for the supported family.
- **Credential Search Projection**: A non-authoritative search document derived from authoritative credential data and safe to rebuild from the authoritative store.
- **Credential Index Job**: The durable outbox record that links a committed authoritative credential write to asynchronous search projection work.

## Success Criteria

### Measurable Outcomes

- **SC-001**: Producers can register representative Open Badges and CLR credentials and receive schema-native authoritative responses without wrapper-specific fields.
- **SC-002**: Equivalent replays of the same credential `id` return the existing authoritative record without creating duplicates in 100% of tested replay scenarios.
- **SC-003**: Conflicting submissions for the same credential `id` return `409 Conflict` without mutating stored authoritative state in 100% of tested conflict scenarios.
- **SC-004**: Consumers can retrieve stored credentials by official `id` and receive the authoritative schema-exact document in 100% of tested retrieval scenarios.
- **SC-005**: Search degradation does not change authoritative registration or retrieval outcomes in the documented smoke and integration flows.

## Acceptance Criteria

- **AC-001**: Registration succeeds only for supported Open Badges 3.0 and CLR 2.0 credential payloads.
- **AC-002**: Successful authoritative responses expose only schema-native credential fields and no service-owned wrapper fields.
- **AC-003**: Equivalent replays return the same authoritative credential document without duplicates.
- **AC-004**: Semantic conflicts for the same credential `id` return `409 Conflict` without mutating authoritative state.
- **AC-005**: Retrieval by official `id` returns the authoritative schema-exact credential document.
- **AC-006**: Search remains projection-only and does not alter authoritative write or read behavior.
- **AC-007**: Health and readiness endpoints keep distinct liveness versus dependency-aware semantics after the redesign.

## Test Strategy

- Contract tests pin the schema-native registration, retrieval, search, health, and readiness shapes.
- Integration tests cover Open Badges and CLR registration, replay, conflict, retrieval, and degraded-search behavior.
- Storage adapter contract tests cover schema-exact persistence, uniqueness, rollback, and retention guarantees.
- Performance verification remains a release gate for registration, retrieval, and search latency targets.
