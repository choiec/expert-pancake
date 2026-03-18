# Tasks: Canonical Source External ID and Direct-Standard Ingest Alignment

**Input**: Design documents from `/specs/002-canonical-source-external-id/`
**Status**: Canonical 002 runtime and active 002 artifact set are implemented and aligned as of 2026-03-18.
**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `quickstart.md`, `contracts/canonical-vocabulary.yaml`, `contracts/memory-ingest.openapi.yaml`
**Tests**: Required. This status file keeps only surviving canonical-002 work and deletes transition-only backlog.

## Task Generation Assumptions

- The repository is still pre-production, so transition-only preservation work stays out of scope.
- Tasks are checked only when verified against the current workspace code, tests, contracts, fixtures, docs, and validation commands.
- Contract, governance, and validation artifacts are considered complete only when they match the canonical-only runtime and public surface.

## Phase 1: Setup

**Purpose**: Lock the canonical 002 scope in the feature artifacts.

- [x] T001 Record the canonical-only Option A scope in /workspaces/rust/specs/002-canonical-source-external-id/spec.md, /workspaces/rust/specs/002-canonical-source-external-id/plan.md, /workspaces/rust/specs/002-canonical-source-external-id/research.md, /workspaces/rust/specs/002-canonical-source-external-id/data-model.md, and /workspaces/rust/specs/002-canonical-source-external-id/quickstart.md

---

## Phase 2: Foundational

**Purpose**: Keep the canonical identity primitives in place for every ingest path.

- [x] T002 Implement canonical URI validation, normalization, and trusted direct-standard derivation in /workspaces/rust/crates/mod_memory/src/domain/source_external_id.rs and /workspaces/rust/crates/app_server/src/handlers/source_register.rs

**Checkpoint**: Canonical identity rules are established for all downstream behavior.

---

## Phase 3: User Story 1 - Canonical identity is consistent across ingest modes (Priority: P1)

**Goal**: Manual and direct-standard ingest converge on one project-owned canonical external identity.

**Independent Test**: Register a canonical/manual payload and a supported direct-standard payload, then confirm both persist canonical `external_id` values and deterministic `source_id` values under the 002 namespace.

- [x] T003 [US1] Enforce project-owned canonical URI acceptance and deterministic UUID v5 `source_id` generation in /workspaces/rust/crates/mod_memory/src/domain/source_identity.rs and /workspaces/rust/crates/mod_memory/src/application/register_source.rs
- [x] T004 [P] [US1] Align canonical/manual and direct-standard contract coverage in /workspaces/rust/tests/contract/register_source_contract.rs, /workspaces/rust/tests/contract/register_source_standard_validation_matrix.rs, and /workspaces/rust/tests/integration/register_source_standard_flow.rs

**Checkpoint**: User Story 1 is implemented and independently testable.

---

## Phase 4: User Story 2 - Replay and conflict follow semantic identity (Priority: P2)

**Goal**: Replay and conflict depend only on canonical identity plus `semantic_payload_hash`.

**Independent Test**: Re-submit semantically equivalent payloads for the same canonical identity and confirm replay returns the existing authoritative row; submit a semantic change and confirm conflict is returned.

- [x] T005 [US2] Enforce `semantic_payload_hash` as the authoritative replay and conflict comparator in /workspaces/rust/crates/mod_memory/src/infra/surreal_source_repo.rs and /workspaces/rust/crates/mod_memory/src/infra/surreal_memory_repo.rs
- [x] T006 [P] [US2] Keep canonical-only runtime and repository behavior without alternate-identifier branches in /workspaces/rust/crates/mod_memory/src/application/register_source.rs and /workspaces/rust/crates/app_server/src/handlers/source_register.rs
- [x] T007 [P] [US2] Verify replay and conflict behavior in /workspaces/rust/tests/integration/register_source_flow.rs, /workspaces/rust/tests/integration/register_source_replay_hashing.rs, and /workspaces/rust/tests/contract/surreal_source_store_contract.rs

**Checkpoint**: User Story 2 is implemented and independently testable.

---

## Phase 5: User Story 3 - Provenance remains auditable (Priority: P3)

**Goal**: Public responses and docs expose canonical provenance cleanly without superseded transition-era leakage.

**Independent Test**: Compare registration and retrieval responses for canonical/manual and direct-standard rows, then confirm docs and fixtures describe only the canonical 002 public model.

- [x] T008 [US3] Preserve public provenance parity in /workspaces/rust/crates/app_server/src/handlers/source_register.rs, /workspaces/rust/crates/app_server/src/handlers/source_get.rs, /workspaces/rust/tests/contract/register_source_contract.rs, and /workspaces/rust/tests/contract/get_source_contract.rs
- [x] T009 [US3] Remove superseded transition wording from the public contract in /workspaces/rust/specs/002-canonical-source-external-id/contracts/memory-ingest.openapi.yaml
- [x] T010 [P] [US3] Keep canonical-only examples and operator guidance aligned in /workspaces/rust/README.md, /workspaces/rust/specs/002-canonical-source-external-id/quickstart.md, /workspaces/rust/tests/fixtures/register_source/canonical_success.json, and /workspaces/rust/tests/fixtures/register_source/standards/open_badges_valid.json

**Checkpoint**: User Story 3 is complete once public contracts and operator docs match the already-implemented canonical runtime.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final validation and repository closeout for the canonical-only model.

- [x] T011 [P] Refresh canonical vocabulary governance in /workspaces/rust/specs/002-canonical-source-external-id/contracts/canonical-vocabulary.yaml
- [x] T012 Run final validation for /workspaces/rust/Cargo.toml and /workspaces/rust/benches/memory_ingest_latency.rs with `cargo test --tests` and `cargo bench --bench memory_ingest_latency --no-run`

---

## Dependencies & Execution Order

### Phase Dependencies

- Setup (Phase 1) establishes the approved canonical-only scope.
- Foundational (Phase 2) must remain intact before any story-level work changes.
- User Story 1 depends on Foundational.
- User Story 2 depends on User Story 1 identity primitives.
- User Story 3 depends on the public response shape established by User Stories 1 and 2.
- Polish depends on the desired user stories being complete.

### User Story Dependencies

- User Story 1 is the MVP and is already complete.
- User Story 2 builds on the canonical identity model and is already complete.
- User Story 3 is complete, including public contract and governance alignment.

### Parallel Opportunities

- T004 can run independently from T003 once the canonical identity files are stable.
- T006 and T007 can run in parallel after T005 because they touch different verification layers.
- T010 can run independently from T008 once the response shape is stable.
- T011 and T012 are independent closeout tasks.

---

## Parallel Example: User Story 2

```bash
# After the replay/comparator rule is stable, these can proceed together:
T006 /workspaces/rust/crates/mod_memory/src/application/register_source.rs + /workspaces/rust/crates/app_server/src/handlers/source_register.rs
T007 /workspaces/rust/tests/integration/register_source_flow.rs + /workspaces/rust/tests/integration/register_source_replay_hashing.rs + /workspaces/rust/tests/contract/surreal_source_store_contract.rs
```

---

## Implementation Strategy

### MVP First

1. Phase 1 and Phase 2 established the canonical-only runtime model.
2. User Story 1 delivered the MVP and is complete.
3. User Stories 2 and 3 completed the semantic replay and provenance surface.
4. Closeout completes when active contracts, governance docs, and validation artifacts match runtime.

### Incremental Delivery

1. Keep the current canonical-only runtime as the implementation baseline.
2. Keep public contracts and governance docs aligned with the already-shipped behavior.
3. Re-run T012 validation after artifact cleanup.

### Suggested Remaining Scope

1. None.

---

## Notes

- Total tasks: 12
- Completed tasks: 12
- Remaining tasks: 0
- No transition-only backlog is retained in this file.