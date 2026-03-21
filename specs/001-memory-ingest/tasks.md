# Tasks: Schema-Native Standard Credential Registry

**Input**: Design documents from `/specs/001-memory-ingest/`  
**Status**: IMPLEMENT-READY  
**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/memory-ingest.openapi.yaml`, `quickstart.md`

## Purpose

Break the schema-native redesign into executable RED -> GREEN -> REFACTOR -> VERIFY work, with story-level traceability back to the refreshed specification and plan.

## TDD Execution Rules

- `RED`: add or tighten the failing proof for the next requirement or risk.
- `GREEN`: implement the smallest behavior needed to satisfy that proof.
- `REFACTOR`: improve boundaries or structure without changing proven behavior.
- `VERIFY`: run the story gate with the repository-standard commands and slow checks where required.

## Dependency Notes

- Phase 1 aligns shared identifiers, routes, and test fixtures with the breaking public contract.
- Phase 2 is blocking for all stories because the old `Source` / `MemoryItem` model must be removed from shared runtime paths.
- User Story 1 is the MVP slice and must complete before retrieval or search validation.
- User Story 2 depends on User Story 1 persistence.
- User Story 3 depends on the outbox and projection state introduced in User Story 1.
- User Story 4 depends on the shared readiness and search degradation wiring from Phase 2 and User Story 3.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Prepare the codebase for the breaking schema-native redesign.

- [ ] T001 Update route inventory and path helpers for `/credentials/*` in `crates/app_server/src/router.rs`, `crates/app_server/src/handlers/mod.rs`, and `crates/app_server/tests/memory_ingest_smoke.rs`
- [ ] T002 [P] Remove wrapper-era API fixture assumptions from `specs/001-memory-ingest/contracts/memory-ingest.openapi.yaml` snapshots and `crates/app_server/tests/memory_ingest_smoke.rs`
- [ ] T003 [P] Refresh shared naming in `crates/mod_memory/src/lib.rs`, `crates/mod_memory/src/application/mod.rs`, and `crates/mod_memory/src/domain/mod.rs` so the exported surface is credential-first rather than source-first

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Replace shared domain and persistence concepts that block every story.

**Critical**: No user-story endpoint work should begin until this phase is complete.

- [ ] T004 Define schema-native domain identifiers and aggregate types in `crates/mod_memory/src/domain/credential.rs`, `crates/mod_memory/src/domain/mod.rs`, and `crates/core_shared/src/lib.rs`
- [ ] T005 [P] Replace wrapper-era repository interfaces with credential-first ports in `crates/mod_memory/src/infra/repo.rs`, `crates/mod_memory/src/infra/mod.rs`, and `crates/mod_memory/src/bootstrap.rs`
- [ ] T006 [P] Replace SurrealDB authoritative record shapes and uniqueness bootstrap in `crates/core_infra/src/surrealdb.rs` and `crates/core_infra/src/lib.rs`
- [ ] T007 [P] Realign shared error mapping for credential-first routes in `crates/core_shared/src/error.rs`, `crates/app_server/src/middleware.rs`, and `crates/app_server/src/handlers/health.rs`
- [ ] T008 Remove wrapper-era route wiring and legacy retrieval handlers from `crates/app_server/src/handlers/source_register.rs`, `crates/app_server/src/handlers/source_get.rs`, `crates/app_server/src/handlers/memory_item_get.rs`, and `crates/app_server/src/router.rs`

**Checkpoint**: Foundation complete. Story work can now proceed against the schema-native model.

## Phase 3: User Story 1 - Register a standard credential (Priority: P1) 🎯 MVP

**Goal**: Accept supported Open Badges and CLR credentials, enforce schema-native validation, and persist authoritative schema-exact credential records.

**Independent Test**: Call `POST /credentials/register` with valid and invalid Open Badges and CLR payloads, then verify authoritative persistence and response shapes.

### RED: User Story 1

- [ ] T009 [P] [US1] Add OpenAPI-backed contract validation for `POST /credentials/register` in `crates/app_server/tests/register_credential_contract.rs` and `crates/app_server/tests/fixtures/register_credential/*.json`
- [ ] T010 [P] [US1] Add integration coverage for Open Badges and CLR create, replay, and conflict flows in `crates/app_server/tests/memory_ingest_smoke.rs`
- [ ] T011 [P] [US1] Add storage-adapter contract coverage for schema-exact persistence and uniqueness in `crates/core_infra/tests/surreal_credential_store_contract.rs`

### GREEN: User Story 1

- [ ] T012 [P] [US1] Implement credential family classification and schema-exact filtering in `crates/mod_memory/src/bootstrap.rs` and `crates/mod_memory/src/domain/credential.rs`
- [ ] T013 [US1] Implement authoritative create-or-replay-or-conflict credential writes in `crates/core_infra/src/surrealdb.rs`
- [ ] T014 [US1] Implement `RegisterCredentialService` in `crates/mod_memory/src/bootstrap.rs`
- [ ] T015 [US1] Implement the registration handler and request mapping in `crates/app_server/src/handlers/credential_register.rs`, `crates/app_server/src/handlers/mod.rs`, and `crates/app_server/src/router.rs`

### REFACTOR: User Story 1

- [ ] T016 [US1] Extract shared schema-native mapping helpers in `crates/mod_memory/src/bootstrap.rs` and `crates/mod_memory/src/domain/credential.rs`

### VERIFY: User Story 1

- [ ] T017 [US1] Run the US1 verification gate with `cargo nextest run -p app_server --test memory_ingest_smoke`, `cargo nextest run -p app_server --test register_credential_contract`, `cargo mutants -p mod_memory --test-tool nextest`, and `cargo llvm-cov nextest -p app_server -p mod_memory --lcov --output-path target/llvm-cov/us1.info`

## Phase 4: User Story 2 - Retrieve authoritative credential documents (Priority: P1)

**Goal**: Retrieve authoritative credentials by official `id` without wrapper-era response fields.

**Independent Test**: Register a credential, then call `GET /credentials/{credential-id}` and verify the response body equals the authoritative stored credential document.

### RED: User Story 2

- [ ] T018 [P] [US2] Add OpenAPI-backed contract validation for `GET /credentials/{credential-id}` in `crates/app_server/tests/get_credential_contract.rs`
- [ ] T019 [P] [US2] Add integration coverage for successful and missing credential retrieval in `crates/app_server/tests/memory_ingest_smoke.rs`

### GREEN: User Story 2

- [ ] T020 [P] [US2] Implement authoritative credential query ports in `crates/mod_memory/src/infra/repo.rs` and `crates/core_infra/src/surrealdb.rs`
- [ ] T021 [US2] Implement `GetCredentialService` in `crates/mod_memory/src/bootstrap.rs`
- [ ] T022 [US2] Implement the retrieval handler in `crates/app_server/src/handlers/credential_get.rs`, `crates/app_server/src/handlers/mod.rs`, and `crates/app_server/src/router.rs`

### REFACTOR: User Story 2

- [ ] T023 [US2] Remove leftover source or memory-item response mappers from `crates/mod_memory/src/bootstrap.rs` and `crates/app_server/src/handlers/`

### VERIFY: User Story 2

- [ ] T024 [US2] Run the US2 verification gate with `cargo nextest run -p app_server --test memory_ingest_smoke` and `cargo nextest run -p app_server --test get_credential_contract`

## Phase 5: User Story 3 - Search credential projections (Priority: P2)

**Goal**: Expose non-authoritative credential search while keeping authoritative writes and reads independent from search health.

**Independent Test**: Query `GET /credentials/search` for indexed fixtures and verify projection hits, then simulate degraded search and confirm authoritative registration and retrieval still succeed.

### RED: User Story 3

- [ ] T025 [P] [US3] Add OpenAPI-backed contract validation for `GET /credentials/search` in `crates/app_server/tests/search_credentials_contract.rs`
- [ ] T026 [P] [US3] Add integration coverage for projection hits and degraded-search behavior in `crates/app_server/tests/memory_ingest_smoke.rs`
- [ ] T027 [P] [US3] Add outbox-to-projection contract coverage in `crates/core_infra/tests/credential_projection_contract.rs`

### GREEN: User Story 3

- [ ] T028 [P] [US3] Implement projection document builders in `crates/mod_memory/src/domain/credential.rs` and `crates/core_infra/src/meilisearch.rs`
- [ ] T029 [US3] Implement durable outbox persistence and projection rehydration in `crates/core_infra/src/surrealdb.rs` and `crates/mod_memory/src/bootstrap.rs`
- [ ] T030 [US3] Implement `SearchCredentialsService` and the search handler in `crates/mod_memory/src/bootstrap.rs`, `crates/app_server/src/handlers/credential_search.rs`, and `crates/app_server/src/router.rs`

### REFACTOR: User Story 3

- [ ] T031 [US3] Extract shared projection mapping helpers in `crates/mod_memory/src/bootstrap.rs` and `crates/core_infra/src/meilisearch.rs`

### VERIFY: User Story 3

- [ ] T032 [US3] Run the US3 verification gate with `cargo nextest run -p app_server --test search_credentials_contract` and `cargo nextest run -p app_server --test memory_ingest_smoke`

## Phase 6: User Story 4 - Operational probes stay aligned (Priority: P2)

**Goal**: Preserve local liveness and dependency-aware readiness semantics after the credential-first redesign.

**Independent Test**: Call `/health` and `/ready` with healthy and degraded dependencies and verify the documented status matrix.

### RED: User Story 4

- [ ] T033 [P] [US4] Add OpenAPI-backed contract validation for `/health` and `/ready` in `crates/app_server/tests/health_readiness_contract.rs`

### GREEN: User Story 4

- [ ] T034 [US4] Update readiness and liveness payload mapping in `crates/app_server/src/handlers/health.rs`, `crates/app_server/src/state.rs`, and `crates/core_infra/src/setup.rs`

### REFACTOR: User Story 4

- [ ] T035 [US4] Remove wrapper-era wording from probe diagnostics in `crates/app_server/src/handlers/health.rs` and `crates/core_infra/src/setup.rs`

### VERIFY: User Story 4

- [ ] T036 [US4] Run the US4 verification gate with `cargo nextest run -p app_server --test health_readiness_contract`

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Close the redesign with consistency, documentation, and broader verification.

- [ ] T037 [P] Remove wrapper-era dead code and exports in `crates/mod_memory/src/`, `crates/core_shared/src/`, and `crates/app_server/src/handlers/`
- [ ] T038 [P] Refresh smoke fixtures and quickstart-aligned examples in `crates/app_server/tests/memory_ingest_smoke.rs` and `specs/001-memory-ingest/quickstart.md`
- [ ] T039 Run the full redesign verification gate with `cargo nextest run --workspace`, `cargo mutants -p mod_memory --test-tool nextest`, and `cargo llvm-cov nextest --workspace --lcov --output-path target/llvm-cov/001-memory-ingest.info`

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup and blocks all user stories.
- **User Story 1 (Phase 3)**: Depends on Phase 2 and delivers the MVP.
- **User Story 2 (Phase 4)**: Depends on User Story 1 persistence.
- **User Story 3 (Phase 5)**: Depends on User Story 1 persistence and Phase 2 search wiring.
- **User Story 4 (Phase 6)**: Depends on Phase 2 and can complete after shared runtime changes land.
- **Polish (Phase 7)**: Depends on all desired stories.

### Parallel Opportunities

- Setup tasks marked `[P]` can run in parallel.
- Foundational tasks T005, T006, and T007 can run in parallel after T004.
- RED tasks within each user story can run in parallel.
- Search projection work can proceed in parallel with probe work after User Story 1 is stable.
