# Implementation Plan: Canonical Source External ID and Direct-Standard Ingest Alignment

**Branch**: `002-canonical-source-external-id` | **Date**: 2026-03-18 | **Spec**: `/workspaces/rust/specs/002-canonical-source-external-id/spec.md`

## Summary

Implement the canonical 002 identity model directly and delete all migration-era compatibility machinery. The resulting system keeps one canonical URI grammar, one deterministic UUID v5 `source_id` rule, one semantic replay comparator, and one public provenance shape.

## Technical context

- **Language/Version**: Rust 2024
- **Primary Dependencies**: axum, tokio, tracing, serde, uuid v5 support, surrealdb, meilisearch-sdk
- **Storage**: SurrealDB authoritative tables `memory_source`, `memory_item`, `memory_index_job`; Meilisearch search projection
- **Testing**: `cargo test --tests`, contract tests, integration tests, unit normalization tests, `cargo test --test memory_ingest_slo -- --nocapture`, `cargo bench --bench memory_ingest_latency --no-run`
- **Target Platform**: Linux server workload on Debian 13 dev container and production-equivalent Linux hosts
- **Constraints**: deterministic canonicalization, no lossy normalization, no public exposure of `raw_body_hash`, no high-cardinality metrics labels, no collapse of `source_id` and `external_id`
- **Pre-production decision**: no deployed legacy data exists, so migration and compatibility support are intentionally removed rather than preserved.

## Constitution check

- **Layer boundaries**: Axum handlers remain thin adapters, `mod_memory` owns canonical identity and replay rules, and repositories own persistence semantics.
- **Identifier role separation**: `source_id`, canonical `external_id`, and memory-item URNs remain separate roles.
- **Canonicalization governance**: one project-owned namespace and `canonical_id_version = v1` stay explicit.
- **Direct-standard provenance**: `original_standard_id` remains provenance-only.
- **Observability**: keep request correlation and domain-relevant diagnostics; remove migration-only runtime fields.

## Planning invariants

1. `external_id` is always the canonical URI for governed rows.
2. `source_id` is always derived from `source|v1|{canonical_external_id}`.
3. `semantic_payload_hash` is the only authoritative replay and conflict comparator.
4. `raw_body_hash` is diagnostics-only and never affects public identity semantics.
5. Registration and retrieval expose the same public provenance envelope under `source_metadata.system`.
6. No runtime code remains for migration, remap, mixed-population behavior, or compatibility aliases.
7. Memory-item URN generation does not change.

## Implementation shape

### Domain

- Keep canonical URI parsing and construction in `mod_memory::domain::source_external_id`.
- Keep deterministic UUID v5 generation in `mod_memory::domain::source_identity`.
- Keep normalization rules for source domain and object id.

### Application and repository

- Registration computes canonical identity and deterministic `source_id` before persistence.
- Repository replay checks use only canonical `external_id` plus `semantic_payload_hash`.
- Retrieval resolves only canonical authoritative rows; no remap or legacy alias path exists.

### HTTP surface

- Preserve only the active public endpoints.
- Registration and retrieval return matching provenance fields.
- Public responses omit raw-body and internal-only diagnostics.

### Observability

- Keep `request_id`, `trace_id`, canonical identity context, and domain-relevant `decision_reason` values.
- Keep bounded metrics labels: `method`, `route`, `status_code`, `document_type`, `ingest_kind`, `decision_reason`.
- Remove migration-only runtime fields from logs, metrics, and responses.

## Deletions required by this option

- delete the dedicated migration subsystem
- delete remap lookup and mixed-population read paths
- delete migration-phase write denial
- delete compatibility branches for legacy aliases or fallback IDs
- delete any runtime dependence on `canonical_payload_hash`

## Validation strategy

1. Run contract, integration, unit, and perf-oriented tests.
2. Confirm manual canonical validation and direct-standard derivation still pass.
3. Confirm replay and conflict behavior still pass.
4. Confirm no runtime symbol or response shape includes migration-only state.
5. Confirm the bench target still builds.

## Status

This plan is implemented as the repository’s current 002-only runtime model.
