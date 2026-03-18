# Quickstart: Canonical Source External ID Rollout

## Purpose

Describe the authoritative rollout sequence, safety gates, verification expectations, and validation workflow for canonical `external_id` governance and deterministic `source_id` migration.

## Prerequisites

- Rust stable toolchain with edition 2024 support
- Docker and Docker Compose
- Local SurrealDB and Meilisearch instances used by the memory-ingest slice
- Snapshot and restore access for SurrealDB tables `memory_source`, `memory_item`, and `memory_index_job`
- Meilisearch export or rebuild procedure validated before first production rollout

## Public API Validation After Implementation

1. Start infrastructure.

```bash
docker compose up -d surrealdb meilisearch
```

2. Run the service.

```bash
cargo run -p app_server
```

3. Verify canonical/manual ingest accepts only canonical URIs.

```bash
curl -i http://127.0.0.1:3000/sources/register \
  -H 'content-type: application/json' \
  --data '{
    "title": "Canonical Source",
    "external-id": "https://api.cherry-pick.net/cc/v1p3/nebooks.co.kr:eng3-ch01",
    "document-type": "markdown",
    "content": "# Heading\n\nBody"
  }'
```

Expected result:

- `201 Created`
- top-level `external_id` exactly matches the submitted canonical URI
- `source_metadata.system.canonical_id_version = "v1"`
- `source_metadata.system.ingest_kind = "canonical"`
- `source_metadata.system.semantic_payload_hash` is present
- `x-request-id` response header is present

4. Verify malformed or non-canonical manual identifiers are rejected.

```bash
curl -i http://127.0.0.1:3000/sources/register \
  -H 'content-type: application/json' \
  --data '{
    "title": "Bad Canonical Source",
    "external-id": "urn:badge:001",
    "document-type": "markdown",
    "content": "Body"
  }'
```

Expected result:

- `400 Bad Request`
- structured error body contains `request_id`
- operator diagnostics include `decision_reason = MANUAL_CANONICAL_REJECTED`

5. Verify direct-standard ingest derives canonical `external_id` and preserves the original payload `id` separately.

```bash
curl -i http://127.0.0.1:3000/sources/register \
  -H 'content-type: application/json' \
  --data '{
    "@context": ["https://www.w3.org/ns/credentials/v2"],
    "type": ["VerifiableCredential", "OpenBadgeCredential"],
    "id": "urn:example:badge:001",
    "name": "Rust Badge",
    "issuer": {"id": "https://issuer.example.org"}
  }'
```

Expected result:

- `201 Created`
- top-level `external_id = "https://api.cherry-pick.net/ob/v2p0/issuer.example.org:urn%3Aexample%3Abadge%3A001"`
- `source_metadata.system.ingest_kind = "direct_standard"`
- `source_metadata.system.original_standard_id = "urn:example:badge:001"`
- `source_metadata.system.semantic_payload_hash` is present
- public response omits `raw_body_hash`

6. Verify replay semantics ignore raw formatting and raw-id spelling noise after canonicalization.

- Submit two direct-standard payloads that normalize to the same canonical URI and the same semantic projection.
- Expect the second request to return `200 OK` with the same `source_id`.
- Confirm operator diagnostics include `decision_reason = REPLAY_SEMANTIC_MATCH`.

7. Verify conflict semantics reject semantic divergence.

- Submit a payload that resolves to the same canonical URI and a different semantic projection.
- Expect `409 Conflict`.
- Confirm operator diagnostics include `decision_reason = CONFLICT_SEMANTIC_MISMATCH`.

8. Run regression and performance gates.

```bash
cargo test --tests
cargo test --test memory_ingest_slo -- --nocapture
cargo bench --bench memory_ingest_latency --no-run
```

## Rollout Runbook

### Phase 0: Pre-migration checklist

1. Create and verify a SurrealDB snapshot for `memory_source`, `memory_item`, and `memory_index_job`.
2. Create and verify a Meilisearch export or rehearse full rebuild.
3. Confirm no indexing backlog remains.
4. Put registration writes into maintenance mode.
5. Confirm the release build contains canonical read compatibility, migration reporting, and verification tooling.
6. Run dry-run against a production-equivalent snapshot.

Cutover cannot proceed until every checklist item is complete.

### Phase 1: Dry-run

The migration command runs in dry-run mode first and emits one JSON report.

The required structure below is the authoritative dry-run and migration-verification contract. The example uses illustrative values, but every emitted row must follow this exact field set and the conditional rules for `original_standard_id` and `raw_body_hash`.

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
      "legacy_external_id": "legacy-id-or-uri",
      "candidate_canonical_external_id": "https://api.cherry-pick.net/cc/v1p3/nebooks.co.kr:eng3-ch01",
      "candidate_source_seed": "source|v1|https://api.cherry-pick.net/cc/v1p3/nebooks.co.kr:eng3-ch01",
      "candidate_source_id": "uuid",
      "classification": "migratable",
      "decision_reason": "LEGACY_ROW_MIGRATABLE",
      "legacy_resolution_path": "legacy_only",
      "canonical_id_version": "v1",
      "semantic_payload_hash": "hex",
      "raw_body_hash_present": true,
      "raw_body_hash": "hex",
      "dependent_reference_counts": {
        "memory_item": 0,
        "memory_index_job": 0,
        "search_projection": 0
      },
      "planned_action": "rewrite"
    },
    {
      "legacy_source_id": "uuid",
      "legacy_external_id": "legacy-id-or-uri",
      "candidate_canonical_external_id": "https://api.cherry-pick.net/qti/v3p0/kice.re.kr:20240621",
      "candidate_source_seed": "source|v1|https://api.cherry-pick.net/qti/v3p0/kice.re.kr:20240621",
      "candidate_source_id": "uuid",
      "classification": "consolidate",
      "decision_reason": "LEGACY_ROW_CONSOLIDATE_MATCH",
      "legacy_resolution_path": "shadow_duplicate",
      "canonical_id_version": "v1",
      "original_standard_id": "urn:example:badge:001",
      "semantic_payload_hash": "hex",
      "raw_body_hash_present": false,
      "dependent_reference_counts": {
        "memory_item": 0,
        "memory_index_job": 0,
        "search_projection": 0
      },
      "planned_action": "consolidate"
    }
  ]
}
```

Required row rules:

- Every row must include `legacy_source_id`, `legacy_external_id`, `candidate_canonical_external_id`, `candidate_source_seed`, `candidate_source_id`, `classification`, `decision_reason`, `legacy_resolution_path`, `canonical_id_version`, `semantic_payload_hash`, `raw_body_hash_present`, `dependent_reference_counts`, and `planned_action`.
- Include `original_standard_id` only when it exists on the classified source.
- Include `raw_body_hash` only when `raw_body_hash_present = true`.
- `candidate_source_seed` must be exactly `source|{canonical_id_version}|{candidate_canonical_external_id}`.
- Operator review is pass or fail from the JSON alone: recompute UUID v5 from each row's `candidate_source_seed` and stop if any recomputation does not equal the emitted `candidate_source_id`.

### Dry-run acceptance criteria

- every in-scope row appears in `rows`
- `unmigratable_rows = 0`
- `conflict_groups = 0`
- `reference_gap_rows = 0`
- every row conforms to the authoritative required structure above
- every row's `candidate_source_seed` equals `source|{canonical_id_version}|{candidate_canonical_external_id}`
- every row's `candidate_source_id` is recomputed from that exact seed and rollout stops on any mismatch
- every duplicate canonical identity is already resolved to `consolidate`

Failure of any criterion stops rollout.

### Phase 2: Execution window

1. Keep registration writes disabled.
2. Execute migration against the latest verified snapshot state.
3. Allow reads only through canonical lookup or the legacy remap path.
4. Record migration diagnostics for every rewritten or consolidated row.

### Mixed-population rules during execution

- `old row only`: readable through legacy path; registration writes denied.
- `new row only`: readable through canonical lookup; registration writes denied until verification completes.
- `old + new coexist`: rewritten row is authoritative; legacy row resolves through remap path only.
- `partially migrated population`: read-only; registration writes denied.
- `same logical object in old and new semantics`: consolidate if semantic hash matches; abort if semantic hash differs.
- `duplicate canonical identity candidates`: consolidate if semantic hash matches; abort if semantic hash differs.

## Verification Requirements

The rollout must execute verification queries that prove all of the following before sign-off:

1. Every authoritative `memory_source.external_id` uses the project-owned canonical namespace.
2. Every authoritative source row has `source_metadata.system.canonical_id_version = "v1"`.
3. No authoritative source row still has `source_metadata.system.canonical_payload_hash`.
4. Every `memory_item.source_id` points to an existing `memory_source.source_id`.
5. Every `memory_index_job.source_id` points to an existing `memory_source.source_id`.
6. The surviving authoritative source-row count equals the dry-run expected canonical identity count after consolidation.
7. Dependent memory-item and index-job counts match the pre-migration counts after repointing.
8. Every dry-run or execution row remains seed-reproducible: the emitted `candidate_source_seed` matches `source|{canonical_id_version}|{candidate_canonical_external_id}` and recomputes to the emitted `candidate_source_id`.

## Stop Conditions

Stop execution and restore the snapshot if any of the following occurs:

- any row is classified `unmigratable`
- any canonical identity collision has divergent semantic payload hashes
- any dependent reference cannot be rewritten
- any verification query fails
- any snapshot or backup gate fails
- any registration write is observed during the maintenance window

## Rollback Posture

- Rollback is full snapshot restore only.
- Partial reverse rewrites are not allowed.
- After restore, keep registration writes disabled, re-run dry-run, and do not retry until the new dry-run satisfies all acceptance criteria.

## Rewrite Completeness Threshold

- 100% of in-scope source rows are rewritten or consolidated according to the dry-run plan.
- 100% of dependent references are repointed.
- 0 authoritative rows remain with non-canonical `external_id`, legacy `canonical_payload_hash`, or non-deterministic `source_id`.

## Final Sign-off Gates

1. All verification requirements pass.
2. Canonical/manual registration smoke test passes.
3. Direct-standard registration smoke test passes.
4. Replay and conflict smoke tests pass.
5. Retrieval of migrated rows passes.
6. Observability diagnostics include decision reasons, request correlation, and migration phase fields.
7. Snapshots remain retained through the post-cutover validation window.

## Mandatory Regression Acceptance

### Object-id collision matrix

- `eng3-ch01`
- `eng3_ch01`
- `eng3ch01`
- reserved URI characters
- spaces
- case preservation
- raw-length versus encoded-length edges

### Source-domain edge matrix

- scheme stripping
- port removal
- `www.` normalization
- trailing dot handling
- IDN punycode
- userinfo rejection
- path contamination rejection
- query-derived host rejection
- ambiguous authority rejection

### Canonical URI regression

- golden outputs
- alias non-leakage
- namespace output stability
