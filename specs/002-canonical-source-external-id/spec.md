# Feature Specification: Canonical Source External ID and Direct-Standard Ingest Alignment

**Feature Branch**: `002-canonical-source-external-id`  
**Created**: 2026-03-18  
**Status**: Ready for Tasks  
**Input**: User description: "001-memory-ingest에 대한 후속 변경 스펙을 작성하라. 주제는 canonical source external_id 도입 및 direct-standard ingest 정렬이다."

## Problem & Context

`001-memory-ingest` established the first authoritative ingest flow, but it left source identity, replay semantics, and legacy rollout safety underspecified. The current system allows `external_id` to mean different things across ingest modes, uses a legacy `canonical_payload_hash` term that drifts from the intended semantic comparator, and does not define an operator-safe migration model for deterministic `source_id` adoption.

This feature closes those gaps. It standardizes one canonical source identity grammar, preserves direct-standard provenance separately, replaces ambiguous replay terminology with one authoritative semantic hash contract, and defines the migration and observability model required to roll the change out safely.

## Relationship to 001-memory-ingest

- This feature amends the meaning of canonical `external_id` introduced by `001-memory-ingest`.
- This feature preserves the identifier-role separation established in `001-memory-ingest`: internal `source_id`, canonical `external_id`, and derived memory-item URNs remain distinct.
- This feature replaces the `canonical_payload_hash` vocabulary from `001-memory-ingest` with the authoritative term `semantic_payload_hash` and constrains the old name to one-way migration compatibility only.
- This feature requires deterministic UUID v5 `source_id` derivation for every persisted source row and defines the migration posture that removes legacy non-deterministic rows from steady state.

## Clarifications

### Session 2026-03-18

- Q: Which `source_domain` normalization pipeline governs canonical identity? → A: Trim whitespace, parse as URL or authority input, remove scheme if present, extract host only, lowercase it, remove one leading `www.`, remove trailing dot, remove port, preserve remaining subdomains exactly, normalize IDNs to ASCII punycode, and reject userinfo, path-derived host, query-derived host, and ambiguous authority forms.
- Q: Which `object_id` normalization policy governs canonical identity? → A: Trim outer whitespace only, reject empty after trim, preserve internal spaces and case, preserve unreserved characters as-is, percent-encode spaces and reserved or non-unreserved characters from UTF-8 bytes, enforce raw and encoded length bounds, and reject destructive stripping or collapsing.
- Q: How are standard and version vocabulary governed? → A: Persist authoritative family and version vocabulary in `specs/002-canonical-source-external-id/contracts/canonical-vocabulary.yaml`; accept aliases only as input mapping hints; persist only canonical tokens.
- Q: How are original standard identifiers preserved and surfaced? → A: Preserve them as `source_metadata.system.original_standard_id`, keep `external_id` as the primary identity, and expose provenance in both registration and retrieval responses using the same public metadata shape.
- Q: What replay and conflict semantics govern canonical identity adoption? → A: `same canonical external_id + same semantic_payload_hash = replay`; `same canonical external_id + different semantic_payload_hash = conflict`; `raw_body_hash` is diagnostic-only.
- Q: What is the authoritative source-id seed contract? → A: The seed string is `source|{canonical_id_version}|{canonical_external_id}`; `canonical_id_version` is part of the seed for every new or migrated row.
- Q: How are legacy rows classified for rollout? → A: Every legacy row is classified as `migratable`, `consolidate`, or `unmigratable`; cutover proceeds only with `unmigratable = 0` and `conflict_groups = 0`.
- Q: What is the mixed-population posture? → A: Mixed legacy and new rows are tolerated only during an offline migration window with registration writes disabled; steady state after cutover contains only canonical rows with deterministic UUID v5 `source_id` values.
- Q: What is the observability surface for this feature? → A: Structured logs, traces, and bounded-cardinality metrics are mandatory internal diagnostics; provenance fields in API responses and request-correlation headers are the public contract surface.

## Goals

- **G1**: Define one project-owned canonical URI grammar for `external_id` that applies consistently to canonical/manual ingest and direct-standard ingest.
- **G2**: Preserve the original third-party standard identifier separately from the canonical system identifier.
- **G3**: Make replay, idempotency, and conflict outcomes depend on canonical identity plus semantic payload equivalence rather than raw payload formatting.
- **G4**: Introduce explicit versioning for canonical identifier grammar and include it in deterministic `source_id` derivation.
- **G5**: Define a complete migration, rollback, and mixed-population safety model for rewriting all stored `source_id` values to deterministic UUID v5.
- **G6**: Operationalize constitution-level observability requirements for canonicalization, replay, conflict, lookup, and migration decisions.
- **G7**: Align specification, contract, domain model, and runbook artifacts so `/speckit.tasks` can generate implementation work without ambiguity.

## Non-Goals

- Changing the existing memory-item URN grammar or its deterministic UUID v5 behavior.
- Expanding the set of direct-standard payload families beyond the Open Badges and CLR profiles inherited from `001-memory-ingest`.
- Supporting a steady-state mixed legacy and canonical source population after rollout.

## Users & Actors

- **Source Producer**: Registers sources through canonical/manual ingest and needs a stable canonical identity contract.
- **Standard Ingest Producer**: Submits supported standard payloads and needs the platform to preserve the original payload identifier without confusing it with canonical identity.
- **Operator**: Executes migration and needs a deterministic dry-run, bounded rollback posture, and operator-facing diagnostics.
- **Contract and QA Owner**: Maintains API contracts and regression suites and needs one unambiguous identifier, provenance, replay, and migration model.

## User Scenarios & Testing

### User Story 1 - Canonical Identity Is Consistent Across Ingest Modes (Priority: P1)

A source producer registers content through canonical/manual ingest or direct-standard ingest and receives the same kind of canonical `external_id`: a project-owned URI that encodes standard, version, source domain, and object identity.

**Independent Test**: Register one canonical/manual request and one direct-standard request, then verify both produce `external_id` values that conform to the same canonical URI grammar and preserve original standard provenance separately.

**Acceptance Scenarios**:

1. **Given** a canonical/manual ingest request with a valid canonical `external_id`, **When** registration succeeds, **Then** the returned and persisted `external_id` remains exactly that canonical URI and the response includes public provenance metadata with `canonical_id_version`, `ingest_kind`, and `semantic_payload_hash`.
2. **Given** a direct-standard payload whose institution context, standard family, and version can be determined, **When** registration succeeds, **Then** the system stores a canonical URI in `external_id`, stores the payload's original standard `id` separately in server-managed metadata, and returns the same provenance envelope shape as retrieval.
3. **Given** a direct-standard payload whose source domain cannot be determined confidently, **When** registration is attempted, **Then** the request is rejected and a structured diagnostic reason is emitted.

### User Story 2 - Replay and Conflict Decisions Follow Canonical Semantics (Priority: P1)

A downstream consumer replays the same logical source and expects formatting-only differences or raw-standard-id representation differences not to create duplicate authoritative records, while true semantic changes for the same canonical source identity are rejected as conflicts.

**Independent Test**: Submit the same logical source multiple times with semantically equivalent and semantically different payloads and verify replay or conflict behavior against the canonical URI plus semantic payload hash.

**Acceptance Scenarios**:

1. **Given** the same canonical `external_id` and the same `semantic_payload_hash`, **When** the source is resubmitted with formatting-only differences, **Then** the system returns the existing authoritative identifiers rather than creating duplicates.
2. **Given** the same canonical `external_id` but a different `semantic_payload_hash`, **When** the source is resubmitted, **Then** the system rejects the request as a conflict.
3. **Given** two supported standard payloads that normalize to the same canonical URI, **When** both are submitted, **Then** replay or conflict decisions are made from canonical URI plus semantic payload hash and never from raw payload `id` spelling alone.

### User Story 3 - Provenance Remains Auditable (Priority: P1)

A producer or operator inspects a registered source and can distinguish the canonical platform identity from the original standard identifier and the grammar version used to create it.

**Independent Test**: Retrieve a governed source created through either ingest path and verify that registration and retrieval expose the same public provenance shape.

**Acceptance Scenarios**:

1. **Given** a source created through direct-standard ingest, **When** registration or retrieval returns the source, **Then** `source_metadata.system` includes `canonical_id_version`, `ingest_kind`, `original_standard_id`, and `semantic_payload_hash`.
2. **Given** a source created through canonical/manual ingest, **When** registration or retrieval returns the source, **Then** `source_metadata.system` includes `canonical_id_version`, `ingest_kind`, and `semantic_payload_hash`, and excludes `original_standard_id`.
3. **Given** a governed source, **When** the operator inspects logs or traces for the request, **Then** the request or handler correlation context, canonical identity, and decision reason are traceable without consulting raw payload content.

### User Story 4 - Existing Records Migrate Safely To Deterministic Source IDs (Priority: P1)

A system operator rolls out the new canonical identity and deterministic `source_id` rules with a controlled migration so every stored source record ends up with a deterministic UUID v5 `source_id` and no rollout-blocking ambiguity remains.

**Independent Test**: Run dry-run classification against legacy rows, execute migration against a controlled dataset, and verify retrieval, replay, reference integrity, and rollback criteria.

**Acceptance Scenarios**:

1. **Given** a legacy source row that can derive a canonical external identity and a deterministic seed, **When** migration runs, **Then** the row is classified as `migratable` or `consolidate`, receives the deterministic UUID v5 target `source_id`, and all dependent references resolve correctly.
2. **Given** a legacy row that cannot derive a canonical external identity, has inconsistent hash aliases, or collides semantically with another row, **When** dry-run or execution reaches it, **Then** the run is marked failed and cutover does not proceed.
3. **Given** a migration window, **When** the system is in partial rewrite state, **Then** registration writes are denied and reads resolve through the documented legacy resolution path until verification passes or rollback restores the snapshot.

## Edge Cases

- A canonical/manual ingest request supplies an `external_id` outside the project-owned namespace; the request fails validation and emits `MANUAL_CANONICAL_REJECTED`.
- A direct-standard payload contains a valid original `id` but no trustworthy institution domain; the request fails with `DIRECT_STANDARD_REJECTED_UNTRUSTED_DOMAIN`.
- Two payloads differ only in case, percent-encoding, punctuation, or spaces that normalization rules preserve; they remain distinct unless the documented normalization rules say they are equivalent.
- An `object_id` contains reserved URI characters such as `/`, `?`, `#`, or spaces; the system preserves identity through deterministic encoding rather than dropping those characters.
- Two legacy rows resolve to the same canonical identity and the same semantic payload hash; migration consolidates them to one deterministic target row.
- Two legacy rows resolve to the same canonical identity and different semantic payload hashes; migration aborts.
- A migration dry-run encounters a row with only `canonical_payload_hash`; the run maps it into `semantic_payload_hash`, reports the alias consumption, and fails if the authoritative hash cannot be reconstructed consistently.

## Identity, Migration, and Observability Constraints

- The feature uses all three identifier roles and keeps them separate: internal `source_id`, canonical external `external_id`, and derived memory-item URNs.
- `source_id` is derived from the exact seed string `source|{canonical_id_version}|{canonical_external_id}` for every new or migrated row.
- Canonical `external_id` uses the project-owned grammar `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}`.
- The normative canonical URI examples remain:
  - `https://api.cherry-pick.net/qti/v3p0/kice.re.kr:20240621`
  - `https://api.cherry-pick.net/cc/v1p3/nebooks.co.kr:eng3-ch01`
  - `https://api.cherry-pick.net/case/v1p0/moe.go.kr:6ma01-01`
- `canonical_id_version` is `v1` for this feature and is persisted on every governed row.
- `semantic_payload_hash` is the only authoritative replay and conflict comparator.
- `raw_body_hash` is retained only for diagnostics and migration audit when a raw body exists; it is never an authoritative replay input and is never exposed by the public API contract.
- `canonical_payload_hash` is a legacy compatibility alias that is read only during migration classification and is deleted from authoritative storage as rows are rewritten.
- Mixed legacy and canonical populations are tolerated only during the offline migration window. Registration writes are denied during that window. Retrieval resolves through a deterministic remap path until verification passes.
- Structured logs, traces, and metrics are mandatory internal diagnostics. Public observability surface is limited to request-correlation headers and the public provenance envelope in API responses.

## Requirements

### Functional Requirements

- **FR-001**: The system MUST treat `external_id` as a canonical source identity under the project-owned namespace `https://api.cherry-pick.net/` and MUST reject new writes whose persisted `external_id` would not conform to the grammar `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}`.
- **FR-002**: The system MUST enforce the vocabulary registry in `specs/002-canonical-source-external-id/contracts/canonical-vocabulary.yaml`. Aliases may be accepted as input mapping hints and are never persisted.
- **FR-003**: The system MUST normalize `source-domain` by trimming whitespace, accepting bare authority or URL-like input, removing any scheme, extracting the host only, lowercasing it, removing one leading `www.`, removing any trailing dot and port, and normalizing IDNs to ASCII punycode. The system MUST reject userinfo, path contamination, query-derived host, and ambiguous authority inputs.
- **FR-004**: The system MUST normalize `object_id` by trimming outer whitespace only, rejecting empty values after trim, preserving case and internal spacing, percent-encoding spaces plus reserved or non-unreserved bytes, enforcing a maximum raw length of 256 UTF-8 bytes and a maximum normalized length of 1024 ASCII characters, and rejecting destructive stripping or collapsing.
- **FR-005**: The system MUST preserve the raw direct-standard payload identifier as `source_metadata.system.original_standard_id` and MUST keep it distinct from canonical `external_id`.
- **FR-006**: The authoritative server-managed provenance fields are `canonical_id_version`, `ingest_kind`, `semantic_payload_hash`, and `original_standard_id` when present. These fields are exposed in registration and retrieval responses using the same public metadata shape.
- **FR-007**: Direct-standard ingest MUST derive canonical `external_id` from the resolved `standard`, `version`, `source-domain`, and `object_id`, preserve the original standard `id` separately, and reject the request if any canonical component cannot be determined confidently.
- **FR-008**: Canonical/manual ingest MUST accept a caller-supplied `external_id` only when that value already matches the canonical URI grammar.
- **FR-009**: Replay and idempotency semantics MUST use canonical `external_id` together with `semantic_payload_hash`.
- **FR-010**: `same canonical external_id + same semantic_payload_hash` MUST return replay; `same canonical external_id + different semantic_payload_hash` MUST return conflict.
- **FR-011**: The authoritative persisted replay-comparator field name is `semantic_payload_hash`. The old field name `canonical_payload_hash` MUST be read only during migration classification and MUST be removed from authoritative storage on rewritten rows.
- **FR-012**: The diagnostics-only field `raw_body_hash` MUST be stored only when the raw body exists, MUST never participate in replay or conflict decisions, MUST never appear in metrics labels, and MUST never appear in public API responses.
- **FR-013**: The deterministic `source_id` seed MUST be `source|{canonical_id_version}|{canonical_external_id}`. No other seed components are permitted.
- **FR-014**: Legacy row seed completion MUST derive `canonical_external_id` and set `canonical_id_version = v1` before target `source_id` generation. Legacy-only seed branches are prohibited.
- **FR-015**: Every legacy row in rollout scope MUST classify as `migratable`, `consolidate`, or `unmigratable`. `migratable` means canonical identity, deterministic seed, and dependent reference rewrite are all complete. `consolidate` means multiple rows resolve to one canonical identity with the same semantic payload hash. `unmigratable` means canonical identity, semantic equivalence, or reference rewrite cannot be completed safely.
- **FR-016**: Migration dry-run MUST produce a machine-readable JSON report whose authoritative per-row schema is fixed across specification, plan, data model, quickstart, and tasks. Every row MUST include `legacy_source_id`, `legacy_external_id`, `candidate_canonical_external_id`, `candidate_source_id`, `candidate_source_seed`, `classification`, `decision_reason`, `legacy_resolution_path`, `canonical_id_version`, `semantic_payload_hash`, `raw_body_hash_present`, `dependent_reference_counts`, and `planned_action`. Every row MUST include `original_standard_id` when present on the source being classified. Every row MUST include `raw_body_hash` only when `raw_body_hash_present = true` and MUST omit `raw_body_hash` when `raw_body_hash_present = false`.
- **FR-016a**: For every classified row, migration dry-run and migration verification MUST recompute `candidate_source_id` from the exact governed seed contract `source|{canonical_id_version}|{candidate_canonical_external_id}` and MUST fail if the emitted `candidate_source_id` does not equal the UUID v5 derived from that exact seed. The verification output MUST make per-row seed reproducibility reviewable from the emitted row data alone.
- **FR-017**: Rollout cutover MUST stop when dry-run or execution reports any `unmigratable` row, any duplicate canonical identity with divergent semantic payload hashes, any missing dependent reference rewrite, any verification query mismatch, or any snapshot or backup gate failure.
- **FR-018**: The system MUST deny registration writes during the partial migration window. Retrieval remains available and MUST resolve source lookups using the documented legacy resolution path until cutover verification succeeds or rollback restores the snapshot.
- **FR-019**: Observability MUST emit structured logs, traces, and metrics for canonicalization, replay, conflict, migration classification, migration execution, and mixed-population lookup decisions. Required diagnostic fields are `canonical_external_id`, `original_standard_id`, `canonical_id_version`, `semantic_payload_hash`, `raw_body_hash` policy, `migration_phase`, `legacy_resolution_path`, `decision_reason`, and request or handler correlation context.
- **FR-020**: Metrics MUST use bounded-cardinality labels only. Hash values, `canonical_external_id`, and `original_standard_id` are logs-and-traces-only fields.
- **FR-021**: The runbook MUST define pre-migration checklist items, dry-run acceptance criteria, verification query requirements, snapshot and backup gates, rollback posture, stop conditions, rewrite completeness threshold, and final sign-off gates.
- **FR-022**: The normalization regression suite is mandatory. It MUST cover the object-id collision matrix, source-domain edge matrix, and canonical URI golden-output stability cases listed in this specification.
- **FR-023**: Public contracts and documentation MUST stop implying that direct-standard payload `id` is the canonical external identifier.

### Non-Functional Constraints

- Canonicalization is deterministic, documented, and versioned.
- Provenance is auditable at the API and diagnostics layers without exposing raw body content.
- Validation failures for canonical identity inputs fail before authoritative state is created.
- Migration risk is bounded by offline write freeze, snapshot-backed rollback, deterministic verification, and zero-tolerance stop conditions.

## Key Entities

- **Canonical Source Identifier**: The authoritative external identity for a source, stored in `external_id` as a project-owned URI composed of `standard`, `version`, `source-domain`, and `object-id`.
- **Deterministic Source Identifier**: The internal `source_id` for every stored row after migration, derived as UUID v5 from `source|{canonical_id_version}|{canonical_external_id}`.
- **Source Identity Provenance**: Reserved server-managed metadata describing canonical identity and replay semantics using `canonical_id_version`, `ingest_kind`, `semantic_payload_hash`, and `original_standard_id` when present.
- **Migration Classification Report**: The machine-readable dry-run or execution artifact that classifies each legacy row as `migratable`, `consolidate`, or `unmigratable` and records the operator action.
- **Operational Decision Diagnostic**: The internal structured log or trace event that records canonicalization, replay, conflict, migration, or legacy lookup decisions using the required diagnostics fields.

## Acceptance Criteria

- **AC-001**: New canonical/manual registrations succeed only when `external_id` already conforms to the canonical URI grammar.
- **AC-002**: New direct-standard registrations persist canonical URI `external_id` values and retain the raw standard payload `id` separately in `source_metadata.system.original_standard_id`.
- **AC-003**: Registration and retrieval responses expose the same public provenance shape using `source_metadata.system.canonical_id_version`, `source_metadata.system.ingest_kind`, `source_metadata.system.semantic_payload_hash`, and `source_metadata.system.original_standard_id` when present.
- **AC-004**: `same canonical external_id + same semantic_payload_hash` replays the authoritative row; `same canonical external_id + different semantic_payload_hash` returns conflict.
- **AC-005**: `raw_body_hash` is never returned by the public API, never used for replay or conflict, and never used as a metric label.
- **AC-006**: Every in-scope legacy row is classified in dry-run as `migratable`, `consolidate`, or `unmigratable`, every dry-run row conforms to the authoritative row schema in `FR-016`, every row's `candidate_source_id` is reproducibly recomputed from `source|{canonical_id_version}|{candidate_canonical_external_id}`, and cutover proceeds only when `unmigratable = 0` and `conflict_groups = 0`.
- **AC-007**: During the partial migration window, registration writes are denied and retrieval resolves through the documented legacy resolution path.
- **AC-008**: After cutover, every authoritative source row uses deterministic UUID v5 `source_id`, every persisted `external_id` is canonical, and no authoritative row retains `canonical_payload_hash`.
- **AC-009**: The mandatory object-id collision matrix covers `eng3-ch01`, `eng3_ch01`, `eng3ch01`, reserved URI characters, spaces, case preservation, and raw-length versus encoded-length boundaries.
- **AC-010**: The mandatory source-domain edge matrix covers scheme stripping, port removal, `www.` normalization, trailing dot handling, IDN punycode, userinfo rejection, path contamination rejection, query-derived host rejection, and ambiguous authority rejection.
- **AC-011**: Canonical URI regression covers golden outputs, alias non-leakage, and namespace output stability.
- **AC-012**: Structured logs, traces, and metrics contain the required diagnostic fields and decision taxonomy for registration, replay, conflict, migration classification, migration execution, and legacy lookup resolution.

## Success Criteria

### Measurable Outcomes

- **SC-001**: 100% of sources created after feature enablement persist canonical `external_id` values and record `canonical_id_version = v1`.
- **SC-002**: 100% of replay validations for semantically equivalent submissions resolve to the existing authoritative identifiers.
- **SC-003**: 100% of semantically conflicting submissions for the same canonical identity are rejected.
- **SC-004**: 100% of direct-standard rows preserve `original_standard_id` separately from canonical `external_id`.
- **SC-005**: Migration dry-run achieves `unmigratable = 0`, `conflict_groups = 0`, and complete dependent-reference enumeration before cutover.
- **SC-006**: Post-cutover verification finds zero legacy `source_id` rows outside the deterministic UUID v5 contract and zero authoritative rows retaining `canonical_payload_hash`.
- **SC-007**: Reviewers can compare spec, plan, data model, quickstart, and OpenAPI contract and find no contradictory definitions of `external_id`, `original_standard_id`, `canonical_id_version`, `semantic_payload_hash`, `raw_body_hash`, or migration behavior.

## Open Questions

None.

## Recommended Next Command

- `/speckit.tasks` to decompose the closed design into implementation tasks.
