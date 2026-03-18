# Tasks: Canonical Source External ID and Direct-Standard Ingest Alignment

**Input**: Design documents from `/specs/002-canonical-source-external-id/`
**Status**: Updated for Option A pre-production simplification on 2026-03-18
**Prerequisites**: `spec.md`, `plan.md`, `research.md`, `data-model.md`, `quickstart.md`, `contracts/canonical-vocabulary.yaml`, `contracts/memory-ingest.openapi.yaml`, `checklists/requirements.md`
**Tests**: Required. This option keeps only the canonical 002 runtime semantics and intentionally deletes all migration and compatibility behavior.

## Scope

This task list reflects the chosen pre-production simplification:

- keep canonical/manual URI validation
- keep direct-standard trusted-domain derivation
- keep deterministic `source_id`
- keep `semantic_payload_hash` replay and conflict semantics
- keep public provenance parity
- delete migration, remap, mixed-population, cutover, rollback, and compatibility code paths

## Completed Tasks

- [X] T001 Remove the dedicated migration subsystem and all migration-only exports
  Files/Areas: `crates/mod_memory/src/infra/mod.rs`, `crates/mod_memory/src/infra/migration.rs`.
  Outcome: Deleted dry-run, execute, verify, rollback, cutover, snapshot, and migration-report code because no deployed legacy data needs preservation.

- [X] T002 Remove mixed-population and remap behavior from runtime storage and query paths
  Files/Areas: `crates/core_infra/src/surrealdb.rs`, `crates/mod_memory/src/infra/surreal_source_query.rs`.
  Outcome: Deleted remapped lookup state, migration-phase state, backup gates, and migration-window write denial so the store exposes only steady-state canonical behavior.

- [X] T003 Collapse application and repository result types to canonical-only semantics
  Files/Areas: `crates/mod_memory/src/infra/repo.rs`, `crates/mod_memory/src/application/register_source.rs`, `crates/mod_memory/src/application/get_source.rs`, `crates/mod_memory/src/infra/surreal_source_repo.rs`, `crates/mod_memory/src/infra/surreal_memory_repo.rs`.
  Outcome: Removed transition-only fields from `SourceBundle`, registration results, retrieval results, and bundle assembly.

- [X] T004 Remove migration-only observability fields from handlers and metrics
  Files/Areas: `crates/app_server/src/handlers/source_register.rs`, `crates/app_server/src/handlers/source_get.rs`, `crates/app_server/src/state.rs`, `crates/app_server/src/middleware.rs`.
  Outcome: Removed `migration_phase` and `legacy_resolution_path` from logs and metrics while preserving request correlation and domain-relevant decision reasons.

- [X] T005 Enforce the 002 identity and replay model as the only runtime model
  Files/Areas: `crates/mod_memory/src/domain/source.rs`, `crates/mod_memory/src/domain/source_external_id.rs`, `crates/mod_memory/src/domain/source_identity.rs`, `crates/mod_memory/src/application/register_source.rs`.
  Outcome: Kept canonical URI validation, trusted direct-standard derivation, deterministic UUID v5 `source_id`, provenance parity, and semantic replay or conflict behavior; removed lingering legacy-only field stripping logic.

- [X] T006 Update tests to validate only surviving 002 semantics
  Files/Areas: `crates/app_server/src/handlers/source_register.rs`, `tests/integration/observability_metrics.rs`.
  Outcome: Replaced stale legacy-oriented handler assertions and updated metrics expectations to the simplified label set.

- [X] T007 Update repository documentation and feature artifacts to describe the canonical-only model
  Files/Areas: `README.md`, `tests/contract/README.md`, `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/tasks.md`.
  Outcome: Removed migration operator language and added an explicit pre-production note that legacy compatibility is intentionally omitted.

- [X] T008 Run validation commands for the simplified implementation
  Files/Areas: `tests/`, `benches/`.
  Outcome: `cargo test --tests` passes and the bench target remains part of the validation contract.

## Validation

- [X] `cargo test --tests`
- [X] `cargo bench --bench memory_ingest_latency --no-run`
  Purpose: Make rollback and sign-off criteria explicit at the same granularity as the migration implementation.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/checklists/requirements.md`.
  Acceptance Criteria: Runbook and checklist state that rollback is full snapshot restore only; partial reverse rewrite is prohibited; operator pass or fail rules name dry-run, rewrite completeness, verification queries, stop conditions, and retained snapshot ownership.
  Dependency: T005, T034, T036, T037.
  Risk Closed: Rollback ambiguity, operator handoff risk, sign-off drift.

### Workstream F. Observability and Diagnostics

- [X] T039 [P] [US3] Add observability assertions for registration and retrieval in tests/integration/observability_tracing_flow.rs and tests/integration/observability_metrics.rs
  Title: Prove canonical identity context is observable end to end.
  Purpose: Make observability a tested requirement rather than a logging afterthought.
  Files/Areas: `tests/integration/observability_tracing_flow.rs`, `tests/integration/observability_metrics.rs`.
  Acceptance Criteria: Tests assert presence of `request_id`, `trace_id`, `handler`, `route`, `method`, `canonical_external_id`, `canonical_id_version`, `semantic_payload_hash`, `ingest_kind`, and `decision_reason`; metrics assertions prove bounded-cardinality label usage and absence of forbidden identifiers or hashes.
  Dependency: T023, T024, T031, T037.
  Risk Closed: Observability gap, unbounded metrics risk, missing traceability.

- [X] T040 [P] [US3] Wire structured log fields in crates/app_server/src/handlers/source_register.rs, crates/app_server/src/handlers/source_get.rs, and crates/mod_memory/src/application/register_source.rs
  Title: Emit the required structured log fields.
  Purpose: Capture canonicalization, replay, conflict, and provenance outcomes with the mandatory fields and no ad hoc field naming drift.
  Files/Areas: `crates/app_server/src/handlers/source_register.rs`, `crates/app_server/src/handlers/source_get.rs`, `crates/mod_memory/src/application/register_source.rs`.
  Acceptance Criteria: Logs emit `request_id`, `trace_id`, `handler`, `route`, `method`, `source_id` when known, `canonical_external_id`, `original_standard_id`, `canonical_id_version`, `semantic_payload_hash`, `raw_body_hash_present`, `raw_body_hash` when present, `migration_phase`, `legacy_resolution_path`, `decision_reason`, and `ingest_kind` in the relevant scenarios.
  Dependency: T023, T024, T030, T031, T037.
  Risk Closed: Missing structured fields, log taxonomy drift, operator investigation gaps.

- [X] T041 [P] [US4] Wire trace correlation fields in crates/app_server/src/middleware.rs, crates/app_server/src/router.rs, and crates/core_infra/src/surrealdb.rs
  Title: Propagate trace and request correlation through online and offline flows.
  Purpose: Keep online handlers and offline migration execution in the same correlation model.
  Files/Areas: `crates/app_server/src/middleware.rs`, `crates/app_server/src/router.rs`, `crates/core_infra/src/surrealdb.rs`.
  Acceptance Criteria: Request and trace IDs propagate from HTTP entry points into repository and migration diagnostics; offline migration commands also emit traceable correlation context; public `X-Request-Id` behavior remains stable.
  Dependency: T039.
  Risk Closed: Broken trace correlation, disconnected offline diagnostics, request-correlation drift.

- [X] T042 [P] [US4] Wire bounded-cardinality metrics expectations in crates/app_server/src/middleware.rs and tests/integration/observability_metrics.rs
  Title: Enforce bounded metrics for canonicalization and migration decisions.
  Purpose: Prevent canonical identifiers and hashes from leaking into metrics labels while still exposing actionable operator counters.
  Files/Areas: `crates/app_server/src/middleware.rs`, `tests/integration/observability_metrics.rs`.
  Acceptance Criteria: Metrics use only `method`, `route`, `status_code`, `document_type`, `ingest_kind`, `migration_phase`, and `decision_reason`; forbidden identifiers and hash values never appear as labels; tests fail on label-set drift.
  Dependency: T039.
  Risk Closed: High-cardinality metrics risk, observability cost blow-up, policy drift.

- [X] T043 [US4] Wire the decision_reason taxonomy in crates/mod_memory/src/application/register_source.rs, crates/mod_memory/src/infra/surreal_source_repo.rs, and crates/core_infra/src/surrealdb.rs
  Title: Make decision_reason taxonomy authoritative in code.
  Purpose: Ensure every validation, replay, conflict, migration, and remap branch emits one closed taxonomy value.
  Files/Areas: `crates/mod_memory/src/application/register_source.rs`, `crates/mod_memory/src/infra/surreal_source_repo.rs`, `crates/core_infra/src/surrealdb.rs`.
  Acceptance Criteria: Manual validation, direct-standard mapping, replay, conflict, migratable, consolidate, unmigratable, remap lookup, verification success, abort, and rollback branches each emit the plan-approved taxonomy value; no free-form decision strings remain.
  Dependency: T022, T030, T033, T037, T040, T041, T042.
  Risk Closed: Taxonomy drift, unsearchable diagnostics, inconsistent operator reasoning.

- [X] T044 [US4] Document public versus internal diagnostics boundaries in specs/002-canonical-source-external-id/quickstart.md, specs/002-canonical-source-external-id/research.md, and README.md
  Title: Document diagnostics boundaries for operators and reviewers.
  Purpose: Make it explicit which fields belong in API contracts versus internal logs, traces, and metrics.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/research.md`, `README.md`.
  Acceptance Criteria: Documentation lists all required internal diagnostics fields, states that `raw_body_hash`, `migration_phase`, `legacy_resolution_path`, and `decision_reason` are internal-only, and explains where operator-facing diagnostics are expected to appear.
  Dependency: T005, T006, T039, T040, T041, T042, T043.
  Risk Closed: Public diagnostics boundary drift, operator confusion, reviewer uncertainty over observability commitments.

### Workstream G. Regression-Hardening Tests

- [X] T045 [P] [US1] Add the object_id collision matrix in tests/unit/normalization_edges.rs and tests/fixtures/register_source/replay_hashing/
  Title: Freeze object-id collision behavior.
  Purpose: Prove that normalization does not collapse distinct producer identifiers.
  Files/Areas: `tests/unit/normalization_edges.rs`, `tests/fixtures/register_source/replay_hashing/`.
  Acceptance Criteria: The matrix explicitly covers `eng3-ch01`, `eng3_ch01`, `eng3ch01`, reserved URI characters, spaces, case preservation, and raw-length versus encoded-length edges; each case names the expected canonical URI output or rejection.
  Dependency: T011, T019.
  Risk Closed: Object-id normalization regression, false equivalence, canonical URI instability.

- [X] T046 [P] [US1] Add the source_domain edge matrix in tests/unit/normalization_edges.rs and tests/fixtures/register_source/validation_matrix/
  Title: Freeze source-domain edge behavior.
  Purpose: Prove the governed domain-normalization pipeline accepts and rejects exactly the intended inputs.
  Files/Areas: `tests/unit/normalization_edges.rs`, `tests/fixtures/register_source/validation_matrix/`.
  Acceptance Criteria: The matrix explicitly covers scheme stripping, port removal, `www.` normalization, trailing dot, IDN punycode, userinfo rejection, path contamination rejection, query-derived host rejection, and ambiguous authority rejection; each case has an expected normalized host or rejection reason.
  Dependency: T010, T019.
  Risk Closed: Domain normalization regression, unsafe authority acceptance, false source-domain equivalence.

- [X] T047 [P] [US1] Add canonical URI golden-output and alias non-leakage tests in tests/unit/normalization_edges.rs and tests/contract/openapi_smoke.rs
  Title: Freeze canonical URI output stability.
  Purpose: Ensure canonical URI examples, namespace shape, and alias handling stay stable across code and contract changes.
  Files/Areas: `tests/unit/normalization_edges.rs`, `tests/contract/openapi_smoke.rs`.
  Acceptance Criteria: Golden tests pin normative URI examples; alias non-leakage tests prove persisted outputs always use canonical family and version tokens; registration and retrieval examples remain consistent with the OpenAPI contract.
  Dependency: T006, T007, T010, T011, T016.
  Risk Closed: Namespace drift, alias leakage, OpenAPI-example regression.

- [X] T048 [P] [US2] Add replay and conflict regression coverage in tests/integration/register_source_replay_hashing.rs and tests/contract/register_source_standard_errors.rs
  Title: Freeze canonical replay and conflict behavior.
  Purpose: Make the closed replay or conflict rules executable across canonical/manual and direct-standard inputs.
  Files/Areas: `tests/integration/register_source_replay_hashing.rs`, `tests/contract/register_source_standard_errors.rs`.
  Acceptance Criteria: Tests explicitly assert same canonical plus same semantic hash equals replay, same canonical plus different semantic hash equals conflict, and mixed legacy or new coexistence cannot produce false replay or false conflict.
  Dependency: T022, T029, T032.
  Risk Closed: Replay regression, conflict regression, coexistence false positives.

- [X] T049 [P] [US4] Add migration regression coverage in tests/contract/surreal_source_store_contract.rs, tests/integration/indexing_status_mapping_flow.rs, and tests/integration/multi_instance_consistency.rs
  Title: Freeze migration classification and rewrite safety.
  Purpose: Make dry-run, rewrite, partial migration, and verification behavior fail loudly on any rollout-risk regression.
  Files/Areas: `tests/contract/surreal_source_store_contract.rs`, `tests/integration/indexing_status_mapping_flow.rs`, `tests/integration/multi_instance_consistency.rs`.
  Acceptance Criteria: Tests cover dry-run classification, `migratable`, `consolidate`, and `unmigratable` rows, partial migration behavior, rewrite safety, dependent-reference rewrite safety, and verification-query pass or fail outcomes.
  Dependency: T033, T034, T035, T036, T037.
  Risk Closed: Migration regression, rewrite safety regression, verification-path regression.

- [X] T050 [P] [US4] Add observability regression assertions in tests/integration/observability_tracing_flow.rs and tests/integration/observability_metrics.rs
  Title: Freeze required diagnostics field presence.
  Purpose: Guarantee that rollout-critical diagnostics remain present for runtime and migration investigation.
  Files/Areas: `tests/integration/observability_tracing_flow.rs`, `tests/integration/observability_metrics.rs`.
  Acceptance Criteria: Tests assert `decision_reason`, `migration_phase`, `legacy_resolution_path`, and canonical identity context are emitted in the scenarios where they are required; missing fields fail the suite.
  Dependency: T039 through T043.
  Risk Closed: Diagnostics regression, incomplete migration tracing, missing operator evidence.

### Workstream H. Rollout Runbook and Operator Readiness

- [X] T051 [P] Rewrite the pre-migration checklist in specs/002-canonical-source-external-id/quickstart.md and specs/002-canonical-source-external-id/checklists/requirements.md
  Title: Publish the pre-migration operator gate list.
  Purpose: Make every rollout prerequisite reviewer-checkable before dry-run execution begins.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/checklists/requirements.md`.
  Acceptance Criteria: Checklist names SurrealDB snapshot creation, restore rehearsal, Meilisearch export or rebuild readiness, indexing backlog drain, maintenance-mode enablement, and release-binary readiness as separate gates with explicit pass criteria.
  Dependency: T005, T038.
  Risk Closed: Missing backup gate, incomplete preflight, operator-readiness gap.

- [X] T052 [P] Write dry-run instructions and interpretation guidance in specs/002-canonical-source-external-id/quickstart.md and specs/002-canonical-source-external-id/research.md
  Title: Publish dry-run execution and report interpretation guidance.
  Purpose: Make the dry-run JSON artifact actionable for operators and reviewers.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/research.md`.
  Acceptance Criteria: Runbook distinguishes the authoritative required dry-run structure from abbreviated illustrative examples, explains how to execute dry-run, interpret summary counts, evaluate per-row `migratable`, `consolidate`, and `unmigratable` outcomes, recompute `candidate_source_id` from each row's exact seed string `source|{canonical_id_version}|{candidate_canonical_external_id}`, and decide pass or fail from the dry-run JSON alone without unstated judgment calls.
  Dependency: T034, T051.
  Risk Closed: Dry-run interpretation drift, operator inconsistency, sign-off ambiguity.

- [X] T053 [P] Document verification query procedure and rewrite-completeness thresholds in specs/002-canonical-source-external-id/quickstart.md and specs/002-canonical-source-external-id/checklists/requirements.md
  Title: Publish verification queries and 100 percent completeness gates.
  Purpose: Make cutover readiness depend on concrete verifications rather than general confidence.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/checklists/requirements.md`.
  Acceptance Criteria: Runbook names verification queries for canonical namespace, `canonical_id_version`, alias removal, dependent-reference completeness, surviving-row counts, and canonical lookup uniqueness; each query has an explicit pass or fail rule and 100 percent completeness expectation.
  Dependency: T036, T051, T052.
  Risk Closed: Rollout verification gap, partial rewrite acceptance, unverifiable cutover readiness.

- [X] T054 [P] Document stop conditions and rollback posture in specs/002-canonical-source-external-id/quickstart.md and specs/002-canonical-source-external-id/checklists/requirements.md
  Title: Publish rollout stop and rollback rules.
  Purpose: Make every abort trigger and the restore-only rollback posture explicit before execution.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/checklists/requirements.md`.
  Acceptance Criteria: Runbook states that any `unmigratable` row, semantic collision, missing rewrite, verification failure, backup-gate failure, or maintenance-window write triggers stop; rollback is full snapshot restore only; retry preconditions are documented.
  Dependency: T037, T038, T051.
  Risk Closed: Unsafe cutover continuation, rollback confusion, incomplete abort handling.

- [X] T055 [P] Publish final sign-off and post-cutover validation in specs/002-canonical-source-external-id/quickstart.md, specs/002-canonical-source-external-id/checklists/requirements.md, and README.md
  Title: Publish final sign-off and post-cutover validation steps.
  Purpose: Separate final approval from implementation completion so rollout-critical checks do not get buried.
  Files/Areas: `specs/002-canonical-source-external-id/quickstart.md`, `specs/002-canonical-source-external-id/checklists/requirements.md`, `README.md`.
  Acceptance Criteria: Sign-off includes canonical/manual smoke, direct-standard smoke, replay, conflict, retrieval, observability, snapshot retention, and post-cutover validation as separate approvals; reviewer can see which evidence closes each approval.
  Dependency: T044, T050, T051, T052, T053, T054.
  Risk Closed: Rollout-critical work buried in general documentation, incomplete final verification, weak operator handoff.

- [X] T056 Run post-implementation validation commands from specs/002-canonical-source-external-id/quickstart.md and repository test targets
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