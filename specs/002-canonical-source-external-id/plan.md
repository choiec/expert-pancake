# Implementation Plan: Canonical Source External ID and Direct-Standard Ingest Alignment

**Branch**: `002-canonical-source-external-id` | **Date**: 2026-03-18 | **Spec**: `/workspaces/rust/specs/002-canonical-source-external-id/spec.md`
**Input**: Feature specification from `/workspaces/rust/specs/002-canonical-source-external-id/spec.md`

## Summary

Align source registration around one governed identity model while preserving identifier-role separation. `source_id` becomes a deterministic UUID v5 derived from canonical source identity for all persisted rows, `external_id` becomes the single project-owned canonical external URI, the original third-party payload `id` is preserved in reserved provenance metadata, and replay/conflict classification uses canonical `external_id` plus a semantic payload hash that ignores raw-formatting noise and raw-standard-id spelling differences once they canonicalize to the same identity.

The implementation stays inside the existing Axum handler -> `mod_memory` application -> domain -> SurrealDB repository boundary, but it now includes an explicit migration track. The main design work is to introduce domain-owned canonical external-id and source-id derivation rules, wire them into the boundary and application layers, and add a migration that rewrites every persisted `source_id` reference to the UUID v5 scheme.

## Technical Context

**Language/Version**: Rust edition 2024  
**Primary Dependencies**: `axum`, `tokio`, `serde`, `serde_json`, `uuid`, `sha2`, `validator`, `surrealdb`, `meilisearch-sdk`, `tower`, `tracing`  
**Storage**: SurrealDB as authoritative source storage and replay/conflict gate; Meilisearch as non-authoritative search projection  
**Testing**: `cargo test --tests`, contract tests under `tests/contract`, integration tests under `tests/integration`, unit tests under `tests/unit`, perf/SLO coverage under `tests/perf` and `benches/`  
**Target Platform**: Linux server in the repository dev container and deployment-like container runtime  
**Project Type**: Rust workspace web service with layered crates (`app_server`, `mod_memory`, `core_*`)  
**Performance Goals**: Keep existing ingest SLO gates intact: registration p95/p99 <= 5000 ms, source retrieval p95/p99 <= 200 ms, search projection p95/p99 <= 500 ms for current benchmark profile  
**Constraints**: Preserve handler/service/repository separation; no destructive object-id stripping; migrate all persisted `source_id` references to UUID v5 in this feature; fail fast on invalid canonical identity; retain first-commit raw-body preservation for direct-standard ingest; keep read-path degradation from blocking authoritative writes  
**Scale/Scope**: End-to-end change across planning artifacts, OpenAPI/contracts, migration design, app-server register boundary, `mod_memory` application/domain/infra, and unit/contract/integration tests for the single registration vertical slice

## Constitution Check

*GATE: Passed before Phase 0 research. Re-checked after Phase 1 design and still passed.*

- **Layer boundaries**: Preserved. Axum remains the request-shape and direct-standard mapping boundary. `RegisterSourceService` remains the application orchestration layer. Surreal repositories remain the authoritative replay/conflict and migration enforcement boundary.
- **Identifier role separation**: Preserved explicitly. `source_id` becomes an internal deterministic UUID v5 derived from canonical source identity, `external_id` remains the governed canonical URI, and memory-item URNs remain deterministic derived identifiers.
- **Canonical namespace and normalization**: Satisfied. The design standardizes `external_id` under `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}` with explicit `canonical_id_version = v1`, domain normalization, and non-lossy object-id encoding.
- **Direct-standard provenance**: Satisfied. Original standard payload `id` is preserved at `source_metadata.system.original_standard_id` and never replaces canonical `external_id`.
- **Non-destructive normalization**: Satisfied. The plan explicitly forbids destructive strip/collapse, routes object-id handling through percent-encoding only after outer trim, and derives `source_id` only from canonical identity rather than mutable request shape.
- **Cross-artifact sync**: Satisfied. This plan includes synchronized updates for spec artifacts, data model docs, OpenAPI, vocabulary contract, migration design, handler boundary, domain/application rules, repository semantics, and tests.

## Project Structure

### Documentation (this feature)

```text
specs/002-canonical-source-external-id/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── canonical-vocabulary.yaml
│   └── memory-ingest.openapi.yaml
└── tasks.md
```

### Source Code (repository root)

```text
crates/
├── app_server/
│   └── src/handlers/
│       └── source_register.rs
├── core_infra/
│   └── src/surrealdb.rs
└── mod_memory/
    └── src/
        ├── application/register_source.rs
        ├── domain/
        │   ├── normalization.rs
        │   ├── source.rs
        │   └── [new helper module for canonical external-id/source-id derivation if needed]
        └── infra/
            ├── repo.rs
            ├── surreal_memory_repo.rs
            └── surreal_source_repo.rs

specs/
├── 001-memory-ingest/
│   ├── research.md
│   ├── data-model.md
│   ├── spec.md
│   └── contracts/memory-ingest.openapi.yaml
└── 002-canonical-source-external-id/
    └── [planning artifacts above]

tests/
├── contract/
├── integration/
└── unit/
```

**Structure Decision**: Keep the existing workspace and crate boundaries. The feature is still a vertical change inside the ingest slice, but it now adds a migration concern that must touch authoritative and projection storage together.

## Phase Breakdown

### Phase 0: Research and Rule Consolidation

Goal: remove remaining technical ambiguity before implementation.

- Ratify canonical URI grammar `v1`, provenance fields, replay/conflict semantics, and deterministic source-id seed rules in `research.md`.
- Define whether `SourceExternalId` becomes a domain value object and how the vocabulary registry is governed.
- Decide semantic hash strategy for canonical/manual vs direct-standard ingest.
- Decide mandatory migration scope and rollback prerequisites for all persisted `source_id` references.

### Phase 1: Spec, Data Model, and Contract Alignment

Goal: make the identity and migration model executable at the artifact level.

- Update `specs/001-memory-ingest/spec.md` and `data-model.md` to describe deterministic UUID v5 `source_id` behavior.
- Update `specs/001-memory-ingest/research.md` and OpenAPI artifacts so direct-standard identity semantics no longer imply raw payload `id` as canonical identity.
- Publish `specs/002-canonical-source-external-id/contracts/canonical-vocabulary.yaml` as the governed registry and `memory-ingest.openapi.yaml` as the planned API contract delta.
- Update examples and schemas to show canonical URI `external_id`, deterministic `source_id`, `canonical_id_version`, and `original_standard_id`.

### Phase 2: Canonical External ID and Source ID Domain Model

Goal: centralize canonicalization, source-id derivation, and validation in domain-owned types.

- Introduce `SourceExternalId` value object and supporting parsed components.
- Add parser/formatter/validator helpers for `standard`, `version`, `source_domain`, and `object_id`.
- Define deterministic `source_id` derivation from a fixed namespace and canonical source seed.
- Model server-managed provenance metadata fields so application and infra layers stop building ad hoc JSON.
- Define canonical-id version enum or constant set with `v1` as the first persisted rule set.

### Phase 3: Boundary Mapping for Direct-Standard Ingest

Goal: make app-server registration produce governed canonical identifiers before the application layer sees the command.

- In `source_register.rs`, keep canonical/manual ingest as validation-only for caller-supplied canonical URIs.
- For direct-standard ingest, resolve boundary profile, trusted issuer/publisher domain, registry-backed standard/version, and object-id encoding to produce canonical `external_id`.
- Preserve raw payload `id` in server-managed provenance metadata, not in `external_id`.
- Define validation error strategy: malformed canonical/manual ids return `400 INVALID_INPUT`; direct-standard mapping failures that cannot produce trusted canonical components return `400 INVALID_STANDARD_PAYLOAD`.

### Phase 4: Application, Repository, and Migration Alignment

Goal: move create/replay semantics and stored source identity onto one deterministic rule set.

- Strengthen `RegisterSourceCommand` validation so manual ingest requires already-canonical `external_id` and ingest-mode-specific provenance completeness.
- Replace random UUID allocation with deterministic `source_id` derivation based on canonical source identity.
- Replace the current single `canonical_payload_hash` mental model with a semantic hash used for idempotency and an audit-only raw-body hash for diagnostics where applicable.
- Align Surreal repository replay logic so duplicate `external_id` rows compare semantic hashes and preserve first-commit raw body on replay.
- Implement a migration that rewrites every persisted `source_id` reference to the UUID v5 rule and fails closed on partial rewrite.

### Phase 5: Regression Suite Updates

Goal: lock the new semantics at unit, contract, integration, and migration levels.

- Add unit coverage for canonical URI parsing, source-domain normalization, object-id encoding, vocabulary alias mapping, deterministic source-id derivation, and semantic projection hashing.
- Update contract tests to assert canonical `external_id` examples, deterministic `source_id`, provenance fields, and 400/409 semantics.
- Update integration replay tests so raw `id` spelling changes that canonicalize to the same URI replay correctly while semantically different payloads still conflict.
- Add migration integration coverage that rewrites existing rows and verifies retrieval, replay, and projection references remain consistent under the new `source_id` values.

### Phase 6: Documentation, Migration Runbook, and Cleanup

Goal: ship with coherent operator guidance, migration workflow, and rollback posture.

- Update quickstart and examples to use canonical URI `external_id` forms and deterministic UUID v5 `source_id` semantics.
- Document mandatory source-id migration workflow and verification steps.
- Document rollback boundaries: code rollback is safe only if migration state is handled explicitly; data rollback requires a planned reverse migration or restore path.

## Dependency Ordering

1. Research decisions must land first because the plan depends on one canonical grammar, one source-id seed contract, one provenance shape, and one replay rule.
2. Contract/data-model alignment comes before code changes so handler, application, repository, and migration updates share the same vocabulary.
3. Domain value objects, deterministic source-id derivation, and provenance types come before handler and service changes because they define the accepted command shape.
4. Migration design comes before repository and projection updates because every persisted `source_id` reference must move together.
5. Boundary mapping comes before repository replay updates because direct-standard ingest must emit canonical `external_id`, deterministic `source_id`, and semantic hashes consistently.
6. Repository replay/conflict and migration changes come before full regression updates because tests need the final identity and rewrite rules.
7. Documentation and cleanup come last so examples reflect the implemented and tested semantics rather than provisional design.

## Architecture and Component Design

### 1. App-Server Boundary

- Keep `crates/app_server/src/handlers/source_register.rs` as the only HTTP boundary for request-family detection.
- Split current `canonicalize_request()` behavior into two explicit branches:
  - canonical/manual ingest: parse request DTO, validate that `external-id` already matches the canonical URI grammar, reject non-canonical values without rewriting.
  - direct-standard ingest: classify supported profile, extract top-level `id` as `original_standard_id`, derive canonical components from the trusted payload context, and emit a canonical URI.
- Move direct-standard canonicalization helpers behind small private functions or an internal helper module to keep the handler readable.
- Preserve raw request body as authoritative content for direct-standard ingest and calculate `raw_body_hash` there if retained.

### 2. Application Layer

- `RegisterSourceCommand` should carry validated identity/provenance inputs rather than loosely-coupled strings.
- Recommended command evolution:
  - keep `external_id: String` for persistence compatibility at the boundary between app and repo
  - derive `source_id` deterministically from canonical source identity instead of allocating a random UUID
  - add `canonical_id_version`
  - replace or rename `canonical_payload_hash` to `semantic_payload_hash`
  - add optional `raw_body_hash`
  - add server-managed provenance fields needed to materialize `source_metadata.system`
- Validation rules must branch by ingest kind:
  - canonical/manual: `external_id` must parse as canonical URI, `original_standard_id` may be absent
  - direct-standard: `external_id` must be derived canonical URI, `original_standard_id` must be present, and trusted source domain evidence must already have been resolved

### 3. Domain Model

- Introduce `SourceExternalId` as a domain value object with:
  - parsed fields: `standard`, `version`, `source_domain`, `object_id_raw`, `object_id_normalized`, `canonical_uri`
  - constructor paths: `parse_canonical_uri()` for manual ingest and `from_components()` for direct-standard mapping
  - formatter: always render the canonical URI form
  - validator: enforce namespace ownership, vocabulary membership, non-lossy object-id encoding, and length bounds
- Model `CanonicalIdVersion` explicitly with `V1` persisted as `"v1"`.
- Model deterministic `source_id` derivation with a fixed namespace and a documented canonical source seed contract.
- Expand `SourceSystemMetadata` into a richer domain struct containing at minimum:
  - `canonical_id_version`
  - `ingest_kind`
  - `original_standard_id`
  - `semantic_payload_hash`
  - `raw_body_hash`
  - optional profile/debug fields if needed for operator visibility
- Keep `Source` and `NewSource` persistence-friendly by serializing this metadata into `source_metadata.system` rather than scattering JSON writes across layers.

### 3A. Source-ID Migration Design

- Add an explicit migration workflow that rewrites every existing `source_id` to the UUID v5 value derived from canonical source identity.
- Identify and update every persisted `source_id` reference, including authoritative rows, indexing/outbox rows, search projections, and tests/fixtures that assume historical IDs.
- Decide whether migration runs in one transactionally-bounded batch per source or through a resumable offline migration job with verification checkpoints.

### 4. Normalization and Semantic Hashing

- Keep content normalization for memory items unchanged unless canonical projection requires metadata-derived placeholders.
- Add a semantic projection builder in `crates/mod_memory/src/domain/normalization.rs` that computes idempotency hashes from meaning, not raw body formatting.
- Direct-standard semantic projection should:
  - parse JSON payload
  - replace top-level identity comparison input with the canonical `external_id` or normalized component set
  - exclude audit-only provenance values that do not change source semantics
  - preserve semantically meaningful fields exactly after deterministic JSON canonicalization
- Canonical/manual semantic projection should hash the effective canonical registration command shape rather than raw request body formatting.
- Raw body preservation remains intact; raw-body hashes are diagnostic only and must not drive conflict semantics.

### 5. Repository and Conflict Semantics

- Surreal uniqueness remains on canonical `external_id` only.
- `prepare_create_or_replay()` and `commit_registration()` continue to distinguish create vs replay, but the comparison field becomes `semantic_payload_hash`.
- Governed creates derive `source_id` before persistence from canonical source identity.
- Replay returns the existing bundle only when canonical `external_id` matches and semantic payload hash matches.
- Conflict returns `409 EXTERNAL_ID_CONFLICT` when canonical `external_id` matches and semantic payload hash differs.
- Migration must eliminate mixed pre-feature `source_id` populations before rollout completes.

## Data Model Implications

- `Source.source_id` now has one meaning: a deterministic UUID v5 internal identifier derived from canonical source identity.
- `Source.external_id` now has one meaning: the canonical source URI.
- `source_metadata.system` becomes the authoritative identity provenance envelope.
- The raw standard payload `id` moves into `source_metadata.system.original_standard_id`.
- `semantic_payload_hash` supersedes `canonical_payload_hash` as the replay/conflict field name in docs and code.
- `raw_body_hash` is optional, audit-only, and most relevant for direct-standard ingest.
- No change is required to memory-item URN structure, but `source_id` generation and every persisted `source_id` reference must move to the UUID v5 rule.

## Interface and Contract Considerations

- `POST /sources/register`
  - canonical/manual requests keep the same public fields but now require `external-id` to already be canonical
  - direct-standard requests return canonical `external_id` values in the response, not raw payload ids
  - response `source_id` semantics must be documented as deterministic UUID v5
- `GET /sources/{source-id}` should surface `source_metadata.system.canonical_id_version`, `ingest_kind`, and `original_standard_id` when present
- OpenAPI should define:
  - `external-id` pattern/description for canonical URI grammar
  - deterministic UUID v5 semantics for `source_id`
  - `SourceSystemMetadata` schema with server-managed reserved fields
  - examples that show direct-standard provenance distinctly from canonical identity
- `contracts/canonical-vocabulary.yaml` governs persisted standard/version tokens and input aliases

## Failure Modes and Edge Cases

- Unsupported namespace or malformed canonical/manual URI: reject with `400 INVALID_INPUT`.
- Direct-standard payload with missing or untrusted issuer/publisher domain: reject with `400 INVALID_STANDARD_PAYLOAD`.
- Raw `id` present but empty after trim: reject with `400 INVALID_STANDARD_PAYLOAD`.
- Object id exceeding raw or encoded length limits: reject before storage.
- Two raw ids with different case or percent-encoding that canonicalize to the same URI: replay/conflict should depend on canonical URI plus semantic hash, not raw id spelling.
- Migration failures that rewrite only part of the `source_id` graph must fail closed and require repair before service rollout completes.

## Security and Privacy Notes

- Continue treating `source_metadata.system` as reserved server-managed state that cannot be overwritten by caller metadata.
- Reject ambiguous source domains instead of guessing, which avoids accepting attacker-controlled path/query/userinfo tricks as trusted identity.
- Keep raw payload preservation limited to current direct-standard behavior; no new sensitive data retention is introduced by this feature beyond storing the original standard id separately.
- Treat source-id migration as a privileged data rewrite with explicit verification and rollback controls.

## Performance and Scalability Notes

- Canonical URI parsing, percent-encoding, and UUID v5 derivation are request-local CPU work and should stay well below existing registration SLOs.
- Semantic projection hashing adds JSON parse/canonicalize work for direct-standard ingest, but the system already canonicalizes JSON for replay hashing today; scope growth is modest if implemented in-place.
- Migration runtime may be the dominant operational cost of this feature. Design it to be resumable or safely restartable if it cannot finish within one maintenance window.
- Avoid runtime YAML parsing in the hot path. The vocabulary artifact should remain the governance source, with implementation backed by a small static registry or build-time checked mirror.

## Test Strategy

### Unit Tests

- Add parser/validator coverage for:
  - canonical URI acceptance and rejection
  - deterministic `source_id` derivation stability for equivalent canonical/manual and direct-standard inputs
  - domain normalization including scheme stripping, `www.` handling, port removal, IDN punycode, and ambiguous host rejection
  - object-id raw/encoded length limits and percent-encoding behavior
  - vocabulary alias mapping to canonical tokens
  - semantic hash equivalence when raw direct-standard ids differ but canonicalize to the same URI

### Contract Tests

- Update `tests/contract/register_source_contract.rs` to assert canonical response examples, deterministic `source_id`, and provenance metadata shapes.
- Update `tests/contract/register_source_standard_errors.rs` to cover trusted-domain failure, invalid canonical/manual id, and invalid object-id length cases.
- Update `tests/contract/get_source_contract.rs` so source retrieval asserts `canonical_id_version` and `original_standard_id` visibility.
- Update OpenAPI smoke coverage to pin the new schemas and examples.

### Integration Tests

- Update `tests/integration/register_source_replay_hashing.rs` to cover raw id spelling variants that canonicalize to one URI and replay successfully.
- Add or update direct-standard flow tests to assert first-commit raw body preservation while replay uses semantic hashes.
- Add migration integration coverage that rewrites existing rows and verifies source retrieval, memory item retrieval, and projection references remain consistent under new `source_id` values.
- Preserve existing concurrency tests to ensure uniqueness still hinges on canonical `external_id` under concurrent writes.

### Performance Validation

- Re-run existing test and benchmark commands after implementation:
  - `cargo test --tests`
  - `cargo test --test memory_ingest_slo -- --nocapture`
  - `cargo bench --bench memory_ingest_latency --no-run`
- Add fixture variants if needed so the SLO suite includes canonicalized direct-standard ids that differ only in raw spelling.
- Add migration timing and verification reporting for production closeout.

## File-by-File Change Map

| Path | Planned Change |
|------|----------------|
| `specs/002-canonical-source-external-id/research.md` | Ratify canonical identity, deterministic source-id derivation, semantic hash, provenance, registry, and migration decisions |
| `specs/002-canonical-source-external-id/data-model.md` | Define deterministic source-id components, provenance metadata, migration model, and replay semantics |
| `specs/002-canonical-source-external-id/contracts/canonical-vocabulary.yaml` | Publish standard/version registry and input alias governance |
| `specs/002-canonical-source-external-id/contracts/memory-ingest.openapi.yaml` | Publish planned API contract with canonical `external_id`, deterministic `source_id`, and provenance schemas |
| `specs/002-canonical-source-external-id/quickstart.md` | Document validation, migration, and rollout workflow for the feature |
| `specs/001-memory-ingest/research.md` | Align source identity and direct-standard semantics with the new deterministic source-id and canonical external-id model |
| `specs/001-memory-ingest/data-model.md` | Update `Source` and retrieval view descriptions for deterministic `source_id` and provenance metadata |
| `specs/001-memory-ingest/spec.md` | Replace server-assigned source-id language with deterministic UUID v5 source-id language |
| `specs/001-memory-ingest/contracts/memory-ingest.openapi.yaml` | Align existing contract examples and schemas with the new identity model |
| `crates/app_server/src/handlers/source_register.rs` | Derive canonical external ids for direct-standard ingest, validate manual canonical ids, preserve original standard id, and define 400 strategies |
| `crates/mod_memory/src/application/register_source.rs` | Strengthen command validation, deterministic source-id derivation, ingest-mode branching, and system metadata assembly |
| `crates/mod_memory/src/domain/source.rs` | Introduce `SourceExternalId`, deterministic source-id derivation, `CanonicalIdVersion`, and richer `SourceSystemMetadata` |
| `crates/mod_memory/src/domain/[new canonical-id helper module]` | Implement parser/formatter/validator utilities if `source.rs` becomes too dense |
| `crates/mod_memory/src/domain/normalization.rs` | Build semantic canonical projection and semantic/raw-body hash utilities |
| `crates/mod_memory/src/infra/surreal_memory_repo.rs` | Compare semantic hashes on duplicate canonical `external_id`; preserve replay/conflict semantics |
| `crates/mod_memory/src/infra/surreal_source_repo.rs` | Ensure create-or-replay lookup remains keyed by canonical `external_id` and metadata reads stay aligned |
| `crates/core_infra/src/surrealdb.rs` | Add source-id migration support for authoritative tables if migration is repository-driven |
| `tests/unit/normalized_json_hash.rs` | Rename or extend toward semantic projection hash assertions |
| `tests/unit/normalization_edges.rs` | Add object-id/domain canonicalization, source-id derivation, and parser edge cases |
| `tests/unit/[new canonical external id tests]` | Cover `SourceExternalId` parser/formatter/validator paths |
| `tests/integration/register_source_replay_hashing.rs` | Assert replay/conflict behavior under canonicalized ids and semantic hash rules |
| `tests/integration/[new source-id migration flow]` | Verify full rewrite of existing source_id references to UUID v5 |

## Migration and Rollback Notes

- `source_id` migrates to deterministic UUID v5 for all rows. No UUID v4 `source_id` remains in persistent state after rollout.
- Existing legacy `external_id` rows may also require canonicalization or mapping support during migration if source-id derivation depends on canonical seed completion.
- This feature includes mandatory rewrite of dependent `source_id` references. No grandfathering, no mixed-mode compatibility window.
- Rollback strategy:
  - code rollback is only safe with a prepared reverse migration or snapshot restore plan
  - data rollback cannot assume old `source_id` values still exist once migration commits
  - contract/docs rollback must not reintroduce UUID v4 source-id assumptions

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Deterministic source-id seed is underspecified | Equivalent requests could still diverge or migration could generate unstable ids | Document the exact seed contract and add unit plus migration tests that pin canonical/manual and direct-standard equivalence |
| Source-id migration misses dependent references | Retrieval, indexing, or projections could break after rewrite | Enumerate every `source_id` reference in authoritative and projection storage and verify them in migration integration tests |
| Semantic hash still includes raw-id spelling noise | False conflicts or missed replays | Build semantic projection that rewrites identity to canonical external-id before hashing and add replay fixtures for id spelling variants |
| Boundary domain trust rules are too weak | Canonical ids could encode attacker-controlled or ambiguous domains | Reject untrusted/ambiguous host extraction and require issuer/publisher or configured producer-domain provenance |
| Domain logic spreads across handler and service inconsistently | Drift between manual and direct-standard ingest | Centralize parser/formatter/validator rules in domain value objects and keep handlers thin adapters |
| Docs and tests lag behind code | Future regressions in identifier meaning | Treat OpenAPI, vocabulary contract, unit tests, contract tests, migration tests, and integration tests as part of the same change set |
| Migration rollback is incomplete | Persistent state could be stranded between old and new identities | Require snapshot/restore or explicit reverse-migration procedure before production rollout |

## Open Decisions for Task Breakdown

- Whether the vocabulary registry is enforced by a build-time generated Rust table or a manually mirrored static module with parity tests.
- Whether the `source_id` seed should be exactly the canonical external-id string or a prefixed derivation string such as `source:v1:{external_id}` for future-proofing.
- Whether `semantic_payload_hash` should fully replace the existing field name in persisted metadata or be introduced alongside a compatibility alias for one release.
- Whether to expose `raw_body_hash` through retrieval contracts or keep it storage-only under `source_metadata.system`.

## Complexity Tracking

No constitution violations or complexity exceptions are required for this feature.
