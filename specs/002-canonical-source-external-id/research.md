# Research: Canonical Source External ID and Direct-Standard Ingest Alignment

## Purpose

Ratify the technical decisions required to implement canonical `external_id` governance together with deterministic UUID v5 `source_id` generation across all persisted rows.

## Decision 1: Introduce a domain-owned canonical external-id value object

- **Decision**: Add a `SourceExternalId` value object to the `mod_memory` domain and make it the single rule owner for parsing, formatting, and validating canonical external identifiers.
- **Rationale**: The current implementation treats `external_id` as an unconstrained string and duplicates meaning between handler logic, service validation, and repository conflict handling. A value object gives one place to enforce project-owned namespace, vocabulary, source-domain normalization, non-lossy object-id encoding, and `canonical_id_version`.
- **Alternatives considered**:
  - Keep `external_id` as a string and validate only at the handler: rejected because repository and service layers would still operate on ad hoc semantics.
  - Parse canonical URIs only in tests/docs: rejected because runtime drift would remain likely.

## Decision 2: Standardize `external_id` and derive `source_id` deterministically from canonical source identity

- **Decision**: Keep `source_id` as a separate internal role, but generate it deterministically as UUID v5 from a project-owned namespace plus canonical source seed. Keep memory-item URNs derived exactly as they are today.
- **Rationale**: The requirement is to eliminate UUID v4 usage for source identity while keeping internal and external identifiers separate. Canonical-source-seed derivation makes `source_id` stable across replay and migration.
- **Alternatives considered**:
  - Keep `source_id` as UUID v4: rejected because it does not satisfy deterministic identity requirements.
  - Re-seed memory-item URNs from canonical `external_id`: rejected because it would change immutable retrieval identifiers.

## Decision 2A: Migrate all existing source rows to the UUID v5 rule in this feature

- **Decision**: The rollout includes a mandatory migration that rewrites existing `source_id` values and every dependent reference so no mixed v4/v5 population remains in authoritative or projection storage.
- **Rationale**: Mixed identity regimes would complicate retrieval, replay, contracts, and future code. The requirement is to stop using v4 entirely, not just for new writes.
- **Alternatives considered**:
  - Grandfather legacy rows: rejected because it leaves v4 in active storage.
  - Dual-read or dual-write compatibility windows: rejected because they prolong mixed semantics and migration risk.

## Decision 3: Canonical URI grammar `v1` is the only accepted persisted form for new writes

- **Decision**: Persist new governed `external_id` values only as `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}` and record `canonical_id_version = v1` in `source_metadata.system`.
- **Rationale**: The grammar encodes the fields required for reconciliation and stays auditable through an explicit version marker.
- **Alternatives considered**:
  - Accept multiple URI forms and normalize later: rejected because replay/conflict behavior would remain ambiguous.
  - Store version only in docs, not data: rejected because future grammar changes would become opaque.

## Decision 4: Preserve original standard payload ids as provenance only

- **Decision**: Direct-standard ingest stores the raw payload `id` in `source_metadata.system.original_standard_id` and never uses it as the persisted canonical `external_id`.
- **Rationale**: This satisfies provenance requirements while keeping canonical identity under project governance.
- **Alternatives considered**:
  - Overwrite `external_id` with raw standard ids and add a second canonical field: rejected because it breaks contract simplicity and weakens idempotency semantics.
  - Drop the original id after canonicalization: rejected because auditability would be lost.

## Decision 5: Replay/conflict semantics use canonical `external_id` plus semantic payload hash

- **Decision**: Use canonical `external_id` plus `semantic_payload_hash` as the authoritative replay/conflict gate. Retain `raw_body_hash` only for audit/debug if useful.
- **Rationale**: Formatting-only differences and raw-standard-id spelling differences should not create new records when they canonicalize to the same logical source. The current normalized raw-JSON hash is not strong enough for that because it still treats raw `id` spelling as semantic.
- **Alternatives considered**:
  - Continue using normalized JSON of the raw request body: rejected because equivalent raw ids could still diverge.
  - Compare only canonical `external_id` and ignore payload content: rejected because true conflicts would be missed.

## Decision 6: Build the semantic hash from a canonical projection, not the raw direct-standard body

- **Decision**: For direct-standard ingest, compute `semantic_payload_hash` from a deterministic canonical projection that uses canonical `external_id` and strips audit-only/raw-provenance noise. For canonical/manual ingest, hash the normalized command shape rather than raw request formatting.
- **Rationale**: This keeps replay semantics tied to meaning rather than transport syntax while still letting retrieval preserve the first committed raw body.
- **Alternatives considered**:
  - Reuse the existing raw normalized JSON hasher for all ingest kinds: rejected for the reasons above.
  - Store only raw-body hash: rejected because it makes formatting differences authoritative.

## Decision 7: Preserve first-commit raw body for direct-standard ingest

- **Decision**: Keep the current behavior where the first accepted direct-standard UTF-8 body remains the authoritative `json_document` content returned by retrieval endpoints.
- **Rationale**: Existing tests and operator expectations depend on retrieval reflecting the first authoritative submission, and the feature request explicitly says raw body preservation should remain under review rather than be removed.
- **Alternatives considered**:
  - Rewrite stored content to canonical JSON: rejected because it would change retrieval semantics and hide provenance.

## Decision 8: Trusted source-domain derivation must be explicit and reject ambiguity

- **Decision**: For direct-standard ingest, derive `source-domain` from a trusted issuer/publisher host in the payload or an ingest-context producer domain. If neither is trustworthy, reject the request.
- **Rationale**: Silent domain guessing would create unstable or unsafe canonical identifiers.
- **Alternatives considered**:
  - Use the raw payload `id` authority blindly: rejected because `id` can be opaque or user-controlled.
  - Fall back to any URL-looking field in the payload: rejected because trust would be unclear.

## Decision 9: Govern standard/version vocabulary through a repository artifact, not runtime remote config

- **Decision**: Publish `contracts/canonical-vocabulary.yaml` as the governance source for canonical family/version tokens and aliases. Implementation should use a static or build-time checked mirror rather than parse YAML on every request.
- **Rationale**: The constitution requires a versioned repository artifact, but request-path performance and failure handling are better if runtime parsing stays out of the hot path.
- **Alternatives considered**:
  - Runtime remote registry lookup: rejected because it adds latency and operational fragility to registration.
  - No registry artifact, only Rust constants: rejected because docs/contracts would stop being authoritative.

## Decision 10: Existing rows are rewritten under a single migration plan

- **Decision**: Rewrite existing rows so `source`, `memory_item`, indexing jobs, search projections, and any other persisted `source_id` references all move to the deterministic UUID v5 scheme in one governed migration.
- **Rationale**: The rollout requirement is global v5 adoption, so migration must be a first-class part of the feature rather than deferred.
- **Alternatives considered**:
  - Leave rows unchanged: rejected because it violates the no-v4 requirement.

## Implementation Implications

- `source_register.rs` needs explicit canonical/manual validation and direct-standard canonicalization paths.
- `RegisterSourceCommand` needs richer validated identity/provenance fields.
- Deterministic `source_id` generation must replace random UUID allocation in application flow.
- Migration planning must cover all persistent `source_id` references, not just the `source` table.
- `SourceSystemMetadata` must evolve from a minimal hash + ingest-kind pair to a provenance envelope.
- Replay/conflict tests must stop treating raw payload ids as canonical identity.

## Residual Risks

- Semantic projection boundaries can be underspecified if the implementation does not clearly document which fields are included or rewritten before hashing.
- Vocabulary drift remains possible if the YAML contract and Rust registry mirror are not parity-tested.

## Status

All planning-time clarifications required for implementation are resolved for this feature.