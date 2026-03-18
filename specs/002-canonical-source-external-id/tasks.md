# Tasks: Canonical Source External ID and Direct-Standard Ingest Alignment

**Input**: Design documents from `/specs/002-canonical-source-external-id/`
**Status**: Regenerated from closed plan decisions on 2026-03-18
**Prerequisites**: `spec.md`, `plan.md`, `research.md`, `data-model.md`, `quickstart.md`, `contracts/canonical-vocabulary.yaml`, `contracts/memory-ingest.openapi.yaml`, `checklists/requirements.md`
**Tests**: Required. This feature changes canonical identity, replay and conflict semantics, deterministic `source_id`, migration safety, observability, and rollout verification gates.

## 1. Regenerated Task Structure

### Workstream Overview

1. **Artifact and contract alignment**
   - Close drift across spec, plan, data model, research, quickstart, OpenAPI, vocabulary, and review checklist artifacts before code changes land.
2. **Domain and identity implementation**
   - Establish domain-owned canonical identity, normalization, deterministic `source_id`, provenance persistence, and fail-fast validation.
3. **Handler, application, repository behavior**
   - Replace legacy raw-payload identifier semantics with canonical URI behavior and provenance-parity responses.
4. **Mixed-population safety**
   - Lock legacy-only, canonical-only, coexistence, remap, deny-write, and duplicate-candidate rules to the migration window only.
5. **Migration execution and verification**
   - Implement row classification, dry-run JSON output, rewrite execution, dependent-reference rewrite, stop conditions, and verification queries.
6. **Observability and diagnostics**
   - Wire structured logs, traces, bounded metrics, and the decision taxonomy for canonicalization, replay, migration, and remap decisions.
7. **Regression-hardening tests**
   - Freeze normalization, canonical URI, replay or conflict, migration, and observability regressions with explicit matrices and fixtures.
8. **Rollout runbook and operator readiness**
   - Produce operator pass or fail guidance for pre-migration gates, dry-run interpretation, verification, rollback, and post-cutover validation.

### Execution Order

1. Workstream A blocks all implementation because it closes contract and artifact drift.
2. Workstream B blocks all runtime behavior changes because every handler, repository, migration, and test path depends on one domain-owned identity model.
3. Workstream C depends on Workstream B and delivers user-visible canonical identity, replay, and provenance behavior.
4. Workstream D depends on Workstreams B and C because mixed-population safety reuses the canonical runtime semantics.
5. Workstream E depends on Workstreams B, C, and D because migration execution must target the final identity and mixed-population rules.
6. Workstream F runs in parallel with late Workstreams C through E once decision outputs are stable, but must complete before rollout sign-off.
7. Workstream G starts after the corresponding behavior exists and must complete before rollout sign-off.
8. Workstream H starts after Workstream A, runs in parallel with implementation, and must be finalized after Workstreams E through G complete.

### Parallel Opportunities

- Workstream A: `T001` through `T008` can be split across reviewers once terminology is fixed.
- Workstream B: `T009`, `T010`, `T011`, `T012`, and `T017` are parallel once file ownership is assigned.
- Workstream C: `T020`, `T021`, and `T022` can run in parallel after `T018` and `T019` land.
- Workstream D: `T027` through `T031` can run in parallel after remap and write-freeze primitives exist.
- Workstream E: `T033`, `T034`, `T035`, and `T036` can run in parallel after classification primitives exist.
- Workstream F: `T039`, `T040`, `T041`, and `T042` can run in parallel before taxonomy consolidation in `T043`.
- Workstream G: `T045` through `T050` are intentionally separable by matrix.
- Workstream H: `T051` through `T055` can be split between operator-doc and verification-doc owners.

## 2. Tasks

### Workstream A. Artifact and Contract Alignment

- [ ] T001 Align canonical identity and replay terminology in specs/002-canonical-source-external-id/spec.md
  Title: Align spec.md with authoritative canonical identity decisions.
  Purpose: Close spec drift so the reviewer sees one definition for canonical `external_id`, provenance, deterministic `source_id`, replay or conflict, mixed-population posture, and observability boundaries.
  Files/Areas: `specs/002-canonical-source-external-id/spec.md`.
  Acceptance Criteria: Goals, requirements, edge cases, acceptance criteria, and examples state that `external_id` is always the project-owned canonical URI; `original_standard_id` is provenance-only; `canonical_id_version = v1` is mandatory; `semantic_payload_hash` is authoritative; `raw_body_hash` is diagnostics-only; mixed legacy or new coexistence exists only during the migration window; reviewer can diff the spec against the plan without contradictory vocabulary.
  Dependency: None.
  Risk Closed: Spec drift, contract drift, reviewer ambiguity.

- [ ] T002 Align implementation invariants and rollout gates in specs/002-canonical-source-external-id/plan.md
  Title: Align plan.md with the closed execution model.
  Purpose: Ensure the plan remains the implementation source for deterministic seed governance, replay semantics, migration stop conditions, observability fields, and rollout verification.
  Files/Areas: `specs/002-canonical-source-external-id/plan.md`.
  Acceptance Criteria: The plan uses the final seed contract `source|{canonical_id_version}|{canonical_external_id}`; replay and conflict rules match the closed decision table; mixed-population write denial and remap reads are explicit; verification queries and rollback posture are unchanged from the final design; any task references added point to the regenerated backlog without reopening design questions.
  Dependency: T001.
  Risk Closed: Plan drift, implementation-order ambiguity, rollout-gate drift.

- [ ] T003 Align entity, provenance, migration, and diagnostics fields in specs/002-canonical-source-external-id/data-model.md
  Title: Align data-model.md with authoritative storage and diagnostics semantics.
  Purpose: Freeze the stored field set and invariants that runtime code, migration code, and tests must share.
  Files/Areas: `specs/002-canonical-source-external-id/data-model.md`.
  Acceptance Criteria: The model requires `canonical_id_version` on governed rows; keeps `original_standard_id` under `source_metadata.system`; defines `semantic_payload_hash` as the only authoritative comparator; marks `raw_body_hash` as internal only; defines remap lookup state and per-row migration report fields exactly as the plan requires; reviewer can trace every persisted field to one runtime owner.
  Dependency: T001, T002.
  Risk Closed: Data-model drift, migration-report drift, provenance drift.

- [ ] T004 Align closed rationale and reject criteria in specs/002-canonical-source-external-id/research.md
  Title: Align research.md with the already-closed design decisions.
  Purpose: Record which drift each final decision closes so later reviewers do not reopen settled design choices.
  Files/Areas: `specs/002-canonical-source-external-id/research.md`.
  Acceptance Criteria: Research notes explicitly preserve the final decisions for canonical URI persistence, deterministic seed contract, authoritative hash policy, mixed-population write freeze, classification triad, stop conditions, observability scope, and full-snapshot rollback; no alternate semantics remain documented as viable.
  Dependency: T001, T002, T003.
  Risk Closed: Design-reopening risk, rationale drift, reviewer misread of why behavior changed.

- [ ] T005 Align rollout instructions and operator verification flow in specs/002-canonical-source-external-id/quickstart.md
  Title: Align quickstart.md with rollout-critical execution and verification.
  Purpose: Turn the plan-approved migration and verification model into operator steps with clear pass or fail criteria.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`.
  Acceptance Criteria: Quickstart covers pre-migration gates, dry-run JSON interpretation, write-deny window, remap reads, verification queries, stop conditions, rollback posture, final sign-off, and post-cutover validation with explicit expected outcomes; no step implies partial reverse rewrite or steady-state mixed population.
  Dependency: T002, T003, T004.
  Risk Closed: Operator-runbook drift, verification-gap risk, rollback confusion.

- [ ] T006 Align public API schemas and examples in specs/002-canonical-source-external-id/contracts/memory-ingest.openapi.yaml
  Title: Align memory-ingest OpenAPI with canonical identity and provenance parity.
  Purpose: Freeze the public contract so registration and retrieval expose canonical URI semantics without leaking internal diagnostics.
  Files/Areas: `specs/002-canonical-source-external-id/contracts/memory-ingest.openapi.yaml`.
  Acceptance Criteria: Registration and retrieval schemas both expose the same `source_metadata.system` fields; manual registration accepts only canonical URI examples; direct-standard examples show canonical URI `external_id` plus provenance-only `original_standard_id`; `raw_body_hash`, `migration_phase`, `legacy_resolution_path`, and `decision_reason` stay out of the public schema; `503` write-denied behavior is documented.
  Dependency: T001, T003, T005.
  Risk Closed: Public contract drift, provenance drift, diagnostics leakage.

- [ ] T007 Align canonical family and version registry rules in specs/002-canonical-source-external-id/contracts/canonical-vocabulary.yaml
  Title: Align canonical-vocabulary.yaml with the governed canonical URI grammar.
  Purpose: Make the canonical registry the single authoritative vocabulary and mapping artifact for runtime and test fixtures.
  Files/Areas: `specs/002-canonical-source-external-id/contracts/canonical-vocabulary.yaml`.
  Acceptance Criteria: Canonical family and version tokens match the plan; aliases are input hints only; direct-standard profile mappings list only supported profiles; governance notes require synchronized updates to spec, OpenAPI, tests, and fixtures; reviewer can confirm persisted aliases are impossible.
  Dependency: T001, T002.
  Risk Closed: Vocabulary drift, alias leakage, mapping drift.

- [ ] T008 [P] Align review checklist and example fixtures in specs/002-canonical-source-external-id/checklists/requirements.md and tests/fixtures/register_source/
  Title: Align review checklist and fixtures with the final canonical model.
  Purpose: Ensure reviewer checklists and request or response examples validate the same canonical identity, provenance, replay, migration, and observability rules as the code will implement.
  Files/Areas: `specs/002-canonical-source-external-id/checklists/requirements.md`, `tests/fixtures/register_source/`.
  Acceptance Criteria: Checklist items call out canonical identity, migration classification, mixed-population safety, observability fields, and rollout gates explicitly; canonical/manual, direct-standard, replay, conflict, and validation fixtures no longer encode raw-payload-id-as-external-id or legacy `canonical_payload_hash` semantics; reviewer can inspect fixtures and checklist without inferring unstated behavior.
  Dependency: T001 through T007.
  Risk Closed: Review-gap risk, fixture drift, contract example drift.

### Workstream B. Domain Model, Normalization, and Identity Implementation

- [ ] T009 [P] [US1] Introduce canonical identity value objects in crates/mod_memory/src/domain/source.rs, crates/mod_memory/src/domain/mod.rs, and crates/mod_memory/src/domain/source_external_id.rs
  Title: Create domain-owned canonical identity types.
  Purpose: Move canonical URI ownership into the domain so handlers, application services, and repositories stop assembling identity strings ad hoc.
  Files/Areas: `crates/mod_memory/src/domain/source.rs`, `crates/mod_memory/src/domain/mod.rs`, `crates/mod_memory/src/domain/source_external_id.rs`.
  Acceptance Criteria: Canonical URI components, provenance fields, and deterministic identity seed inputs have dedicated types or constructors; the domain exposes parse and build APIs for canonical/manual and direct-standard flows; handler or repository code no longer owns canonical URI string concatenation rules.
  Dependency: T001 through T007.
  Risk Closed: Code drift across layers, handler-owned identity logic, repository-owned identity logic.

- [ ] T010 [P] [US1] Implement source_domain normalization in crates/mod_memory/src/domain/normalization.rs and tests/unit/normalization_edges.rs
  Title: Implement authoritative source-domain normalization.
  Purpose: Encode the governed host normalization pipeline once and reject unsafe or ambiguous authorities before identity creation.
  Files/Areas: `crates/mod_memory/src/domain/normalization.rs`, `tests/unit/normalization_edges.rs`.
  Acceptance Criteria: The implementation trims whitespace, strips scheme, removes one leading `www.`, removes trailing dot and port, lowercases the host, punycodes IDNs, and rejects userinfo, path contamination, query-derived host, and ambiguous authority inputs; unit coverage names each required edge case directly.
  Dependency: T009.
  Risk Closed: Source-domain normalization drift, false equivalence, false acceptance of unsafe authorities.

- [ ] T011 [P] [US1] Implement object_id non-lossy normalization in crates/mod_memory/src/domain/normalization.rs and tests/unit/normalization_edges.rs
  Title: Implement authoritative object-id normalization.
  Purpose: Preserve semantically meaningful characters and spacing while producing deterministic URI-safe canonical identifiers.
  Files/Areas: `crates/mod_memory/src/domain/normalization.rs`, `tests/unit/normalization_edges.rs`.
  Acceptance Criteria: Only outer whitespace is trimmed; empty-after-trim input fails; case and internal spaces are preserved; reserved or non-unreserved bytes are percent-encoded from UTF-8 bytes; raw and encoded length limits are enforced; no destructive stripping or collapsing remains.
  Dependency: T009.
  Risk Closed: Object-id collision risk, lossy normalization, namespace instability.

- [ ] T012 [P] [US1] Implement canonical_id_version persistence and deterministic source_id derivation in crates/mod_memory/src/domain/source.rs and crates/mod_memory/src/domain/source_identity.rs
  Title: Implement the deterministic seed contract.
  Purpose: Enforce one internal identity rule for new and migrated rows.
  Files/Areas: `crates/mod_memory/src/domain/source.rs`, `crates/mod_memory/src/domain/source_identity.rs`.
  Acceptance Criteria: Governed rows always persist `canonical_id_version = v1`; the only seed contract is `source|{canonical_id_version}|{canonical_external_id}`; deterministic UUID v5 output is stable for equivalent canonical identities; no legacy-only or raw-body-derived seed branch exists.
  Dependency: T009.
  Risk Closed: Deterministic source-id drift, future-version collision risk, migration and runtime mismatch.

- [ ] T013 [P] [US1] Separate canonical external_id from provenance in crates/mod_memory/src/application/register_source.rs and crates/mod_memory/src/domain/source.rs
  Title: Enforce external_id role separation.
  Purpose: Make canonical `external_id` the authoritative persisted identity while keeping direct-standard raw IDs in provenance only.
  Files/Areas: `crates/mod_memory/src/application/register_source.rs`, `crates/mod_memory/src/domain/source.rs`.
  Acceptance Criteria: `external_id` is always populated with the canonical URI on create or replay; `original_standard_id` is stored only under `source_metadata.system`; no branch persists a third-party raw `id` into authoritative `external_id` storage.
  Dependency: T009, T010, T011, T012.
  Risk Closed: External-id role confusion, provenance overwrite, direct-standard passthrough risk.

- [ ] T014 [P] [US2] Adopt semantic_payload_hash as the authoritative comparator in crates/mod_memory/src/domain/source.rs and crates/mod_memory/src/application/register_source.rs
  Title: Adopt semantic_payload_hash across the domain model.
  Purpose: Replace legacy `canonical_payload_hash` semantics with the plan-approved replay or conflict comparator.
  Files/Areas: `crates/mod_memory/src/domain/source.rs`, `crates/mod_memory/src/application/register_source.rs`.
  Acceptance Criteria: Authoritative domain fields and command objects use `semantic_payload_hash`; `canonical_payload_hash` is treated as migration-classification input only; no live write path or public response refers to the legacy name.
  Dependency: T009.
  Risk Closed: Replay comparator drift, vocabulary drift, contract and code mismatch.

- [ ] T015 [P] [US2] Enforce raw_body_hash diagnostic-only policy in crates/mod_memory/src/domain/source.rs and crates/mod_memory/src/application/register_source.rs
  Title: Restrict raw_body_hash to diagnostics and audit only.
  Purpose: Prevent raw-body hashes from becoming a hidden replay, conflict, or public-contract input.
  Files/Areas: `crates/mod_memory/src/domain/source.rs`, `crates/mod_memory/src/application/register_source.rs`.
  Acceptance Criteria: `raw_body_hash` is stored only when a raw body exists; replay and conflict comparison code cannot read it; metrics label definitions cannot include it; public response mappers omit it by construction.
  Dependency: T014.
  Risk Closed: False replay, false conflict, diagnostics leakage, high-cardinality metrics drift.

- [ ] T016 [US1] Load the canonical registry from one governed source in crates/mod_memory/src/domain/source_external_id.rs and crates/mod_memory/src/application/register_source.rs
  Title: Wire canonical vocabulary governance into runtime identity construction.
  Purpose: Stop family and version tokens, alias mapping, and direct-standard profile mapping from drifting across files.
  Files/Areas: `crates/mod_memory/src/domain/source_external_id.rs`, `crates/mod_memory/src/application/register_source.rs`.
  Acceptance Criteria: Runtime identity construction uses a single mirrored vocabulary or static mapping derived from `canonical-vocabulary.yaml`; aliases are accepted only as input mapping hints; persisted family and version tokens always use canonical values.
  Dependency: T007, T009, T010, T011.
  Risk Closed: Runtime vocabulary drift, canonical token drift, alias persistence.

- [ ] T017 [P] [US1] Implement fail-fast validation for manual and direct-standard identity inputs in crates/mod_memory/src/domain/source_external_id.rs, crates/mod_memory/src/application/register_source.rs, and crates/app_server/src/handlers/source_register.rs
  Title: Reject invalid canonical identity inputs before state creation.
  Purpose: Guarantee that invalid canonical/manual values or untrusted direct-standard mappings fail before any authoritative write or derived UUID is created.
  Files/Areas: `crates/mod_memory/src/domain/source_external_id.rs`, `crates/mod_memory/src/application/register_source.rs`, `crates/app_server/src/handlers/source_register.rs`.
  Acceptance Criteria: Manual non-canonical URIs fail fast; direct-standard requests without a trustworthy domain or valid object ID fail fast; validation errors map to closed decision reasons; no repository writes occur for rejected requests.
  Dependency: T010 through T016.
  Risk Closed: Partial-write risk, invalid identity persistence, inconsistent validation across layers.

### Workstream C. Handler, Application, Repository, and Contract Behavior

- [ ] T018 [P] [US1] Update registration contract coverage in tests/contract/register_source_contract.rs, tests/contract/register_source_standard_validation_matrix.rs, and tests/contract/openapi_smoke.rs
  Title: Lock registration contracts to the canonical identity model.
  Purpose: Make contract tests fail on any reintroduction of raw payload ID passthrough, provenance drift, or public diagnostics leakage.
  Files/Areas: `tests/contract/register_source_contract.rs`, `tests/contract/register_source_standard_validation_matrix.rs`, `tests/contract/openapi_smoke.rs`.
  Acceptance Criteria: Tests assert canonical/manual success and rejection, direct-standard canonicalization, provenance parity, and `503` write-denied behavior; all examples align with OpenAPI and quickstart artifacts.
  Dependency: T006, T013, T014, T015, T017.
  Risk Closed: Contract drift, registration behavior drift, public-response drift.

- [ ] T019 [P] [US1] Update registration fixtures in tests/fixtures/register_source/canonical_success.json, tests/fixtures/register_source/standards/, and tests/fixtures/register_source/validation_matrix/
  Title: Update registration fixtures for canonical URI and provenance separation.
  Purpose: Make every canonical/manual and direct-standard fixture embody the final external-id and provenance rules.
  Files/Areas: `tests/fixtures/register_source/canonical_success.json`, `tests/fixtures/register_source/standards/`, `tests/fixtures/register_source/validation_matrix/`.
  Acceptance Criteria: Accepted fixtures persist canonical URI `external_id`; direct-standard fixtures keep raw `id` only in provenance expectations; rejection fixtures encode fail-fast domain or object-id failures; no fixture expects legacy `canonical_payload_hash` or non-deterministic `source_id` behavior.
  Dependency: T008, T017.
  Risk Closed: Fixture drift, replay and validation blind spots.

- [ ] T020 [P] [US1] Remove direct-standard raw payload id passthrough in crates/app_server/src/handlers/source_register.rs and crates/mod_memory/src/application/register_source.rs
  Title: Replace raw payload ID passthrough with canonical identity derivation.
  Purpose: Ensure direct-standard ingest never stores or returns a third-party raw `id` as authoritative `external_id`.
  Files/Areas: `crates/app_server/src/handlers/source_register.rs`, `crates/mod_memory/src/application/register_source.rs`.
  Acceptance Criteria: Direct-standard paths derive canonical URI from governed components; raw payload `id` is persisted only as `source_metadata.system.original_standard_id`; registration returns the canonical URI regardless of payload `id` spelling.
  Dependency: T013, T016, T017, T019.
  Risk Closed: Direct-standard identity drift, raw ID passthrough, canonical contract violation.

- [ ] T021 [P] [US1] Enforce manual canonical validation fail-fast in crates/app_server/src/handlers/source_register.rs and crates/mod_memory/src/domain/source_external_id.rs
  Title: Fail fast on invalid caller-supplied canonical URIs.
  Purpose: Keep canonical/manual ingest from creating authoritative rows outside the project-owned namespace.
  Files/Areas: `crates/app_server/src/handlers/source_register.rs`, `crates/mod_memory/src/domain/source_external_id.rs`.
  Acceptance Criteria: Manual requests with non-canonical URI grammar or namespace fail before application or repository persistence; responses map to closed validation error surfaces; logs and traces carry the rejection decision reason.
  Dependency: T017, T018.
  Risk Closed: Namespace escape, invalid-canonical acceptance, hidden partial state.

- [ ] T022 [P] [US2] Replace replay or conflict behavior in crates/mod_memory/src/application/register_source.rs and crates/mod_memory/src/infra/surreal_source_repo.rs
  Title: Implement canonical replay and conflict decisions.
  Purpose: Make runtime replay semantics follow canonical URI plus semantic hash and nothing else.
  Files/Areas: `crates/mod_memory/src/application/register_source.rs`, `crates/mod_memory/src/infra/surreal_source_repo.rs`.
  Acceptance Criteria: Same canonical URI plus same `semantic_payload_hash` returns replay; same canonical URI plus different `semantic_payload_hash` returns conflict; raw formatting and `raw_body_hash` cannot affect the result; repository lookups key off canonical identity and semantic hash only.
  Dependency: T014, T015, T018.
  Risk Closed: False replay, false conflict, repository and application mismatch.

- [ ] T023 [P] [US3] Align registration response provenance in crates/app_server/src/handlers/source_register.rs and crates/mod_memory/src/application/register_source.rs
  Title: Return the public provenance envelope on registration.
  Purpose: Ensure registration exposes the same public provenance shape as retrieval.
  Files/Areas: `crates/app_server/src/handlers/source_register.rs`, `crates/mod_memory/src/application/register_source.rs`.
  Acceptance Criteria: Successful create and replay responses include `canonical_id_version`, `ingest_kind`, `semantic_payload_hash`, and `original_standard_id` only when present; `raw_body_hash`, `migration_phase`, and `legacy_resolution_path` never appear in public registration responses.
  Dependency: T006, T013, T014, T015, T020, T022.
  Risk Closed: Registration or retrieval provenance drift, public diagnostics leakage.

- [ ] T024 [P] [US3] Align retrieval response provenance in crates/app_server/src/handlers/source_get.rs, crates/mod_memory/src/application/get_source.rs, and crates/mod_memory/src/infra/surreal_source_query.rs
  Title: Return the same public provenance envelope on retrieval.
  Purpose: Make retrieval reflect the exact authoritative provenance model already fixed for registration.
  Files/Areas: `crates/app_server/src/handlers/source_get.rs`, `crates/mod_memory/src/application/get_source.rs`, `crates/mod_memory/src/infra/surreal_source_query.rs`.
  Acceptance Criteria: Retrieval responses expose the same `source_metadata.system` fields as registration; `external_id` remains canonical at the top level; diagnostic-only fields remain internal; legacy rows resolved through remap still surface canonical provenance.
  Dependency: T006, T016, T023.
  Risk Closed: Retrieval contract drift, provenance inconsistency, remap response inconsistency.

- [ ] T025 [P] [US2] Align repository lookup, upsert, and conflict semantics in crates/mod_memory/src/infra/surreal_source_repo.rs and crates/mod_memory/src/infra/surreal_source_query.rs
  Title: Make repository semantics canonical-aware.
  Purpose: Ensure storage lookups, upserts, and conflict checks match application-layer canonical identity behavior.
  Files/Areas: `crates/mod_memory/src/infra/surreal_source_repo.rs`, `crates/mod_memory/src/infra/surreal_source_query.rs`.
  Acceptance Criteria: Canonical URI drives uniqueness checks; repository upserts return replay for semantic matches and conflict for semantic mismatches; lookup paths for canonical external ID and deterministic `source_id` remain explicit and testable; no raw standard ID fallback remains in steady-state logic.
  Dependency: T012, T020, T022, T024.
  Risk Closed: Storage-layer drift, duplicate-row creation, hidden legacy fallback.

- [ ] T026 [US1] Persist deterministic source_id governance in crates/mod_memory/src/infra/surreal_source_repo.rs and crates/core_infra/src/surrealdb.rs
  Title: Persist canonical rows with deterministic governed source_id values.
  Purpose: Make every new authoritative row conform to the final seed contract at persistence time.
  Files/Areas: `crates/mod_memory/src/infra/surreal_source_repo.rs`, `crates/core_infra/src/surrealdb.rs`.
  Acceptance Criteria: New or replayed rows resolve to the deterministic UUID v5 `source_id` derived from `source|v1|{canonical_external_id}`; persistence code has no non-deterministic branch for governed rows; canonical ID version is stored with the row.
  Dependency: T012, T025.
  Risk Closed: Persistence-time source-id drift, migration and runtime identity mismatch.

### Workstream D. Mixed-Population Safety

- [ ] T027 [P] [US4] Add old-row-only behavior coverage in tests/integration/get_source_flow.rs and tests/integration/memory_ingest_vertical_slice.rs
  Title: Prove old-row-only read behavior during the migration window.
  Purpose: Ensure legacy-only rows remain readable through the documented resolution path while writes stay denied.
  Files/Areas: `tests/integration/get_source_flow.rs`, `tests/integration/memory_ingest_vertical_slice.rs`.
  Acceptance Criteria: Old-row-only fixtures show retrieval succeeds through the legacy or remap path, registration writes return write-denied behavior during migration, and no false replay or false conflict decision is evaluated on live writes in this state.
  Dependency: T024, T025, T026.
  Risk Closed: Mixed-population blind spot, unexpected write acceptance, legacy-read regression.

- [ ] T028 [P] [US4] Add new-row-only behavior coverage in tests/integration/register_source_flow.rs and tests/integration/get_source_flow.rs
  Title: Prove new-row-only behavior after cutover.
  Purpose: Verify steady state contains only canonical rows with deterministic `source_id` and normal replay or conflict semantics.
  Files/Areas: `tests/integration/register_source_flow.rs`, `tests/integration/get_source_flow.rs`.
  Acceptance Criteria: New-row-only datasets allow registration and retrieval through canonical identity; responses carry canonical provenance; replay or conflict outcomes follow semantic rules; no legacy remap path is exercised.
  Dependency: T022, T023, T024, T026.
  Risk Closed: Steady-state drift, accidental legacy-path retention.

- [ ] T029 [P] [US4] Add old-and-new coexistence behavior coverage in tests/integration/multi_instance_consistency.rs and tests/integration/get_source_flow.rs
  Title: Prove coexistence remains a transient read-only migration state.
  Purpose: Verify old plus new rows can coexist only during the migration window and that reads prefer the rewritten authoritative row.
  Files/Areas: `tests/integration/multi_instance_consistency.rs`, `tests/integration/get_source_flow.rs`.
  Acceptance Criteria: Coexistence tests show registration writes are denied, reads prefer the rewritten canonical row, legacy IDs resolve via remap with explicit legacy-resolution diagnostics, same-hash duplicates consolidate, different-hash duplicates abort.
  Dependency: T027, T028.
  Risk Closed: False replay across old and new rows, false conflict across old and new rows, steady-state coexistence risk.

- [ ] T030 [P] [US4] Implement remap lookup behavior in crates/mod_memory/src/infra/surreal_source_query.rs, crates/mod_memory/src/application/get_source.rs, and crates/app_server/src/handlers/source_get.rs
  Title: Implement deterministic remap lookup during migration.
  Purpose: Resolve legacy `source_id` reads safely to the canonical target row throughout the migration window.
  Files/Areas: `crates/mod_memory/src/infra/surreal_source_query.rs`, `crates/mod_memory/src/application/get_source.rs`, `crates/app_server/src/handlers/source_get.rs`.
  Acceptance Criteria: Retrieval by legacy `source_id` resolves to the target deterministic row with an explicit legacy-resolution path; retrieval by canonical identity resolves directly to the canonical row; remap lookup is disabled once cutover verification completes.
  Dependency: T024, T025, T026.
  Risk Closed: Broken reads during migration, ambiguous lookup behavior, stale remap exposure after cutover.

- [ ] T031 [P] [US4] Enforce write deny behavior during the migration window in crates/app_server/src/state.rs, crates/app_server/src/middleware.rs, and crates/app_server/src/handlers/source_register.rs
  Title: Deny registration writes during rewrite and verification phases.
  Purpose: Eliminate live-write races while legacy and canonical rows coexist.
  Files/Areas: `crates/app_server/src/state.rs`, `crates/app_server/src/middleware.rs`, `crates/app_server/src/handlers/source_register.rs`.
  Acceptance Criteria: Registration writes return the documented denial response during migration phases; reads remain available; any write observed during the maintenance window emits a stop-condition diagnostic; tests cover denied write behavior explicitly.
  Dependency: T018, T027, T030.
  Risk Closed: Mixed-population race condition, partial migration corruption, untracked maintenance-window writes.

- [ ] T032 [US4] Handle duplicate canonical identity candidates in crates/mod_memory/src/infra/surreal_source_repo.rs and crates/core_infra/src/surrealdb.rs
  Title: Enforce consolidate-versus-abort handling for duplicate canonical candidates.
  Purpose: Prevent false replay and false conflict during migration classification and coexistence reads.
  Files/Areas: `crates/mod_memory/src/infra/surreal_source_repo.rs`, `crates/core_infra/src/surrealdb.rs`.
  Acceptance Criteria: Same canonical URI plus same semantic hash yields `consolidate`; same canonical URI plus different semantic hash yields `unmigratable` and stops rollout; shadow duplicates do not surface as parallel authoritative rows.
  Dependency: T022, T025, T029, T030, T031.
  Risk Closed: Duplicate canonical identity ambiguity, false replay, false conflict, migration cutover corruption.

### Workstream E. Migration Execution and Verification

- [ ] T033 [P] [US4] Implement migratable, consolidate, and unmigratable classification in crates/mod_memory/src/infra/surreal_source_repo.rs and crates/core_infra/src/surrealdb.rs
  Title: Implement the closed migration classification model.
  Purpose: Classify every legacy row under one of the three allowed outcomes with no residual ambiguous state.
  Files/Areas: `crates/mod_memory/src/infra/surreal_source_repo.rs`, `crates/core_infra/src/surrealdb.rs`.
  Acceptance Criteria: Classification uses canonical URI derivation, `semantic_payload_hash`, dependent-reference coverage, and duplicate-candidate rules; no row exits without `migratable`, `consolidate`, or `unmigratable`; the stored decision reason matches the taxonomy.
  Dependency: T012, T014, T025, T032.
  Risk Closed: Legacy-row migration classification execution gap, ambiguous cutover readiness, divergent runtime and migration rules.

- [ ] T034 [P] [US4] Implement dry-run JSON output in crates/mod_memory/src/infra/surreal_source_repo.rs and crates/core_infra/src/surrealdb.rs
  Title: Emit the fixed machine-readable dry-run report.
  Purpose: Give operators and reviewers one authoritative artifact for pass or fail classification before rewrite execution starts.
  Files/Areas: `crates/mod_memory/src/infra/surreal_source_repo.rs`, `crates/core_infra/src/surrealdb.rs`.
  Acceptance Criteria: Dry-run output includes run summary counts and, for every row, the authoritative schema fields `legacy_source_id`, `legacy_external_id`, `candidate_canonical_external_id`, `candidate_source_seed`, `candidate_source_id`, `classification`, `decision_reason`, `legacy_resolution_path`, `canonical_id_version`, `semantic_payload_hash`, `raw_body_hash_present`, `dependent_reference_counts`, and `planned_action`, plus `original_standard_id` when present and `raw_body_hash` only when `raw_body_hash_present = true`; every classified row recomputes `candidate_source_id` from the exact governed seed contract `source|{canonical_id_version}|{candidate_canonical_external_id}`; the task fails if any emitted `candidate_source_id` does not match the UUID v5 derived from that exact seed; the emitted row data makes per-row seed reproducibility reviewable; JSON shape matches the authoritative schema in the plan and quickstart exactly.
  Dependency: T033.
  Risk Closed: Dry-run contract drift, operator interpretation risk, migration-audit gap.

- [ ] T035 [P] [US4] Validate source_id rewrite and dependent-reference rewrite in crates/mod_memory/src/infra/surreal_source_repo.rs, crates/mod_memory/src/infra/surreal_memory_repo.rs, and crates/core_infra/src/surrealdb.rs
  Title: Rewrite source rows and dependent references safely.
  Purpose: Move authoritative rows and every dependent reference to deterministic `source_id` values without leaving missing or split references behind.
  Files/Areas: `crates/mod_memory/src/infra/surreal_source_repo.rs`, `crates/mod_memory/src/infra/surreal_memory_repo.rs`, `crates/core_infra/src/surrealdb.rs`.
  Acceptance Criteria: Actual rewrite updates authoritative source rows, `memory_item.source_id`, and `memory_index_job.source_id`; missing rewrite coverage stops execution; rewritten rows remove authoritative `canonical_payload_hash`; pass or fail is visible via verification helpers.
  Dependency: T033, T034.
  Risk Closed: Dependent-reference rewrite gap, partial rewrite corruption, legacy alias retention.

- [ ] T036 [P] [US4] Implement rewrite completeness threshold verification in crates/core_infra/src/surrealdb.rs and crates/mod_memory/src/infra/surreal_source_query.rs
  Title: Verify 100 percent rewrite completeness before cutover.
  Purpose: Enforce the rollout rule that no authoritative legacy row, alias field, or dangling reference remains.
  Files/Areas: `crates/core_infra/src/surrealdb.rs`, `crates/mod_memory/src/infra/surreal_source_query.rs`.
  Acceptance Criteria: Verification queries confirm canonical namespace compliance, `canonical_id_version = v1`, zero authoritative `canonical_payload_hash`, zero missing dependent references, surviving-row count parity, and one-to-one canonical lookup resolution; any mismatch returns fail.
  Dependency: T035.
  Risk Closed: Verification-gap risk, hidden partial migration, false cutover readiness.

- [ ] T037 [P] [US4] Enforce stop conditions in crates/core_infra/src/surrealdb.rs, crates/app_server/src/middleware.rs, and crates/app_server/src/state.rs
  Title: Stop execution on every rollout-blocking condition.
  Purpose: Make the closed stop-condition list executable rather than documentary.
  Files/Areas: `crates/core_infra/src/surrealdb.rs`, `crates/app_server/src/middleware.rs`, `crates/app_server/src/state.rs`.
  Acceptance Criteria: Any `unmigratable` row, semantic collision, missing dependent rewrite, verification failure, backup gate failure, or maintenance-window write forces an abort signal and leaves the system in the documented rollback posture; logs and traces record the exact stop reason.
  Dependency: T031, T033, T034, T035, T036.
  Risk Closed: Silent rollout failure, unsafe cutover continuation, undocumented abort handling.

- [ ] T038 [US4] Document rollback posture and operator pass or fail checks in specs/002-canonical-source-external-id/quickstart.md and specs/002-canonical-source-external-id/checklists/requirements.md
  Title: Document restore-only rollback and operator sign-off rules.
  Purpose: Make rollback and sign-off criteria explicit at the same granularity as the migration implementation.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/checklists/requirements.md`.
  Acceptance Criteria: Runbook and checklist state that rollback is full snapshot restore only; partial reverse rewrite is prohibited; operator pass or fail rules name dry-run, rewrite completeness, verification queries, stop conditions, and retained snapshot ownership.
  Dependency: T005, T034, T036, T037.
  Risk Closed: Rollback ambiguity, operator handoff risk, sign-off drift.

### Workstream F. Observability and Diagnostics

- [ ] T039 [P] [US3] Add observability assertions for registration and retrieval in tests/integration/observability_tracing_flow.rs and tests/integration/observability_metrics.rs
  Title: Prove canonical identity context is observable end to end.
  Purpose: Make observability a tested requirement rather than a logging afterthought.
  Files/Areas: `tests/integration/observability_tracing_flow.rs`, `tests/integration/observability_metrics.rs`.
  Acceptance Criteria: Tests assert presence of `request_id`, `trace_id`, `handler`, `route`, `method`, `canonical_external_id`, `canonical_id_version`, `semantic_payload_hash`, `ingest_kind`, and `decision_reason`; metrics assertions prove bounded-cardinality label usage and absence of forbidden identifiers or hashes.
  Dependency: T023, T024, T031, T037.
  Risk Closed: Observability gap, unbounded metrics risk, missing traceability.

- [ ] T040 [P] [US3] Wire structured log fields in crates/app_server/src/handlers/source_register.rs, crates/app_server/src/handlers/source_get.rs, and crates/mod_memory/src/application/register_source.rs
  Title: Emit the required structured log fields.
  Purpose: Capture canonicalization, replay, conflict, and provenance outcomes with the mandatory fields and no ad hoc field naming drift.
  Files/Areas: `crates/app_server/src/handlers/source_register.rs`, `crates/app_server/src/handlers/source_get.rs`, `crates/mod_memory/src/application/register_source.rs`.
  Acceptance Criteria: Logs emit `request_id`, `trace_id`, `handler`, `route`, `method`, `source_id` when known, `canonical_external_id`, `original_standard_id`, `canonical_id_version`, `semantic_payload_hash`, `raw_body_hash_present`, `raw_body_hash` when present, `migration_phase`, `legacy_resolution_path`, `decision_reason`, and `ingest_kind` in the relevant scenarios.
  Dependency: T023, T024, T030, T031, T037.
  Risk Closed: Missing structured fields, log taxonomy drift, operator investigation gaps.

- [ ] T041 [P] [US4] Wire trace correlation fields in crates/app_server/src/middleware.rs, crates/app_server/src/router.rs, and crates/core_infra/src/surrealdb.rs
  Title: Propagate trace and request correlation through online and offline flows.
  Purpose: Keep online handlers and offline migration execution in the same correlation model.
  Files/Areas: `crates/app_server/src/middleware.rs`, `crates/app_server/src/router.rs`, `crates/core_infra/src/surrealdb.rs`.
  Acceptance Criteria: Request and trace IDs propagate from HTTP entry points into repository and migration diagnostics; offline migration commands also emit traceable correlation context; public `X-Request-Id` behavior remains stable.
  Dependency: T039.
  Risk Closed: Broken trace correlation, disconnected offline diagnostics, request-correlation drift.

- [ ] T042 [P] [US4] Wire bounded-cardinality metrics expectations in crates/app_server/src/middleware.rs and tests/integration/observability_metrics.rs
  Title: Enforce bounded metrics for canonicalization and migration decisions.
  Purpose: Prevent canonical identifiers and hashes from leaking into metrics labels while still exposing actionable operator counters.
  Files/Areas: `crates/app_server/src/middleware.rs`, `tests/integration/observability_metrics.rs`.
  Acceptance Criteria: Metrics use only `method`, `route`, `status_code`, `document_type`, `ingest_kind`, `migration_phase`, and `decision_reason`; forbidden identifiers and hash values never appear as labels; tests fail on label-set drift.
  Dependency: T039.
  Risk Closed: High-cardinality metrics risk, observability cost blow-up, policy drift.

- [ ] T043 [US4] Wire the decision_reason taxonomy in crates/mod_memory/src/application/register_source.rs, crates/mod_memory/src/infra/surreal_source_repo.rs, and crates/core_infra/src/surrealdb.rs
  Title: Make decision_reason taxonomy authoritative in code.
  Purpose: Ensure every validation, replay, conflict, migration, and remap branch emits one closed taxonomy value.
  Files/Areas: `crates/mod_memory/src/application/register_source.rs`, `crates/mod_memory/src/infra/surreal_source_repo.rs`, `crates/core_infra/src/surrealdb.rs`.
  Acceptance Criteria: Manual validation, direct-standard mapping, replay, conflict, migratable, consolidate, unmigratable, remap lookup, verification success, abort, and rollback branches each emit the plan-approved taxonomy value; no free-form decision strings remain.
  Dependency: T022, T030, T033, T037, T040, T041, T042.
  Risk Closed: Taxonomy drift, unsearchable diagnostics, inconsistent operator reasoning.

- [ ] T044 [US4] Document public versus internal diagnostics boundaries in specs/002-canonical-source-external-id/quickstart.md, specs/002-canonical-source-external-id/research.md, and README.md
  Title: Document diagnostics boundaries for operators and reviewers.
  Purpose: Make it explicit which fields belong in API contracts versus internal logs, traces, and metrics.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/research.md`, `README.md`.
  Acceptance Criteria: Documentation lists all required internal diagnostics fields, states that `raw_body_hash`, `migration_phase`, `legacy_resolution_path`, and `decision_reason` are internal-only, and explains where operator-facing diagnostics are expected to appear.
  Dependency: T005, T006, T039, T040, T041, T042, T043.
  Risk Closed: Public diagnostics boundary drift, operator confusion, reviewer uncertainty over observability commitments.

### Workstream G. Regression-Hardening Tests

- [ ] T045 [P] [US1] Add the object_id collision matrix in tests/unit/normalization_edges.rs and tests/fixtures/register_source/replay_hashing/
  Title: Freeze object-id collision behavior.
  Purpose: Prove that normalization does not collapse distinct producer identifiers.
  Files/Areas: `tests/unit/normalization_edges.rs`, `tests/fixtures/register_source/replay_hashing/`.
  Acceptance Criteria: The matrix explicitly covers `eng3-ch01`, `eng3_ch01`, `eng3ch01`, reserved URI characters, spaces, case preservation, and raw-length versus encoded-length edges; each case names the expected canonical URI output or rejection.
  Dependency: T011, T019.
  Risk Closed: Object-id normalization regression, false equivalence, canonical URI instability.

- [ ] T046 [P] [US1] Add the source_domain edge matrix in tests/unit/normalization_edges.rs and tests/fixtures/register_source/validation_matrix/
  Title: Freeze source-domain edge behavior.
  Purpose: Prove the governed domain-normalization pipeline accepts and rejects exactly the intended inputs.
  Files/Areas: `tests/unit/normalization_edges.rs`, `tests/fixtures/register_source/validation_matrix/`.
  Acceptance Criteria: The matrix explicitly covers scheme stripping, port removal, `www.` normalization, trailing dot, IDN punycode, userinfo rejection, path contamination rejection, query-derived host rejection, and ambiguous authority rejection; each case has an expected normalized host or rejection reason.
  Dependency: T010, T019.
  Risk Closed: Domain normalization regression, unsafe authority acceptance, false source-domain equivalence.

- [ ] T047 [P] [US1] Add canonical URI golden-output and alias non-leakage tests in tests/unit/normalization_edges.rs and tests/contract/openapi_smoke.rs
  Title: Freeze canonical URI output stability.
  Purpose: Ensure canonical URI examples, namespace shape, and alias handling stay stable across code and contract changes.
  Files/Areas: `tests/unit/normalization_edges.rs`, `tests/contract/openapi_smoke.rs`.
  Acceptance Criteria: Golden tests pin normative URI examples; alias non-leakage tests prove persisted outputs always use canonical family and version tokens; registration and retrieval examples remain consistent with the OpenAPI contract.
  Dependency: T006, T007, T010, T011, T016.
  Risk Closed: Namespace drift, alias leakage, OpenAPI-example regression.

- [ ] T048 [P] [US2] Add replay and conflict regression coverage in tests/integration/register_source_replay_hashing.rs and tests/contract/register_source_standard_errors.rs
  Title: Freeze canonical replay and conflict behavior.
  Purpose: Make the closed replay or conflict rules executable across canonical/manual and direct-standard inputs.
  Files/Areas: `tests/integration/register_source_replay_hashing.rs`, `tests/contract/register_source_standard_errors.rs`.
  Acceptance Criteria: Tests explicitly assert same canonical plus same semantic hash equals replay, same canonical plus different semantic hash equals conflict, and mixed legacy or new coexistence cannot produce false replay or false conflict.
  Dependency: T022, T029, T032.
  Risk Closed: Replay regression, conflict regression, coexistence false positives.

- [ ] T049 [P] [US4] Add migration regression coverage in tests/contract/surreal_source_store_contract.rs, tests/integration/indexing_status_mapping_flow.rs, and tests/integration/multi_instance_consistency.rs
  Title: Freeze migration classification and rewrite safety.
  Purpose: Make dry-run, rewrite, partial migration, and verification behavior fail loudly on any rollout-risk regression.
  Files/Areas: `tests/contract/surreal_source_store_contract.rs`, `tests/integration/indexing_status_mapping_flow.rs`, `tests/integration/multi_instance_consistency.rs`.
  Acceptance Criteria: Tests cover dry-run classification, `migratable`, `consolidate`, and `unmigratable` rows, partial migration behavior, rewrite safety, dependent-reference rewrite safety, and verification-query pass or fail outcomes.
  Dependency: T033, T034, T035, T036, T037.
  Risk Closed: Migration regression, rewrite safety regression, verification-path regression.

- [ ] T050 [P] [US4] Add observability regression assertions in tests/integration/observability_tracing_flow.rs and tests/integration/observability_metrics.rs
  Title: Freeze required diagnostics field presence.
  Purpose: Guarantee that rollout-critical diagnostics remain present for runtime and migration investigation.
  Files/Areas: `tests/integration/observability_tracing_flow.rs`, `tests/integration/observability_metrics.rs`.
  Acceptance Criteria: Tests assert `decision_reason`, `migration_phase`, `legacy_resolution_path`, and canonical identity context are emitted in the scenarios where they are required; missing fields fail the suite.
  Dependency: T039 through T043.
  Risk Closed: Diagnostics regression, incomplete migration tracing, missing operator evidence.

### Workstream H. Rollout Runbook and Operator Readiness

- [ ] T051 [P] Rewrite the pre-migration checklist in specs/002-canonical-source-external-id/quickstart.md and specs/002-canonical-source-external-id/checklists/requirements.md
  Title: Publish the pre-migration operator gate list.
  Purpose: Make every rollout prerequisite reviewer-checkable before dry-run execution begins.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/checklists/requirements.md`.
  Acceptance Criteria: Checklist names SurrealDB snapshot creation, restore rehearsal, Meilisearch export or rebuild readiness, indexing backlog drain, maintenance-mode enablement, and release-binary readiness as separate gates with explicit pass criteria.
  Dependency: T005, T038.
  Risk Closed: Missing backup gate, incomplete preflight, operator-readiness gap.

- [ ] T052 [P] Write dry-run instructions and interpretation guidance in specs/002-canonical-source-external-id/quickstart.md and specs/002-canonical-source-external-id/research.md
  Title: Publish dry-run execution and report interpretation guidance.
  Purpose: Make the dry-run JSON artifact actionable for operators and reviewers.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/research.md`.
  Acceptance Criteria: Runbook distinguishes the authoritative required dry-run structure from abbreviated illustrative examples, explains how to execute dry-run, interpret summary counts, evaluate per-row `migratable`, `consolidate`, and `unmigratable` outcomes, recompute `candidate_source_id` from each row's exact seed string `source|{canonical_id_version}|{candidate_canonical_external_id}`, and decide pass or fail from the dry-run JSON alone without unstated judgment calls.
  Dependency: T034, T051.
  Risk Closed: Dry-run interpretation drift, operator inconsistency, sign-off ambiguity.

- [ ] T053 [P] Document verification query procedure and rewrite-completeness thresholds in specs/002-canonical-source-external-id/quickstart.md and specs/002-canonical-source-external-id/checklists/requirements.md
  Title: Publish verification queries and 100 percent completeness gates.
  Purpose: Make cutover readiness depend on concrete verifications rather than general confidence.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/checklists/requirements.md`.
  Acceptance Criteria: Runbook names verification queries for canonical namespace, `canonical_id_version`, alias removal, dependent-reference completeness, surviving-row counts, and canonical lookup uniqueness; each query has an explicit pass or fail rule and 100 percent completeness expectation.
  Dependency: T036, T051, T052.
  Risk Closed: Rollout verification gap, partial rewrite acceptance, unverifiable cutover readiness.

- [ ] T054 [P] Document stop conditions and rollback posture in specs/002-canonical-source-external-id/quickstart.md and specs/002-canonical-source-external-id/checklists/requirements.md
  Title: Publish rollout stop and rollback rules.
  Purpose: Make every abort trigger and the restore-only rollback posture explicit before execution.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/checklists/requirements.md`.
  Acceptance Criteria: Runbook states that any `unmigratable` row, semantic collision, missing rewrite, verification failure, backup-gate failure, or maintenance-window write triggers stop; rollback is full snapshot restore only; retry preconditions are documented.
  Dependency: T037, T038, T051.
  Risk Closed: Unsafe cutover continuation, rollback confusion, incomplete abort handling.

- [ ] T055 [P] Publish final sign-off and post-cutover validation in specs/002-canonical-source-external-id/quickstart.md, specs/002-canonical-source-external-id/checklists/requirements.md, and README.md
  Title: Publish final sign-off and post-cutover validation steps.
  Purpose: Separate final approval from implementation completion so rollout-critical checks do not get buried.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/checklists/requirements.md`, `README.md`.
  Acceptance Criteria: Sign-off includes canonical/manual smoke, direct-standard smoke, replay, conflict, retrieval, observability, snapshot retention, and post-cutover validation as separate approvals; reviewer can see which evidence closes each approval.
  Dependency: T044, T050, T051, T052, T053, T054.
  Risk Closed: Rollout-critical work buried in general documentation, incomplete final verification, weak operator handoff.

- [ ] T056 Run post-implementation validation commands from specs/002-canonical-source-external-id/quickstart.md and repository test targets
  Title: Execute the documented rollout validation suite.
  Purpose: Confirm that contract, integration, migration, observability, SLO, and bench gates all pass under the documented rollout model.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `tests/`, `benches/`.
  Acceptance Criteria: `cargo test --tests`, contract suites, mixed-population and migration flows, observability flows, `cargo test --test memory_ingest_slo -- --nocapture`, and `cargo bench --bench memory_ingest_latency --no-run` all pass or produce tracked failures before sign-off; operator checklist records the result.
  Dependency: T018 through T055.
  Risk Closed: Unverified rollout, regression escape, missing acceptance evidence.

## 3. Coverage Mapping

- Observability gap: `T039`, `T040`, `T041`, `T042`, `T043`, `T044`, `T050`, `T055`, `T056`.
- Legacy-row migration classification execution: `T032`, `T033`, `T034`, `T049`.
- Deterministic source_id seed implementation: `T012`, `T026`, `T033`, `T035`, `T036`.
- semantic_payload_hash authoritative adoption: `T014`, `T022`, `T025`, `T033`, `T048`.
- raw_body_hash policy enforcement: `T015`, `T023`, `T024`, `T040`, `T042`, `T050`.
- Mixed legacy or new coexistence safety: `T027`, `T028`, `T029`, `T030`, `T031`, `T032`, `T049`.
- Registration or retrieval provenance contract alignment: `T006`, `T018`, `T023`, `T024`, `T039`, `T047`.
- Rollout verification gates: `T005`, `T036`, `T037`, `T038`, `T051`, `T052`, `T053`, `T054`, `T055`, `T056`.
- Normalization regression acceptance: `T010`, `T011`, `T045`, `T046`, `T047`.
- Replay or conflict regression: `T022`, `T025`, `T048`.
- Migration dry-run and rollback safety: `T034`, `T035`, `T036`, `T037`, `T038`, `T049`, `T054`, `T055`, `T056`.

## 4. Rollout-Critical Tasks

### Must Finish Before Implementation Starts

- `T001` through `T008`: artifact, contract, checklist, and fixture alignment.

### Can Run In Parallel With Implementation

- `T039` through `T044`: observability and diagnostics wiring once behavior outputs are stable.
- `T045` through `T050`: regression matrices and diagnostics assertions as each corresponding behavior lands.
- `T051` through `T055`: operator runbook, dry-run interpretation, rollback, and sign-off documentation while migration code is being implemented and validated.

### Must Be Verified Before Cutover

- `T031`: maintenance-window write deny behavior.
- `T032` and `T033`: duplicate-candidate handling and row classification.
- `T034`: fixed dry-run JSON output.
- `T035` and `T036`: source rewrite, dependent-reference rewrite, and completeness verification.
- `T037` and `T038`: stop-condition enforcement and restore-only rollback posture.
- `T039` through `T044`: observability evidence and public versus internal diagnostics boundaries.
- `T049` and `T050`: migration and observability regression assertions.
- `T051` through `T056`: operator checklist, verification procedure, sign-off, and full validation execution.

## 5. Gaps Remaining

- None. The remaining work is implementation and verification against the closed plan, not further design clarification.