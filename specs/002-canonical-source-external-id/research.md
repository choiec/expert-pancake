# Research: Canonical Source External ID and Direct-Standard Ingest Alignment

## Purpose

Record the authoritative planning decisions that close the rollout-blocking ambiguity for canonical source identity, replay semantics, observability, and legacy migration safety.

## Decision 1: Canonical URI v1 remains the single persisted external identity form

- **Decision**: Persist governed `external_id` values only as `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}` and record `canonical_id_version = v1` on every governed row.
- **Rationale**: One persisted form eliminates replay ambiguity, keeps namespace ownership explicit, and makes migration verifiable.
- **Alternatives considered**:
  - Accept multiple persisted URI forms: rejected because replay and conflict semantics would remain unstable.
  - Store grammar version only in docs: rejected because migration and future versioning would become opaque.

## Decision 2: Deterministic source-id seed includes canonical version and canonical URI

- **Decision**: Derive `source_id` from the exact seed string `source|{canonical_id_version}|{canonical_external_id}` for every new and migrated row.
- **Rationale**: Including `canonical_id_version` prevents silent collisions across future grammar revisions while keeping the seed rooted in canonical identity only.
- **Alternatives considered**:
  - Seed from canonical URI alone: rejected because a future grammar version could reuse the same text with different semantics.
  - Seed from raw request content: rejected because mutable formatting and non-canonical aliases are forbidden seed inputs.

## Decision 3: Legacy seed completion uses the same contract as new writes

- **Decision**: Legacy rows first derive `canonical_external_id`, then set `canonical_id_version = v1`, then derive target `source_id`; legacy-only seed branches are prohibited.
- **Rationale**: A single seed contract keeps replay, migration verification, and mixed-population lookups deterministic.
- **Alternatives considered**:
  - Preserve legacy random UUIDs: rejected because the constitution now requires deterministic source identity.
  - Build a separate migration-only seed: rejected because it would create two competing identity regimes.

## Decision 4: The authoritative replay field name is semantic_payload_hash

- **Decision**: Persist and expose `semantic_payload_hash` as the authoritative replay and conflict comparator.
- **Rationale**: The name describes the actual behavior and removes the drift created by `canonical_payload_hash`.
- **Alternatives considered**:
  - Keep `canonical_payload_hash` as the main name: rejected because it no longer reflects the intended semantics and conflicts with the closed design vocabulary.

## Decision 5: canonical_payload_hash is a one-way compatibility alias only

- **Decision**: Read `canonical_payload_hash` only while classifying legacy rows; rewrite it into `semantic_payload_hash` and remove the alias from authoritative storage during migration.
- **Rationale**: This preserves migration compatibility without leaving two authoritative names in steady state.
- **Alternatives considered**:
  - Persist both names indefinitely: rejected because that would preserve drift in code, contracts, and operations.
  - Drop the legacy alias without migration handling: rejected because old rows would become unreadable or unverifiable.

## Decision 6: raw_body_hash is diagnostics-only and stays off the public API surface

- **Decision**: Store `raw_body_hash` only when a raw body exists, never use it for replay or conflict, never expose it in public API responses, and never use it as a metric label.
- **Rationale**: The hash is useful for operator diagnostics and migration audit, but it is not part of public source identity semantics.
- **Alternatives considered**:
  - Expose `raw_body_hash` publicly: rejected because it does not add user-facing value and expands the public contract unnecessarily.
  - Discard it entirely: rejected because it weakens operator diagnostics for replay, migration, and raw-body retention questions.

## Decision 7: Registration and retrieval use one public provenance envelope

- **Decision**: Both registration and retrieval responses expose `source_metadata.system` with `canonical_id_version`, `ingest_kind`, `semantic_payload_hash`, and `original_standard_id` when present.
- **Rationale**: One shape eliminates contract drift and gives clients a single authoritative provenance view.
- **Alternatives considered**:
  - Return provenance only on retrieval: rejected because registration would still hide key canonicalization outcomes.
  - Use different registration and retrieval shapes: rejected because that would preserve drift.

## Decision 8: Observability is internal diagnostics plus public request correlation and provenance

- **Decision**: Structured logs, traces, and bounded metrics are internal diagnostics. Public observability surface is limited to request or trace correlation headers, error-body `request_id`, and the public provenance envelope.
- **Rationale**: This honors the constitution while preventing internal operator diagnostics from becoming unstable public API commitments.
- **Alternatives considered**:
  - Treat logs and metrics as public contract: rejected because that would freeze internal diagnostics and high-cardinality fields.
  - Keep observability undefined at the feature level: rejected because the constitution explicitly requires operational traceability.

## Decision 9: Operator-facing decision diagnostics are mandatory for canonicalization, replay, conflict, migration, and mixed-population lookup

- **Decision**: Emit structured decision events for canonicalization acceptance or rejection, replay classification, conflict detection, migration row classification, migration execution, migration abort, migration verification, and legacy lookup resolution.
- **Rationale**: Operators need to explain each identity-affecting outcome without reconstructing it from raw payloads or storage diffs.
- **Alternatives considered**:
  - Emit only generic request logs: rejected because that would not satisfy handler-level traceability requirements.

## Decision 10: Migration classification uses migratable, consolidate, and unmigratable

- **Decision**: Every legacy row is classified as `migratable`, `consolidate`, or `unmigratable`.
- **Rationale**: These three classes cover safe rewrite, duplicate collapse, and cutover-blocking failure without leaving undefined intermediate states.
- **Alternatives considered**:
  - Binary migrate or skip split: rejected because duplicate canonical identities require explicit consolidation semantics.
  - Best-effort migration with partial leftovers: rejected because the rollout requires zero steady-state ambiguity.

## Decision 11: Same canonical identity with different semantic payloads blocks rollout

- **Decision**: If two legacy rows derive the same canonical `external_id` and different `semantic_payload_hash` values, classify the group as `unmigratable` and abort cutover.
- **Rationale**: The system cannot guess which row is authoritative without violating replay or conflict semantics.
- **Alternatives considered**:
  - Pick the newest row: rejected because it hides semantic conflict.
  - Keep both rows under one canonical identity: rejected because uniqueness and replay rules would fail.

## Decision 12: Same canonical identity with the same semantic payload consolidates

- **Decision**: If multiple legacy rows derive the same canonical `external_id` and the same `semantic_payload_hash`, consolidate them to one deterministic target `source_id` and repoint dependent references.
- **Rationale**: Duplicate logical rows should collapse to one authoritative identity in steady state.
- **Alternatives considered**:
  - Preserve duplicates: rejected because uniqueness and replay semantics would remain ambiguous.

## Decision 13: Mixed legacy and canonical populations exist only during an offline write-frozen window

- **Decision**: Disable registration writes during migration execution. Reads remain available through a remap path from legacy `source_id` values to deterministic targets.
- **Rationale**: Live writes against partially rewritten identity state would create race conditions and ambiguous replay outcomes.
- **Alternatives considered**:
  - Keep writes enabled and dual-write: rejected because it prolongs mixed semantics and increases rollback complexity.
  - Shut down all reads and writes: rejected because read-only service continuity is achievable with remap lookup.

## Decision 14: Rollback uses snapshot restore only

- **Decision**: Rollback is a full snapshot restore of authoritative storage plus projection restore or rebuild. Partial reverse rewrites are prohibited.
- **Rationale**: Partial reverse rewrites create new ambiguity and are harder to verify than restoring a known-good snapshot.
- **Alternatives considered**:
  - Best-effort reverse migration: rejected because it cannot guarantee consistent source, memory-item, and indexing state.

## Decision 15: Dry-run report format is contractually fixed

- **Decision**: The dry-run artifact is a JSON report with required run summary, per-row classification, canonical target identifiers, decision reason, legacy resolution path, provenance summary, dependent-reference counts, and action.
- **Rationale**: Operators need deterministic review material before execution, and tasks need a closed report shape to implement against.
- **Alternatives considered**:
  - Human-only markdown report: rejected because execution gating and automated review need a machine-readable source of truth.

## Decision 16: Normalization regression coverage is a mandatory acceptance gate

- **Decision**: Object-id collisions, source-domain edge cases, and canonical URI golden outputs are not discretionary enhancements; they are mandatory acceptance criteria.
- **Rationale**: Canonical identity safety depends on proving that the normalization rules do not collapse or leak aliases.
- **Alternatives considered**:
  - Add edge cases later as follow-up hardening: rejected because rollout safety depends on them now.

## Residual Risks

- No rollout-blocking design ambiguity remains.
- Remaining risk is execution quality only: implementation must follow the closed plan and preserve the public-versus-internal contract split.

## Status

All planning-time ambiguity for the 002 feature is closed.
