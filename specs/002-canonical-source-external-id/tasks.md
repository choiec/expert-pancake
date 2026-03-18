# Tasks: Canonical Source External ID and Direct-Standard Ingest Alignment

**Input**: Design documents from `/specs/002-canonical-source-external-id/`
**Status**: READY FOR IMPLEMENTATION REVIEW
**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/canonical-vocabulary.yaml`, `contracts/memory-ingest.openapi.yaml`, `quickstart.md`

## Task Generation Assumptions

- The constitution already establishes canonical identifier governance, so Phase 1 is a downstream alignment sweep rather than a net-new governance proposal.
- Tests are required for this feature because the constitution and spec explicitly require contract, integration, provenance, replay/conflict, and migration verification.
- Tasks are split so documentation and contract changes complete before implementation work begins.
- File paths below may include new Rust modules where the current workspace benefits from narrower, reviewable ownership boundaries for normalization and source-identity logic.
- User Story 1 is the MVP because it establishes one canonical `external_id` contract across canonical/manual and direct-standard ingest.

## Dependency Notes

- Phase 1 is mandatory pre-implementation documentation work. Do not begin code changes until it is complete.
- Phase 2 is blocking for every user story because canonicalization helpers, parser/validator logic, and deterministic source-id derivation define the shared identity model.
- User Story 1 depends on Phase 2 and establishes the registration boundary and persisted provenance shape.
- User Story 2 depends on User Story 1 because replay/conflict semantics operate on the canonical identity and metadata persisted there.
- User Story 3 depends on User Story 1 because retrieval cannot expose provenance fields until they are persisted.
- User Story 4 depends on User Stories 1 and 2 because migration must move stored rows to the final deterministic identity and replay model.
- Polish work depends on all selected user stories completing.

## Phase 1: Setup (Documentation and Contract Alignment)

**Purpose**: Finish the constitution/spec/contract alignment that must be settled before implementation starts.

- [ ] T001 Align canonical identifier governance language across .specify/memory/constitution.md, specs/001-memory-ingest/spec.md, specs/001-memory-ingest/data-model.md, specs/002-canonical-source-external-id/spec.md
Title: Align constitution and source-ingest specifications.
Purpose: Remove drift between the constitution amendment and the 001/002 feature artifacts so canonical `external_id`, deterministic `source_id`, and provenance terminology mean the same thing everywhere.
Inputs / dependencies: Constitution v2.0.0, `specs/002-canonical-source-external-id/spec.md`, `specs/002-canonical-source-external-id/plan.md`.
Files to touch: `.specify/memory/constitution.md`, `specs/001-memory-ingest/spec.md`, `specs/001-memory-ingest/data-model.md`, `specs/002-canonical-source-external-id/spec.md`.
Expected outputs: Cross-artifact terminology alignment for `external_id`, `source_id`, `canonical_id_version`, `original_standard_id`, and replay/conflict semantics.
Acceptance check: A reviewer can compare the constitution and both feature specs without finding contradictory identifier definitions.
Parallelizable: No.

- [ ] T002 Define canonical external_id grammar and vocabulary registry details in specs/002-canonical-source-external-id/contracts/canonical-vocabulary.yaml, specs/002-canonical-source-external-id/research.md, specs/002-canonical-source-external-id/data-model.md
Title: Define canonical grammar and registry inputs.
Purpose: Freeze the grammar, canonical tokens, alias policy, and normalization examples that implementation must follow.
Inputs / dependencies: T001, `specs/002-canonical-source-external-id/spec.md`, clarification records embedded in the spec.
Files to touch: `specs/002-canonical-source-external-id/contracts/canonical-vocabulary.yaml`, `specs/002-canonical-source-external-id/research.md`, `specs/002-canonical-source-external-id/data-model.md`.
Expected outputs: A governed grammar source for `standard`, `version`, `source-domain`, `object-id`, and `canonical_id_version = v1`.
Acceptance check: The registry and supporting docs contain the same canonical examples and alias policy, and they explicitly reject destructive normalization.
Parallelizable: No.

- [ ] T003 Update registration and retrieval API contracts in specs/002-canonical-source-external-id/contracts/memory-ingest.openapi.yaml, specs/001-memory-ingest/contracts/memory-ingest.openapi.yaml, specs/002-canonical-source-external-id/quickstart.md
Title: Revise OpenAPI and quickstart contract examples.
Purpose: Make the public contract show canonical URI `external_id`, deterministic UUID v5 `source_id`, and provenance fields before code or tests are changed.
Inputs / dependencies: T002.
Files to touch: `specs/002-canonical-source-external-id/contracts/memory-ingest.openapi.yaml`, `specs/001-memory-ingest/contracts/memory-ingest.openapi.yaml`, `specs/002-canonical-source-external-id/quickstart.md`.
Expected outputs: Updated request/response schemas and examples for canonical/manual ingest, direct-standard ingest, retrieval, replay, and conflict cases.
Acceptance check: OpenAPI examples no longer imply that raw standard payload `id` is persisted as canonical `external_id`.
Parallelizable: No.

- [ ] T004 Write compatibility and migration guidance in specs/002-canonical-source-external-id/quickstart.md, specs/002-canonical-source-external-id/research.md, README.md
Title: Capture compatibility and migration notes.
Purpose: Document rollout sequencing, migration scope, rollback posture, and compatibility expectations before implementation begins.
Inputs / dependencies: T001, T002, T003.
Files to touch: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/research.md`, `README.md`.
Expected outputs: A migration note that names affected stores, deterministic `source_id` rewrite expectations, and operator validation steps.
Acceptance check: The rollout notes explicitly describe how legacy rows move to UUID v5 `source_id` and what must be verified before enabling the new write path.
Parallelizable: No.

---

## Phase 2: Foundational (Blocking Identity Infrastructure)

**Purpose**: Implement the shared domain helpers that all ingest, replay, provenance, and migration behavior depend on.

**Critical**: No user-story implementation should start until this phase is complete.

- [ ] T005 [P] Implement source-domain normalization utilities in crates/mod_memory/src/domain/source_domain.rs, crates/mod_memory/src/domain/mod.rs, tests/unit/normalization_edges.rs
Title: Add source-domain normalization utility.
Purpose: Centralize trusted-domain parsing, host extraction, lowercase normalization, `www.` trimming, port removal, trailing-dot removal, and punycode handling.
Inputs / dependencies: T002, T003.
Files to touch: `crates/mod_memory/src/domain/source_domain.rs`, `crates/mod_memory/src/domain/mod.rs`, `tests/unit/normalization_edges.rs`.
Expected outputs: A reusable domain utility that accepts bare authorities or URLs and rejects ambiguous or untrusted host inputs.
Acceptance check: Unit coverage proves the utility preserves meaningful subdomains and rejects host derivations that depend on path, query, fragment, or userinfo semantics.
Parallelizable: Yes.

- [ ] T006 [P] Implement object-id normalization and encoding utilities in crates/mod_memory/src/domain/object_id.rs, crates/mod_memory/src/domain/mod.rs, tests/unit/normalization_edges.rs
Title: Add object-id normalization and encoding utility.
Purpose: Preserve semantic object identity through outer trim, length checks, and deterministic UTF-8 percent-encoding of reserved and non-unreserved characters.
Inputs / dependencies: T002, T003.
Files to touch: `crates/mod_memory/src/domain/object_id.rs`, `crates/mod_memory/src/domain/mod.rs`, `tests/unit/normalization_edges.rs`.
Expected outputs: A bounded, non-lossy object-id helper that preserves case, spaces, punctuation, and leading zeroes unless explicitly encoded.
Acceptance check: Unit coverage demonstrates raw-length and encoded-length enforcement plus correct percent-encoding for spaces and reserved characters.
Parallelizable: Yes.

- [ ] T007 [P] Implement canonical-id version and deterministic source-id derivation helpers in crates/mod_memory/src/domain/source_identity.rs, crates/mod_memory/src/domain/source.rs, tests/unit/normalized_json_hash.rs
Title: Add canonical-id version and source-id derivation helper.
Purpose: Establish the fixed namespace, canonical source seed, and UUID v5 derivation logic shared by new writes and migration.
Inputs / dependencies: T002, T004.
Files to touch: `crates/mod_memory/src/domain/source_identity.rs`, `crates/mod_memory/src/domain/source.rs`, `tests/unit/normalized_json_hash.rs`.
Expected outputs: A deterministic `source_id` derivation path and an explicit `canonical_id_version` model rooted in `v1`.
Acceptance check: Tests prove stable UUID v5 output for equivalent canonical identities and distinct output for distinct canonical source seeds.
Parallelizable: Yes.

- [ ] T008 Implement SourceExternalId parse/format/validate logic in crates/mod_memory/src/domain/source_external_id.rs, crates/mod_memory/src/domain/source.rs, crates/mod_memory/src/lib.rs
Title: Introduce SourceExternalId value object.
Purpose: Make one domain-owned parser/formatter/validator responsible for canonical URI grammar, vocabulary membership, component rendering, and canonical/manual validation.
Inputs / dependencies: T005, T006, T007.
Files to touch: `crates/mod_memory/src/domain/source_external_id.rs`, `crates/mod_memory/src/domain/source.rs`, `crates/mod_memory/src/lib.rs`.
Expected outputs: A value object that can parse canonical URIs for manual ingest and build canonical URIs from normalized components for direct-standard ingest.
Acceptance check: The value object accepts normative examples, rejects out-of-namespace identifiers, and always re-renders the canonical URI form.
Parallelizable: No.

**Checkpoint**: Canonical identity helpers, parser/validator logic, and deterministic `source_id` derivation are ready for story work.

---

## Phase 3: User Story 1 - Canonical Identity Is Consistent Across Ingest Modes (Priority: P1) 🎯 MVP

**Goal**: Canonical/manual and direct-standard ingest both persist the same governed canonical URI form in `external_id`.

**Independent Test**: Register one canonical/manual request and one direct-standard request, then verify both persist canonical URI `external_id` values under the same grammar while storing raw standard provenance separately.

### Tests for User Story 1

- [ ] T009 [P] [US1] Add canonical identity unit coverage in tests/unit/normalization_edges.rs, tests/unit/normalized_json_hash.rs
Title: Add unit tests for canonical identity rules.
Purpose: Pin the grammar, normalization, and derivation behavior before changing the registration flow.
Inputs / dependencies: T005, T006, T007, T008.
Files to touch: `tests/unit/normalization_edges.rs`, `tests/unit/normalized_json_hash.rs`.
Expected outputs: Failing tests for canonical URI parsing, source-domain normalization, object-id encoding, and deterministic source-id equivalence.
Acceptance check: The test suite contains explicit normative and rejection cases for manual and direct-standard identity inputs.
Parallelizable: Yes.

- [ ] T010 [P] [US1] Revise registration contract tests in tests/contract/register_source_contract.rs, tests/contract/register_source_standard_validation_matrix.rs, tests/contract/openapi_smoke.rs
Title: Revise registration contract coverage.
Purpose: Lock the HTTP surface to canonical URI outputs and direct-standard mapping rules before implementation changes land.
Inputs / dependencies: T003, T008.
Files to touch: `tests/contract/register_source_contract.rs`, `tests/contract/register_source_standard_validation_matrix.rs`, `tests/contract/openapi_smoke.rs`.
Expected outputs: Contract expectations for canonical/manual validation, direct-standard canonical URI responses, and OpenAPI parity.
Acceptance check: Contract tests fail until the registration responses and examples match the canonical grammar and provenance model.
Parallelizable: Yes.

- [ ] T011 [P] [US1] Revise canonical/manual and direct-standard registration flows in tests/integration/register_source_flow.rs, tests/integration/register_source_standard_flow.rs
Title: Revise end-to-end ingest identity flows.
Purpose: Specify the independently testable MVP flow for both ingest modes under the new identity rules.
Inputs / dependencies: T003, T008.
Files to touch: `tests/integration/register_source_flow.rs`, `tests/integration/register_source_standard_flow.rs`.
Expected outputs: Failing integration coverage for canonical/manual success and direct-standard success/rejection behavior.
Acceptance check: Integration tests assert canonical URI `external_id`, consistent `source_id` derivation, and rejection when trusted source-domain derivation fails.
Parallelizable: Yes.

### Implementation for User Story 1

- [ ] T012 [US1] Update direct-standard boundary mapping in crates/app_server/src/handlers/source_register.rs, crates/app_server/src/handlers/mod.rs, crates/app_server/src/router.rs
Title: Change direct-standard mapping at the HTTP boundary.
Purpose: Derive canonical `external_id` components from supported direct-standard payloads before the application layer sees the command.
Inputs / dependencies: T008, T010, T011.
Files to touch: `crates/app_server/src/handlers/source_register.rs`, `crates/app_server/src/handlers/mod.rs`, `crates/app_server/src/router.rs`.
Expected outputs: Boundary logic that resolves trusted source-domain, canonical `standard` and `version`, and raw `object_id` from supported standard payloads.
Acceptance check: Direct-standard requests no longer pass raw payload `id` through as canonical `external_id`, and unsupported or untrusted mappings return the documented 400 path.
Parallelizable: No.

- [ ] T013 [US1] Update register-source command validation in crates/mod_memory/src/application/register_source.rs, crates/app_server/src/state.rs, crates/mod_memory/src/domain/source_external_id.rs
Title: Tighten register-source command validation.
Purpose: Enforce canonical/manual validation rules, direct-standard completeness rules, and ingest-kind-specific command shape checks.
Inputs / dependencies: T008, T012.
Files to touch: `crates/mod_memory/src/application/register_source.rs`, `crates/app_server/src/state.rs`, `crates/mod_memory/src/domain/source_external_id.rs`.
Expected outputs: A command path that accepts only already-canonical manual IDs and fully derived direct-standard canonical identities.
Acceptance check: Validation fails fast before persistence when manual IDs are non-canonical or direct-standard mapping leaves any canonical identity component unresolved.
Parallelizable: No.

- [ ] T014 [US1] Persist original_standard_id and canonical_id_version in crates/mod_memory/src/domain/source.rs, crates/mod_memory/src/infra/surreal_source_repo.rs, crates/mod_memory/src/infra/surreal_source_query.rs, crates/core_infra/src/surrealdb.rs
Title: Store canonical provenance metadata.
Purpose: Move `original_standard_id`, `canonical_id_version`, and ingest-kind metadata into the reserved system envelope without letting them replace canonical `external_id`.
Inputs / dependencies: T012, T013.
Files to touch: `crates/mod_memory/src/domain/source.rs`, `crates/mod_memory/src/infra/surreal_source_repo.rs`, `crates/mod_memory/src/infra/surreal_source_query.rs`, `crates/core_infra/src/surrealdb.rs`.
Expected outputs: Server-managed provenance storage for governed rows, with direct-standard payload `id` preserved separately from canonical identity.
Acceptance check: Persisted rows contain `source_metadata.system.original_standard_id` and `canonical_id_version` while `external_id` remains the canonical URI.
Parallelizable: No.

**Checkpoint**: User Story 1 is complete when both ingest modes produce the same canonical identity contract and direct-standard provenance is persisted separately.

---

## Phase 4: User Story 2 - Replay and Conflict Decisions Follow Canonical Semantics (Priority: P1)

**Goal**: Replay and conflict classification depends on canonical `external_id` plus semantic payload equivalence rather than raw formatting or raw ID spelling.

**Independent Test**: Submit semantically equivalent and semantically different registrations for the same canonical source identity and verify replay or conflict results accordingly.

### Tests for User Story 2

- [ ] T015 [P] [US2] Add semantic hash unit coverage in tests/unit/normalized_json_hash.rs, tests/unit/normalization_edges.rs
Title: Add semantic hash unit tests.
Purpose: Pin how canonical projection hashing differs from raw-body hashing.
Inputs / dependencies: T007, T008, T014.
Files to touch: `tests/unit/normalized_json_hash.rs`, `tests/unit/normalization_edges.rs`.
Expected outputs: Failing tests for formatting-only replay equivalence and semantic conflict detection under one canonical identity.
Acceptance check: Unit tests prove that raw ID spelling or whitespace differences do not change the authoritative semantic hash when canonical identity is unchanged.
Parallelizable: Yes.

- [ ] T016 [P] [US2] Revise replay and conflict contract coverage in tests/contract/register_source_contract.rs, tests/contract/register_source_standard_errors.rs
Title: Revise replay/conflict contract tests.
Purpose: Define the expected 200 replay and 409 conflict semantics for canonical/manual and direct-standard requests.
Inputs / dependencies: T003, T014.
Files to touch: `tests/contract/register_source_contract.rs`, `tests/contract/register_source_standard_errors.rs`.
Expected outputs: Contract assertions for replay classification, conflict status codes, and canonical error bodies.
Acceptance check: Contract tests fail until replay uses canonical identity plus semantic payload rules instead of raw-body equivalence.
Parallelizable: Yes.

- [ ] T017 [P] [US2] Revise replay and conflict integration flows in tests/integration/register_source_replay_hashing.rs, tests/integration/register_source_concurrency.rs
Title: Revise replay/conflict integration flows.
Purpose: Verify end-to-end replay and conflict outcomes against real storage and concurrency conditions.
Inputs / dependencies: T014.
Files to touch: `tests/integration/register_source_replay_hashing.rs`, `tests/integration/register_source_concurrency.rs`.
Expected outputs: Failing integration coverage for formatting-only replay success and semantic conflict rejection under concurrent or repeated submissions.
Acceptance check: Integration flows assert stable identifiers on replay and no duplicate authoritative state after concurrent submissions for the same canonical identity.
Parallelizable: Yes.

### Implementation for User Story 2

- [ ] T018 [US2] Align semantic payload hashing in crates/mod_memory/src/domain/normalization.rs, crates/mod_memory/src/application/register_source.rs, crates/mod_memory/src/domain/source.rs
Title: Replace raw-body replay semantics with semantic hashing.
Purpose: Compute the authoritative replay/conflict comparator from canonical identity plus semantic payload projection, while retaining raw-body hash only for diagnostics.
Inputs / dependencies: T015, T016, T017.
Files to touch: `crates/mod_memory/src/domain/normalization.rs`, `crates/mod_memory/src/application/register_source.rs`, `crates/mod_memory/src/domain/source.rs`.
Expected outputs: A semantic hash pipeline that ignores raw-formatting noise and raw-standard-id spelling differences once canonicalized.
Acceptance check: Equivalent logical sources hash identically after canonicalization, and semantically different payloads produce a conflict path.
Parallelizable: No.

- [ ] T019 [US2] Align replay/conflict persistence in crates/mod_memory/src/infra/surreal_source_repo.rs, crates/mod_memory/src/infra/surreal_memory_repo.rs, crates/core_infra/src/surrealdb.rs
Title: Update repository replay and conflict rules.
Purpose: Make authoritative persistence use canonical `external_id` plus semantic payload hash for create, replay, and conflict decisions.
Inputs / dependencies: T018.
Files to touch: `crates/mod_memory/src/infra/surreal_source_repo.rs`, `crates/mod_memory/src/infra/surreal_memory_repo.rs`, `crates/core_infra/src/surrealdb.rs`.
Expected outputs: Repository logic that replays existing rows for equivalent submissions, preserves first-commit raw body, and rejects semantic divergence as conflict.
Acceptance check: Repository-backed tests prove that duplicate canonical identities do not create duplicate rows when semantic payloads match, and that semantic divergence yields the documented conflict error.
Parallelizable: No.

**Checkpoint**: User Story 2 is complete when replay, idempotency, and conflict results are defined only by canonical identity plus semantic payload equivalence.

---

## Phase 5: User Story 3 - Provenance Remains Auditable (Priority: P2)

**Goal**: Retrieval surfaces canonical provenance fields so operators can distinguish canonical identity from preserved original standard identifiers.

**Independent Test**: Retrieve governed canonical/manual and direct-standard sources and verify `canonical_id_version` plus `original_standard_id` visibility where applicable.

### Tests for User Story 3

- [ ] T020 [P] [US3] Revise provenance contract coverage in tests/contract/get_source_contract.rs, tests/contract/openapi_smoke.rs
Title: Revise provenance retrieval contract tests.
Purpose: Lock the retrieval surface to the new provenance envelope and OpenAPI examples.
Inputs / dependencies: T003, T014.
Files to touch: `tests/contract/get_source_contract.rs`, `tests/contract/openapi_smoke.rs`.
Expected outputs: Contract expectations for `source_metadata.system.canonical_id_version`, `ingest_kind`, and `original_standard_id` when present.
Acceptance check: Contract tests fail until retrieval and OpenAPI examples expose provenance fields without collapsing them into `external_id`.
Parallelizable: Yes.

- [ ] T021 [P] [US3] Revise provenance retrieval flows in tests/integration/get_source_flow.rs, tests/integration/memory_ingest_vertical_slice.rs
Title: Revise provenance integration flows.
Purpose: Verify end-to-end retrieval behavior for governed manual and direct-standard rows.
Inputs / dependencies: T014.
Files to touch: `tests/integration/get_source_flow.rs`, `tests/integration/memory_ingest_vertical_slice.rs`.
Expected outputs: Failing integration coverage for provenance visibility and canonical identity precedence in retrieval.
Acceptance check: Integration flows assert that canonical `external_id` remains primary while direct-standard `original_standard_id` appears only as secondary provenance metadata.
Parallelizable: Yes.

### Implementation for User Story 3

- [ ] T022 [US3] Expose provenance fields in crates/app_server/src/handlers/source_get.rs, crates/mod_memory/src/application/get_source.rs, crates/mod_memory/src/infra/surreal_source_query.rs
Title: Surface provenance in source retrieval.
Purpose: Return `canonical_id_version`, `ingest_kind`, and `original_standard_id` from authoritative storage through the application and HTTP layers.
Inputs / dependencies: T020, T021.
Files to touch: `crates/app_server/src/handlers/source_get.rs`, `crates/mod_memory/src/application/get_source.rs`, `crates/mod_memory/src/infra/surreal_source_query.rs`.
Expected outputs: Retrieval responses and query models that distinguish canonical identity from provenance metadata.
Acceptance check: `GET /sources/{source-id}` returns the reserved provenance envelope with canonical identity still represented only by `external_id`.
Parallelizable: No.

**Checkpoint**: User Story 3 is complete when provenance is auditable in retrieval without changing identity precedence.

---

## Phase 6: User Story 4 - Existing Records Are Migrated To Deterministic Source IDs (Priority: P2)

**Goal**: All persisted source rows and dependent references move to deterministic UUID v5 `source_id` values under one controlled migration.

**Independent Test**: Run migration coverage against pre-feature rows and confirm retrieval, replay, indexing, and reference integrity continue to work under the rewritten `source_id` values.

### Tests for User Story 4

- [ ] T023 [P] [US4] Add deterministic source_id adapter coverage in tests/contract/surreal_source_store_contract.rs, tests/contract/indexing_outbox_mapping_contract.rs, tests/unit/normalized_json_hash.rs
Title: Add deterministic source-id contract and unit coverage.
Purpose: Pin the storage and derivation guarantees required for UUID v5 adoption.
Inputs / dependencies: T007, T014, T019.
Files to touch: `tests/contract/surreal_source_store_contract.rs`, `tests/contract/indexing_outbox_mapping_contract.rs`, `tests/unit/normalized_json_hash.rs`.
Expected outputs: Failing tests for deterministic source-id persistence, outbox mapping stability, and identity-seed parity.
Acceptance check: Contract and unit tests prove that the same canonical source seed always yields the same stored `source_id` and downstream mapping keys.
Parallelizable: Yes.

- [ ] T024 [P] [US4] Revise migration and consistency integration flows in tests/integration/multi_instance_consistency.rs, tests/integration/indexing_status_mapping_flow.rs, tests/integration/memory_ingest_vertical_slice.rs
Title: Revise migration and consistency integration flows.
Purpose: Specify the end-to-end behavior of migration, replay, and projection consistency after `source_id` rewrites.
Inputs / dependencies: T019.
Files to touch: `tests/integration/multi_instance_consistency.rs`, `tests/integration/indexing_status_mapping_flow.rs`, `tests/integration/memory_ingest_vertical_slice.rs`.
Expected outputs: Failing integration coverage for migration rewrite success, post-migration retrieval, and projection/reference integrity.
Acceptance check: Integration flows assert that migrated rows remain queryable and that dependent references resolve to the rewritten deterministic `source_id` values.
Parallelizable: Yes.

### Implementation for User Story 4

- [ ] T025 [US4] Derive deterministic source_id for new writes in crates/mod_memory/src/application/register_source.rs, crates/mod_memory/src/domain/source_identity.rs, crates/mod_memory/src/infra/surreal_source_repo.rs
Title: Replace random source-id allocation on the write path.
Purpose: Ensure every governed new write uses the UUID v5 source-id derivation contract before persistence.
Inputs / dependencies: T023.
Files to touch: `crates/mod_memory/src/application/register_source.rs`, `crates/mod_memory/src/domain/source_identity.rs`, `crates/mod_memory/src/infra/surreal_source_repo.rs`.
Expected outputs: Write-path logic that derives `source_id` solely from the canonical source seed instead of allocating random UUIDs.
Acceptance check: New registrations for the same canonical source identity always return the same `source_id`, and new writes never create UUID v4 source IDs.
Parallelizable: No.

- [ ] T026 [US4] Implement source_id migration and reference rewrite in crates/core_infra/src/surrealdb.rs, crates/mod_memory/src/infra/surreal_source_repo.rs, crates/mod_memory/src/infra/surreal_memory_repo.rs, crates/mod_memory/src/infra/meili_indexer.rs
Title: Migrate stored source-id references to UUID v5.
Purpose: Rewrite persisted `source_id` values and every dependent authoritative or projection reference under one governed migration flow.
Inputs / dependencies: T024, T025.
Files to touch: `crates/core_infra/src/surrealdb.rs`, `crates/mod_memory/src/infra/surreal_source_repo.rs`, `crates/mod_memory/src/infra/surreal_memory_repo.rs`, `crates/mod_memory/src/infra/meili_indexer.rs`.
Expected outputs: Migration logic and verification steps that move legacy rows to deterministic source IDs without mixed v4/v5 persistent state.
Acceptance check: Migration coverage proves all affected references are rewritten consistently, and rollout fails closed if the rewrite cannot complete safely.
Parallelizable: No.

**Checkpoint**: User Story 4 is complete when new writes and migrated rows both use the same deterministic UUID v5 `source_id` regime.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Close cross-artifact drift and finalize operator-facing rollout guidance.

- [ ] T027 [P] Update rollout and rollback documentation in specs/002-canonical-source-external-id/quickstart.md, README.md, specs/002-canonical-source-external-id/adr/
Title: Finalize rollout and rollback runbook.
Purpose: Reflect the implemented migration, verification, and rollback posture in operator-facing docs and ADR notes.
Inputs / dependencies: T004, T026.
Files to touch: `specs/002-canonical-source-external-id/quickstart.md`, `README.md`, `specs/002-canonical-source-external-id/adr/`.
Expected outputs: Final runbook steps for rollout, rollback, verification, and post-migration checks.
Acceptance check: An operator can execute the rollout from docs alone without guessing how to verify UUID v5 migration completeness.
Parallelizable: Yes.

- [ ] T028 [P] Sweep fixture and contract parity in specs/002-canonical-source-external-id/research.md, specs/002-canonical-source-external-id/data-model.md, specs/002-canonical-source-external-id/contracts/canonical-vocabulary.yaml, tests/fixtures/register_source/
Title: Perform final artifact parity sweep.
Purpose: Ensure spec, data model, vocabulary, and fixtures all use the same canonical examples and provenance semantics after implementation.
Inputs / dependencies: T026.
Files to touch: `specs/002-canonical-source-external-id/research.md`, `specs/002-canonical-source-external-id/data-model.md`, `specs/002-canonical-source-external-id/contracts/canonical-vocabulary.yaml`, `tests/fixtures/register_source/`.
Expected outputs: Canonical examples, fixtures, and artifact text that match the shipped implementation and contract semantics.
Acceptance check: No published fixture or planning artifact still shows raw standard payload `id` as canonical `external_id` or implies UUID v4 `source_id` behavior.
Parallelizable: Yes.

---

## Dependencies & Execution Order

### Phase Dependencies

- Phase 1: No dependencies. Complete all documentation and contract alignment before code changes.
- Phase 2: Depends on Phase 1. Blocks all user stories.
- Phase 3 (US1): Depends on Phase 2. Establishes the MVP write path.
- Phase 4 (US2): Depends on Phase 3 because replay/conflict rules use the canonical identity model and provenance fields persisted there.
- Phase 5 (US3): Depends on Phase 3 because retrieval needs stored provenance fields.
- Phase 6 (US4): Depends on Phases 3 and 4 because migration must adopt the final canonical identity and replay model.
- Phase 7: Depends on all selected user stories.

### User Story Dependencies

- US1 (P1): Starts after Phase 2 and is the MVP.
- US2 (P1): Starts after US1 persists canonical identity and provenance.
- US3 (P2): Starts after US1 persists provenance fields.
- US4 (P2): Starts after US1 and US2 stabilize the final identity and replay semantics.

### Within Each User Story

- Test tasks should be written first and should fail before implementation.
- Boundary and command-shape changes precede repository changes.
- Repository and migration tasks follow domain and application-level identity rules.
- Retrieval exposure follows persistence of provenance metadata.

## Parallel Opportunities

- Phase 2 foundational helpers can be parallelized as T005, T006, and T007.
- US1 test preparation can be parallelized as T009, T010, and T011.
- US2 test preparation can be parallelized as T015, T016, and T017.
- US3 test preparation can be parallelized as T020 and T021.
- US4 test preparation can be parallelized as T023 and T024.
- Polish work can be parallelized as T027 and T028 after implementation stabilizes.

## Parallel Example: User Story 1

```text
Parallel group A
- T009 canonical identity unit coverage
- T010 registration contract coverage
- T011 ingest identity integration flows
```

## Parallel Example: User Story 2

```text
Parallel group B
- T015 semantic hash unit coverage
- T016 replay/conflict contract coverage
- T017 replay/conflict integration flows
```

## Parallel Example: User Story 4

```text
Parallel group C
- T023 deterministic source_id contract and unit coverage
- T024 migration and consistency integration flows
```

## Implementation Strategy

### MVP First

1. Complete Phase 1 and Phase 2.
2. Complete User Story 1.
3. Stop and validate canonical/manual plus direct-standard ingest identity behavior before moving to replay or migration work.

### Incremental Delivery

1. Deliver US1 to stabilize one canonical ingest contract.
2. Deliver US2 to lock replay and conflict semantics.
3. Deliver US3 to expose auditable provenance.
4. Deliver US4 to complete deterministic source-id migration.
5. Finish Phase 7 for rollout and artifact parity.

## Risks / Sequencing Notes

- Migration work should not start until the canonical source seed and replay semantics are final, otherwise rewritten rows may not match the post-feature write path.
- Contract examples and fixture parity are high-risk drift points because this feature changes identifier semantics across manual and standard ingest modes.
- If new modules are introduced for normalization utilities, keep exports and ownership narrow so handler, application, and repository layers continue to respect the existing boundary separation.