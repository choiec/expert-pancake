# Implementation Plan: Canonical Source External ID and Direct-Standard Ingest Alignment

**Branch**: `002-canonical-source-external-id` | **Date**: 2026-03-18 | **Spec**: `/workspaces/rust/specs/002-canonical-source-external-id/spec.md`  
**Input**: Feature specification from `/workspaces/rust/specs/002-canonical-source-external-id/spec.md`

## Summary

Close all rollout-blocking design gaps for canonical source identity by freezing one canonical URI grammar, one deterministic UUID v5 source-id seed contract, one authoritative semantic replay hash, one public provenance response shape, one internal observability model, and one operator-safe legacy migration model. The implementation work that follows this plan updates the Axum registration and retrieval boundaries, the mod_memory domain and application layer, the SurrealDB persistence model, and the contract and regression suites without changing memory-item URN behavior.

## Technical Context

**Language/Version**: Rust 2024  
**Primary Dependencies**: axum 0.8.1, tokio 1.44.2, tower-http 0.6.2 (`request-id`, `trace`), tracing 0.1.41, tracing-subscriber 0.3.19 JSON formatter, serde 1.0.219, serde_json 1.0.140, uuid 1.16.0 with v5 support, surrealdb 2.3.3, meilisearch-sdk 0.28.0  
**Storage**: SurrealDB authoritative tables `memory_source`, `memory_item`, `memory_index_job`; Meilisearch search projection; FalkorDB unaffected by this feature  
**Testing**: `cargo test --tests`, contract tests, integration tests, unit normalization tests, observability flows, `cargo test --test memory_ingest_slo -- --nocapture`, `cargo bench --bench memory_ingest_latency --no-run`  
**Target Platform**: Linux server workload on Debian 13 dev container and production-equivalent Linux hosts  
**Project Type**: Axum web service with layered handler/service/repository crates  
**Performance Goals**: Preserve existing 001 ingest SLO and bench gates; keep request-path observability bounded-cardinality; run migration in offline maintenance mode instead of the live hot path  
**Constraints**: deterministic canonicalization, no lossy normalization, no steady-state mixed v4/v5 source population, no public exposure of `raw_body_hash`, no high-cardinality metrics labels, no collapse of `source_id` and `external_id`  
**Scale/Scope**: All authoritative source rows, dependent memory items, index jobs, read models, search projections, public contracts, and regression suites in the current memory-ingest slice

## Constitution Check

*GATE: Passed before Phase 0 research and passed again after Phase 1 design.*

- **Layer boundaries**: Pass. Axum handlers remain thin request adapters, mod_memory owns canonical identity and replay rules, repositories own persistence and migration rewrite semantics.
- **Identifier role separation**: Pass. `source_id`, canonical `external_id`, and memory-item URN remain separate roles. Deterministic UUID v5 adoption changes only `source_id` generation.
- **Canonicalization governance**: Pass. The plan fixes one project-owned namespace, one `canonical_id_version = v1`, deterministic normalization rules, and one registry artifact.
- **Direct-standard provenance**: Pass. `original_standard_id` stays separate from canonical `external_id` and is exposed only as provenance.
- **Observability**: Pass. Structured logs, traces, and bounded metrics are required; request ID and trace propagation remain public headers; decision diagnostics are internal.
- **Migration safety**: Pass. The plan defines offline write freeze, dry-run, zero-tolerance stop conditions, verification queries, snapshot-backed rollback, and no steady-state mixed population.
- **Synchronized artifact review**: Pass. This plan aligns spec, research, data model, quickstart, and OpenAPI contract around the same terminology and rollout model.

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
│   └── src/
│       ├── handlers/
│       ├── middleware.rs
│       ├── router.rs
│       └── state.rs
├── core_infra/
│   └── src/
│       └── surrealdb.rs
└── mod_memory/
    └── src/
        ├── application/
        ├── domain/
        └── infra/

tests/
├── contract/
├── integration/
├── perf/
└── unit/
```

**Structure Decision**: Implement the feature inside the existing Axum handler, mod_memory domain/application, and core_infra persistence boundaries. Migration is an offline maintenance operation that uses the same domain and repository rules rather than a parallel identity stack.

## Planning Invariants

1. `external_id` is always the canonical URI for governed rows.
2. `source_id` is always derived from `source|{canonical_id_version}|{canonical_external_id}` for governed rows.
3. `semantic_payload_hash` is the only authoritative replay and conflict comparator.
4. `raw_body_hash` is stored only for diagnostics when a raw body exists and never influences replay, conflict, or public API responses.
5. `canonical_payload_hash` is a legacy read-compatibility alias only during migration and is removed from authoritative storage on rewrite.
6. Registration and retrieval responses expose the same public provenance envelope under `source_metadata.system`.
7. Mixed legacy and canonical populations exist only during an offline write-frozen migration window.
8. Migration cutover requires `unmigratable = 0`, `conflict_groups = 0`, full dependent-reference coverage, and passing verification queries.
9. Metrics labels are bounded-cardinality; canonical identifiers and hash values stay out of metrics.
10. Memory-item URN generation does not change.

## Phase 0: Research Conclusions

### Canonical Identity and Deterministic Seed

- Canonical URI grammar remains `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}`.
- `canonical_id_version` is `v1` for all rows created or rewritten by this feature.
- The deterministic source-id seed string is `source|v1|{canonical_external_id}`.
- Canonical/manual ingest parses and validates the URI directly.
- Direct-standard ingest builds the URI from normalized components defined by the vocabulary registry and the non-lossy normalization rules.

### Authoritative Hash and Compatibility Alias Policy

- `semantic_payload_hash` is the authoritative persisted field name.
- `canonical_payload_hash` is accepted only as legacy input during migration classification.
- Rewritten rows persist `semantic_payload_hash` and delete `canonical_payload_hash` from authoritative storage.
- `raw_body_hash` is retained only when a raw body exists and is never an authoritative comparator.

### Mixed-Population and Migration Safety

- Registration writes are disabled during the partial migration window.
- Reads remain available and resolve source lookups through a remap layer from legacy source IDs to deterministic targets.
- Cutover commits only after verification confirms zero legacy authoritative rows remain.
- Rollback restores snapshots rather than attempting partial forward or backward rewrite.

### Observability Contract Scope

- Public contract surface: `x-request-id` response header, trace propagation headers, error-body `request_id`, and the public provenance envelope in registration and retrieval responses.
- Internal diagnostics surface: structured logs, traces, and bounded-cardinality metrics for canonicalization, replay, conflict, migration classification, migration execution, and legacy lookup resolution.

## Phase 1: Design

### Canonical Identity Model

- Canonical URI components are `standard`, `version`, `source_domain`, `object_id_raw`, `object_id_normalized`, and `canonical_id_version`.
- `source_domain` normalization removes one leading `www.`, strips scheme and port, lowercases the host, punycodes IDNs, and rejects userinfo or query/path-derived hosts.
- `object_id` normalization trims outer whitespace only, preserves case and internal spacing, percent-encodes reserved and non-unreserved bytes, rejects empty values after trim, and enforces raw and encoded length limits.
- Direct-standard mapping stays limited to Open Badges and CLR profiles described in `canonical-vocabulary.yaml`.

### Deterministic Source-ID Seed Contract

- **Seed format**: `source|{canonical_id_version}|{canonical_external_id}`.
- **Seed components**: canonical external URI plus grammar version, prefixed by the literal role marker `source`.
- **Canonical version inclusion**: mandatory.
- **Legacy rewrite completion**: derive `canonical_external_id`, set `canonical_id_version = v1`, then compute target `source_id`; no legacy-only derivation branch exists.
- **Partial migration stability**: target `source_id` is computed before any rewrite; legacy lookups remap to that target during the migration window.
- **Mixed-population lookup consistency**: lookup by canonical identity resolves the target row directly; lookup by legacy `source_id` consults the remap table until cutover verification completes.

### Public Provenance Contract

- Registration and retrieval both return `source_metadata.system` with the same public shape.
- Public fields are `canonical_id_version`, `ingest_kind`, `semantic_payload_hash`, and `original_standard_id` when present.
- `external_id` remains the primary identifier at the top level.
- `raw_body_hash`, `migration_phase`, `legacy_resolution_path`, and operator decision reasons remain internal diagnostics and do not appear in the public API contract.

### Observability Model

#### Required diagnostic fields

Every canonicalization, replay, conflict, migration, and legacy lookup diagnostic event carries these fields when known:

| Field | Purpose | Surface |
|---|---|---|
| `request_id` | correlate request and error surfaces | public header, public error body, logs, traces |
| `trace_id` | distributed trace correlation | headers, logs, traces |
| `handler` | identify handler entry point | logs, traces, metrics |
| `route` | route-level aggregation | logs, traces, metrics |
| `method` | request method | logs, traces, metrics |
| `source_id` | internal source identifier after resolution | logs, traces |
| `canonical_external_id` | canonical identity under evaluation | logs, traces |
| `original_standard_id` | provenance reference for direct-standard rows | logs, traces |
| `canonical_id_version` | grammar version | public provenance, logs, traces |
| `semantic_payload_hash` | authoritative replay comparator | public provenance, logs, traces |
| `raw_body_hash_present` | signals diagnostic hash availability without exposing it in metrics | logs, traces |
| `raw_body_hash` | audit-only hash value | logs, traces |
| `migration_phase` | rollout stage | logs, traces, metrics |
| `legacy_resolution_path` | lookup path used in mixed population | logs, traces |
| `decision_reason` | authoritative taxonomy value | logs, traces, metrics |
| `ingest_kind` | canonical or direct_standard | public provenance, logs, traces, metrics |

#### Decision-reason taxonomy

- `MANUAL_CANONICAL_ACCEPTED`
- `MANUAL_CANONICAL_REJECTED`
- `DIRECT_STANDARD_CANONICALIZED`
- `DIRECT_STANDARD_REJECTED_UNTRUSTED_DOMAIN`
- `DIRECT_STANDARD_REJECTED_INVALID_OBJECT_ID`
- `REPLAY_SEMANTIC_MATCH`
- `CONFLICT_SEMANTIC_MISMATCH`
- `LEGACY_ROW_MIGRATABLE`
- `LEGACY_ROW_CONSOLIDATE_MATCH`
- `LEGACY_ROW_UNMIGRATABLE_MISSING_CANONICAL_COMPONENT`
- `LEGACY_ROW_UNMIGRATABLE_DUPLICATE_CONFLICT`
- `LEGACY_ROW_UNMIGRATABLE_REFERENCE_GAP`
- `LOOKUP_RESOLVED_CANONICAL`
- `LOOKUP_RESOLVED_LEGACY_ALIAS`
- `MIGRATION_ABORTED_STOP_CONDITION`
- `MIGRATION_VERIFIED`
- `MIGRATION_ROLLED_BACK`

#### Metrics rules

- Allowed labels: `method`, `route`, `status_code`, `document_type`, `ingest_kind`, `migration_phase`, `decision_reason`.
- Forbidden labels: `canonical_external_id`, `original_standard_id`, `semantic_payload_hash`, `raw_body_hash`, `source_id`.

### Hash Policy

- `semantic_payload_hash` is computed from the semantic projection of the request after canonical identity resolution.
- Direct-standard semantic projection strips diagnostics-only fields and canonicalizes identity-sensitive inputs before hashing.
- Canonical/manual semantic projection hashes the normalized command shape rather than request formatting.
- `raw_body_hash` is stored only for direct-standard and migrated rows with retained raw bodies.
- `raw_body_hash` is not returned in public responses and does not change replay or conflict semantics.

### Legacy Migration Classification

#### Row classes

- **migratable**: canonical URI derivation succeeds, deterministic target seed is complete, semantic payload hash is available, and every dependent reference can be rewritten.
- **consolidate**: two or more rows resolve to the same canonical URI and the same semantic payload hash; one target row survives and all references repoint to its deterministic target `source_id`.
- **unmigratable**: canonical URI derivation fails, seed completion fails, semantic hashes conflict for one canonical identity, hash alias resolution is inconsistent, or dependent references cannot be rewritten fully.

#### Operator outcomes

- `migratable`: rewrite row and all dependent references to the deterministic target `source_id`, persist canonical provenance, remove legacy hash alias.
- `consolidate`: repoint dependent references to the surviving target row, archive redundant row identifiers in the dry-run and execution reports, remove duplicate authoritative row before cutover completes.
- `unmigratable`: abort run, preserve snapshots, keep registration write freeze in place, and produce row-level failure diagnostics.

#### Reject criteria

- Missing canonical component
- Ambiguous or untrusted `source_domain`
- Invalid `object_id`
- Duplicate canonical identity with divergent `semantic_payload_hash`
- Missing dependent reference coverage
- Missing or unusable snapshot or backup
- Verification query mismatch

### Dry-Run Result Format

The dry-run artifact is a machine-readable JSON document with this required structure:

```json
{
  "run_id": "uuid",
  "migration_phase": "dry_run",
  "summary": {
    "total_rows": 0,
    "migratable_rows": 0,
    "consolidation_groups": 0,
    "unmigratable_rows": 0,
    "conflict_groups": 0,
    "reference_gap_rows": 0,
    "stop_required": false
  },
  "rows": [
    {
      "legacy_source_id": "uuid",
      "candidate_source_id": "uuid",
      "legacy_external_id": "string",
      "canonical_external_id": "string",
      "canonical_id_version": "v1",
      "original_standard_id": "string",
      "semantic_payload_hash": "hex",
      "raw_body_hash_present": true,
      "classification": "migratable",
      "decision_reason": "LEGACY_ROW_MIGRATABLE",
      "legacy_resolution_path": "legacy_only",
      "dependent_reference_counts": {
        "memory_item": 0,
        "memory_index_job": 0,
        "search_projection": 0
      },
      "action": "rewrite"
    }
  ]
}
```

### Mixed-Population Coexistence Model

| State | Registration | Retrieval | Replay / Conflict | Operator action |
|---|---|---|---|---|
| old row only | denied during migration window | allowed through legacy source-id or remap lookup | not evaluated on live writes during migration window | classify row in dry-run, then rewrite or abort |
| new row only | allowed after cutover | allowed through canonical lookup and deterministic source-id | canonical rules apply | normal steady state |
| old + new coexist | denied | reads prefer rewritten canonical row; legacy ID resolves via remap with `legacy_resolution_path = LOOKUP_RESOLVED_LEGACY_ALIAS` | not evaluated on live writes during migration window | transient state only inside migration execution |
| partially migrated population | denied | allowed through remap layer | not evaluated on live writes during migration window | complete verification or restore snapshot |
| same logical object in old and new semantics | denied | rewritten row is authoritative | same semantic hash consolidates; different semantic hash aborts rollout | consolidate or abort |
| duplicate canonical identity candidates | denied | no new steady-state exposure until resolved | same semantic hash consolidates; different semantic hash aborts | consolidate or abort |

### Rollout Safety Model

#### Pre-migration checklist

1. Verify SurrealDB snapshot for `memory_source`, `memory_item`, and `memory_index_job` exists and restore integrity is validated.
2. Verify Meilisearch export exists or full rebuild procedure is validated.
3. Drain indexing backlog to zero.
4. Enable maintenance mode for registration writes.
5. Confirm the release build includes canonical read compatibility, migration diagnostics, and verification queries.
6. Run dry-run against a current production-equivalent snapshot.

#### Dry-run acceptance criteria

- `unmigratable_rows = 0`
- `conflict_groups = 0`
- `reference_gap_rows = 0`
- every row classified
- every target `source_id` derivable from `source|v1|{canonical_external_id}`
- every canonical identity collision resolved to `consolidate` or rejected before execution

#### Verification query requirements

The rollout must execute queries that prove all of the following:

1. Every authoritative `memory_source.external_id` is under `https://api.cherry-pick.net/`.
2. Every authoritative `memory_source.source_metadata.system.canonical_id_version` equals `v1`.
3. No authoritative `memory_source.source_metadata.system.canonical_payload_hash` field remains.
4. No dependent `memory_item.source_id` or `memory_index_job.source_id` points to a missing source row.
5. The count of authoritative source rows equals the dry-run expected surviving canonical identity count after consolidation.
6. The count of dependent memory items and index jobs matches pre-migration counts after repointing.
7. Every lookup by canonical `external_id` resolves to one deterministic `source_id`.

#### Snapshot and backup gates

- Snapshot creation is required immediately before execution.
- Snapshot restore rehearsal on a production-equivalent environment is required before first production rollout.
- Cutover cannot begin without verified snapshot locations and operator ownership.

#### Rollback posture

- Rollback is full snapshot restore only.
- Partial reverse rewrites are prohibited.
- If verification fails after execution, restore SurrealDB snapshot, rebuild or restore Meilisearch projection, keep registration writes disabled, and re-run dry-run before any second attempt.

#### Stop conditions

- Any `unmigratable` row
- Any duplicate canonical identity with divergent semantic payload hashes
- Any missing dependent reference rewrite
- Any failed verification query
- Any missing snapshot or backup gate
- Any unexpected registration write during the maintenance window

#### Rewrite completeness threshold

- 100% of in-scope source rows are rewritten or consolidated according to the dry-run plan.
- 100% of dependent `memory_item` and `memory_index_job` references are repointed.
- 0 authoritative rows remain with non-canonical `external_id`, legacy `canonical_payload_hash`, or non-deterministic `source_id`.

#### Final sign-off gates

1. All verification queries pass.
2. Registration smoke tests pass for canonical/manual, direct-standard, replay, and conflict flows.
3. Retrieval smoke tests pass for migrated rows.
4. Observability events contain the required decision reasons and correlation fields.
5. Snapshot retention remains in place through the post-cutover validation window.

### Validation Strategy

#### Mandatory normalization regression coverage

- **Object-id collision matrix**: `eng3-ch01`, `eng3_ch01`, `eng3ch01`, reserved URI characters, spaces, case preservation, raw-length versus encoded-length edges.
- **Source-domain edge matrix**: scheme stripping, port removal, `www.` normalization, trailing dot handling, IDN punycode, userinfo rejection, path contamination rejection, query-derived host rejection, ambiguous authority rejection.
- **Canonical URI regression**: golden outputs, alias non-leakage, namespace output stability.

#### Contract and integration coverage

- Registration contract tests for canonical/manual success and rejection, direct-standard canonicalization, replay, and conflict.
- Retrieval contract tests for public provenance shape parity.
- Storage contract tests for deterministic `source_id`, replay semantics, and dependent-reference integrity.
- Migration integration tests for dry-run, rewrite, verification, rollback triggers, and mixed-population lookup resolution.
- Observability integration tests for trace propagation, request IDs, structured decision events, and bounded-cardinality metrics.

## Phase Outputs

- `research.md`: authoritative decisions and rationale for the closed design
- `data-model.md`: canonical entities, migration report model, and diagnostics model
- `quickstart.md`: rollout runbook and verification sequence
- `contracts/memory-ingest.openapi.yaml`: public API contract with provenance parity and observability boundary notes

## Complexity Tracking

No constitution violations require justification.
