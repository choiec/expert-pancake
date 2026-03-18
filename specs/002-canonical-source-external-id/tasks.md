# Tasks: Canonical Source External ID and Direct-Standard Ingest Alignment

**Input**: Design documents from `/specs/002-canonical-source-external-id/`
**Status**: Completed for Option A pre-production simplification on 2026-03-18
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

## Completed tasks

- [X] T001 Remove the dedicated migration subsystem and all migration-only exports
  Outcome: The repository keeps no migration implementation or migration-only exports.

- [X] T002 Remove mixed-population and remap behavior from runtime storage and query paths
  Outcome: Runtime storage and query code expose only steady-state canonical behavior.

- [X] T003 Collapse application and repository result types to canonical-only semantics
  Outcome: Transition-only fields were removed from application and repository result models.

- [X] T004 Remove migration-only observability fields from handlers and metrics
  Outcome: Logs and metrics keep request correlation plus domain decision reasons only.

- [X] T005 Enforce the 002 identity and replay model as the only runtime model
  Outcome: Canonical URI validation, trusted direct-standard derivation, deterministic UUID v5 `source_id`, provenance parity, and semantic replay or conflict behavior remain as the only supported semantics.

- [X] T006 Update tests to validate only surviving 002 semantics
  Outcome: Legacy-oriented assertions and fixtures were removed or rewritten to reflect the canonical-only model.

- [X] T007 Update repository documentation and feature artifacts to describe the canonical-only model
  Outcome: Docs now state explicitly that legacy compatibility is intentionally omitted because the project is still pre-production.

- [X] T008 Run validation commands for the simplified implementation
  Outcome: Validation commands complete against the simplified runtime model.

## Validation

- [X] `cargo test --tests`
- [X] `cargo bench --bench memory_ingest_latency --no-run`

## Gaps remaining

- None. The surviving work is ordinary follow-up cleanup, not legacy-compatibility support.