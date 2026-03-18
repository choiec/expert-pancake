# Feature Specification: Canonical Source External ID and Direct-Standard Ingest Alignment

**Feature Branch**: `002-canonical-source-external-id`  
**Created**: 2026-03-18  
**Status**: Implemented as pre-production Option A  
**Input**: User description plus explicit decision to remove all migration and compatibility behavior before production.

## Context

`001-memory-ingest` introduced the first authoritative ingest slice, but this repository is still pre-production. Because no deployed legacy data needs preservation, this feature adopts the simplest possible end state:

- one canonical external identity grammar
- one deterministic `source_id` rule
- one authoritative replay comparator
- no migration or compatibility subsystem

## Goals

- Define one project-owned canonical URI grammar for `external_id` across manual and direct-standard ingest.
- Preserve original direct-standard identifiers as provenance only.
- Make replay and conflict decisions depend only on canonical `external_id` plus `semantic_payload_hash`.
- Persist deterministic UUID v5 `source_id` values derived from canonical identity.
- Keep registration and retrieval provenance aligned.
- Remove all migration, remap, mixed-population, and backward-compatibility behavior.

## Non-goals

- Supporting legacy data migration or cutover.
- Preserving backward compatibility with 001-era manual inputs.
- Supporting steady-state mixed populations.
- Changing memory-item URN generation.

## User stories

### User Story 1 - Canonical identity is consistent across ingest modes

A source producer registers content through canonical/manual ingest or direct-standard ingest and receives one canonical `external_id` under the project-owned namespace.

### User Story 2 - Replay and conflict follow semantic identity

A caller can replay the same logical source without duplicates, while true semantic changes for the same canonical identity are rejected as conflicts.

### User Story 3 - Provenance remains auditable

A caller can distinguish canonical platform identity from original standard provenance in both registration and retrieval responses.

## Requirements

### Functional requirements

- **FR-001**: The system MUST persist `external_id` only under `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}`.
- **FR-002**: Canonical/manual ingest MUST accept only caller-supplied `external_id` values that already match the canonical URI grammar.
- **FR-003**: Direct-standard ingest MUST derive canonical `external_id` from trusted institution domain plus original standard `id`.
- **FR-004**: The system MUST preserve the direct-standard payload `id` as `source_metadata.system.original_standard_id` and MUST keep it distinct from `external_id`.
- **FR-005**: `source_id` MUST be deterministic UUID v5 derived from `source|v1|{canonical_external_id}`.
- **FR-006**: `semantic_payload_hash` MUST be the only authoritative replay and conflict comparator.
- **FR-007**: `same canonical external_id + same semantic_payload_hash` MUST replay the authoritative row.
- **FR-008**: `same canonical external_id + different semantic_payload_hash` MUST return conflict.
- **FR-009**: `raw_body_hash` MAY be stored for diagnostics but MUST NOT affect public identity, replay, or conflict semantics.
- **FR-010**: Registration and retrieval responses MUST expose the same public `source_metadata.system` shape: `canonical_id_version`, `ingest_kind`, `semantic_payload_hash`, and `original_standard_id` when present.
- **FR-011**: Public contracts MUST NOT expose migration-only or internal compatibility fields.
- **FR-012**: The runtime MUST NOT include remap lookup, mixed-population reads, migration write denial, rollback tooling, or compatibility aliases.

### Non-functional constraints

- Canonicalization is deterministic, documented, and versioned.
- Validation failures happen before authoritative state creation.
- Observability preserves request correlation and domain-relevant decision reasons without turning internal diagnostics into a migration contract.

## Acceptance criteria

- **AC-001**: Manual registration succeeds only for canonical project-owned URIs.
- **AC-002**: Non-canonical manual identifiers are rejected.
- **AC-003**: Direct-standard registration stores canonical URI `external_id` and preserves `original_standard_id` separately.
- **AC-004**: Registration and retrieval return the same public provenance shape.
- **AC-005**: Deterministic `source_id` generation is stable for equivalent canonical identities.
- **AC-006**: `semantic_payload_hash` alone governs replay and conflict outcomes.
- **AC-007**: No runtime code remains for migration, remap, mixed-population, alias resolution, or rollback behavior.
- **AC-008**: Public docs no longer imply migration support or backward compatibility.

## Success criteria

- **SC-001**: 100% of newly created sources persist canonical `external_id` values.
- **SC-002**: 100% of replays for semantically equivalent submissions resolve to the existing authoritative identifiers.
- **SC-003**: 100% of semantic conflicts for the same canonical identity are rejected.
- **SC-004**: 100% of direct-standard rows preserve `original_standard_id` separately from canonical `external_id`.
- **SC-005**: Reviewers can inspect the repository and find no migration or compatibility-only runtime path.

## Explicit omission

Legacy compatibility is intentionally omitted because the project is still pre-production and no deployed data must be preserved.
