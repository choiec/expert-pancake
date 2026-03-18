# Feature Specification: Canonical Source External ID and Direct-Standard Ingest Alignment

**Feature Branch**: `002-canonical-source-external-id`  
**Created**: 2026-03-18  
**Status**: Draft  
**Input**: User description: "001-memory-ingest에 대한 후속 변경 스펙을 작성하라. 주제는 canonical source external_id 도입 및 direct-standard ingest 정렬이다."

## Problem & Context

`001-memory-ingest` established the first authoritative ingest flow, but it left an important identity boundary underspecified. The current system treats `external_id` as the main reconciliation key while allowing different meanings for that field across ingest modes. Canonical ingest accepts a caller-supplied `external_id` without enforcing a project-owned grammar, and direct-standard ingest currently stores the payload `id` almost as-is.

That leaves four persistent problems:

- institution, standard, and version context are not guaranteed to appear in the canonical external identity
- equivalent objects can be represented with different raw identifiers, which weakens replay and conflict classification
- the source system's original standard identifier is not clearly separated from the platform's canonical ingest identifier
- the spec, contract, domain model, implementation, and tests can drift because they are not anchored to one explicit external identity grammar

This feature is a follow-up amendment to `001-memory-ingest`. It tightens only the canonical source identity rules. It does not reopen the existing decisions that `source_id` remains a server UUID v4 and that memory item URNs remain deterministic UUID v5-derived identifiers.

## Relationship to 001-memory-ingest

- This feature updates the meaning of canonical `external_id` introduced by `001-memory-ingest`.
- This feature preserves the identifier-role separation established in `001-memory-ingest`: internal `source_id`, canonical `external_id`, and derived memory item URNs remain distinct.
- This feature requires downstream alignment of the existing spec, contracts, domain language, and test fixtures from `001-memory-ingest` so that they no longer imply that a raw standard payload `id` is itself the canonical external identifier.

## Clarifications

### Session 2026-03-18

- Q: Which `source_domain` normalization pipeline should govern canonical identity? → A: Trim whitespace, parse as URL/authority input, remove scheme if present, extract host only, lowercase, remove a single leading `www.`, remove trailing dot, remove port, preserve remaining subdomains exactly, and normalize IDNs to ASCII punycode.
- Q: Which `object_id` normalization policy should govern canonical identity? → A: Trim outer whitespace only, reject empty after trim, preserve internal spaces and case, preserve unreserved characters as-is, percent-encode spaces and reserved or non-unreserved characters using UTF-8 bytes, enforce length limits on both raw and normalized forms, and forbid destructive stripping or collapsing beyond outer trim.
- Q: How should standard and version vocabulary be governed? → A: Store authoritative family and version vocabulary in a versioned repository artifact; persist only fixed canonical tokens such as `qti`, `cc`, `case`, `v3p0`, `v1p3`, and `v1p0`; allow future aliases only as input mappings and never persist them in canonical `external_id`.
- Q: How should original standard identifiers be preserved and surfaced? → A: Fix the reserved provenance field as `source_metadata.system.original_standard_id`, keep `external_id` as the primary retrieval and display identifier, and expose `original_standard_id` only as secondary provenance when present.
- Q: What replay/conflict and rollout policy should govern canonical identity adoption? → A: For new writes, decide replay or conflict by canonical `external_id` plus semantic payload hash, store both semantic and raw-body hashes with raw-body hash used only for audit and debug, preserve first-commit raw body behavior, grandfather legacy records unchanged, and enforce the canonical rule set first for new direct-standard ingest while leaving broader ingest-mode expansion for later planning.
- Default decision: `object_id` maximum size is fixed at 256 UTF-8 bytes after outer trim for the raw form and 1024 ASCII characters for the percent-encoded normalized form; values exceeding either bound are rejected.
- Default decision: current direct-standard family coverage remains the `001-memory-ingest` boundary profiles only, namely Open Badges AchievementCredential-style JSON and CLR credential-style JSON. Registry examples such as `qti`, `cc`, and `case` do not expand direct-standard support in this feature.
- Default decision: for each supported direct-standard family, `standard` and `version` are resolved from the authoritative vocabulary registry entry pinned to that boundary profile; `object_id_raw` is taken from the top-level payload `id`; `object_id_normalized` follows the shared non-lossy encoding rule; and `source_domain` is derived first from a trusted issuer or publisher URL host present in payload metadata, otherwise from the authenticated or configured producer domain bound to the ingest context, otherwise the request is rejected.
- Default decision: the authoritative vocabulary registry file is `specs/002-canonical-source-external-id/contracts/canonical-vocabulary.yaml`, and any registry change requires synchronized updates to spec examples, public contracts, normalization tests, and replay fixtures in the same change set.
- Default decision: rollout for this feature ends after two concurrent write-path changes ship together for new writes only: canonical/manual ingest validates canonical `external_id` as submitted, and direct-standard ingest derives canonical `external_id` under the new rules. No additional ingest-mode expansion or legacy backfill is part of this feature; any future backfill must be proposed as a separate feature.

## Goals

- **G1**: Define one project-owned canonical URI grammar for `external_id` that applies consistently to canonical/manual ingest and direct-standard ingest.
- **G2**: Preserve the original third-party standard identifier separately from the canonical system identifier.
- **G3**: Make replay, idempotency, and conflict outcomes depend on canonical identity plus semantic payload equivalence rather than raw payload formatting.
- **G4**: Introduce explicit versioning for canonical identifier grammar so future grammar changes can be governed without rewriting identifier roles.
- **G5**: Align specification, contract, domain model, and regression tests around the same identifier semantics before implementation changes are made.
- **G6**: State a clear backward-compatibility stance for records created under `001-memory-ingest`.

## Non-Goals

- Changing `source_id` from server-generated UUID v4 to any deterministic or externally derived identifier.
- Changing the existing memory item URN grammar or its deterministic UUID v5 behavior.
- Adopting a destructive normalization rule that removes all special characters from `object_id`.
- Expanding the set of direct-standard payload families beyond Open Badges and CLR, which are the only direct-standard families already in scope for `001-memory-ingest`.
- Performing a mandatory backfill or rewrite of legacy `external_id` values created before this feature is implemented.

## Users & Actors

- **Source Producer**: Registers sources through canonical/manual ingest and needs a stable, reviewable external identifier contract.
- **Standard Ingest Producer**: Submits standard payloads and needs the platform to preserve the original payload identifier without confusing it with the platform's canonical identifier.
- **Downstream Consumer**: Reconciles sources and memory items across replays, conflicts, and retrieval operations using a canonical source identity.
- **Contract and QA Owner**: Maintains API contracts and regression suites and needs one unambiguous identifier meaning across artifacts.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Canonical Identity Is Consistent Across Ingest Modes (Priority: P1)

A source producer registers content through canonical/manual ingest or direct-standard ingest and receives the same kind of canonical `external_id`: a project-owned URI that encodes standard, version, source domain, and object identity.

**Why this priority**: Without one stable identity grammar, replay, contract behavior, and downstream reconciliation remain ambiguous.

**Independent Test**: Can be fully tested by registering one canonical/manual request and one direct-standard request, then verifying both produce `external_id` values that conform to the same canonical URI grammar and preserve original standard provenance separately.

**Acceptance Scenarios**:

1. **Given** a canonical/manual ingest request with a valid canonical `external_id`, **When** registration succeeds, **Then** the returned and persisted `external_id` remains exactly that canonical URI and is treated as the authoritative reconciliation key.
2. **Given** a direct-standard payload whose institution context, standard family, and version can be determined, **When** registration succeeds, **Then** the system stores a canonical URI in `external_id` and stores the payload's original standard `id` separately in server-managed metadata.
3. **Given** a direct-standard payload whose source domain cannot be determined confidently, **When** registration is attempted, **Then** the request is rejected instead of guessing a domain or silently storing the raw payload `id` as `external_id`.

---

### User Story 2 - Replay and Conflict Decisions Follow Canonical Semantics (Priority: P1)

A downstream consumer replays the same logical source and expects formatting-only differences or raw-standard-id representation differences not to create duplicate authoritative records, while true semantic changes for the same canonical source identity are rejected as conflicts.

**Why this priority**: Replay and conflict behavior are the main product consequences of identity semantics; if they stay ambiguous, storage correctness remains unstable.

**Independent Test**: Can be fully tested by submitting the same logical source multiple times with semantically equivalent and semantically different payloads and verifying replay or conflict behavior against the canonical URI plus canonical payload hash.

**Acceptance Scenarios**:

1. **Given** the same canonical `external_id` and the same semantic payload, **When** the source is resubmitted with formatting-only differences, **Then** the system returns the existing authoritative identifiers rather than creating duplicates.
2. **Given** the same canonical `external_id` but a semantically different payload, **When** the source is resubmitted, **Then** the system rejects the request as a conflict.
3. **Given** two standard payloads that refer to the same logical source but use different raw `id` spellings that normalize to the same canonical URI, **When** both are submitted, **Then** replay or conflict decisions are made from the canonical URI and semantic payload comparison rather than the raw `id` string alone.

---

### User Story 3 - Provenance Remains Auditable (Priority: P2)

A standards integrator or operator inspects a registered source and can distinguish the canonical platform identity from the source system's original standard identifier and the grammar version used to create it.

**Why this priority**: Auditability and future migrations depend on explicit provenance, not on inference from raw payloads.

**Independent Test**: Can be fully tested by retrieving a source created through direct-standard ingest and verifying that canonical identity metadata includes grammar version and original standard identifier provenance.

**Acceptance Scenarios**:

1. **Given** a source created through direct-standard ingest, **When** source metadata is retrieved, **Then** it includes `canonical_id_version` and `original_standard_id` in reserved server-managed metadata.
2. **Given** a source created through canonical/manual ingest, **When** source metadata is retrieved, **Then** it includes `canonical_id_version` and indicates that no separate original standard identifier was captured unless explicitly provided as provenance metadata.

---

### User Story 4 - Legacy Records Are Not Rewritten During Rollout (Priority: P3)

A system operator rolls out the new canonical identity rules without forcing immediate migration of previously stored records.

**Why this priority**: Rollout risk stays manageable only if this feature defines a clear compatibility boundary for old data.

**Independent Test**: Can be fully tested by reading legacy sources created before the feature and verifying they remain retrievable while new writes enforce the canonical grammar.

**Acceptance Scenarios**:

1. **Given** a source created before this feature with a non-canonical `external_id`, **When** it is retrieved, **Then** the record remains readable without automatic identifier rewrite.
2. **Given** a new registration after this feature is enabled, **When** the request provides a non-canonical manual `external_id` or a direct-standard mapping cannot produce a canonical URI, **Then** the write is rejected.

### Edge Cases

- A canonical/manual ingest request supplies an `external_id` outside the project-owned namespace; the request must fail validation rather than being rewritten silently.
- A direct-standard payload contains a valid original `id` but no trustworthy institution domain; the request must fail rather than falling back to the raw `id` as canonical identity.
- Two payloads differ only in case, percent-encoding, or punctuation that normalization rules explicitly preserve inside `object_id`; they must not collapse unless the documented normalization rules say they are equivalent.
- An `object_id` contains reserved URI characters such as `/`, `?`, `#`, or spaces; the system must preserve identity through deterministic encoding rather than by dropping those characters.
- A future grammar version is introduced after records already exist with `canonical_id_version = v1`; older records must remain interpretable under the version recorded with each source.
- Legacy records created under `001-memory-ingest` use non-canonical `external_id` values; they remain readable, but new replay behavior for newly written records must not depend on rewriting those historic rows.

### Identity & Canonicalization Constraints *(mandatory when identifiers are affected)*

- The feature uses all three identifier roles and keeps them separate: internal `source_id`, canonical external `external_id`, and derived memory item URNs.
- Canonical `external_id` MUST use the project-owned namespace and grammar `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}`.
- The following examples are normative examples of valid canonical `external_id` values:
  - `https://api.cherry-pick.net/qti/v3p0/kice.re.kr:20240621`
  - `https://api.cherry-pick.net/cc/v1p3/nebooks.co.kr:eng3-ch01`
  - `https://api.cherry-pick.net/case/v1p0/moe.go.kr:6ma01-01`
- `standard` is a lower-case registered family code. It identifies the canonical standards family used for the source and must not be inferred from free-form labels.
- `version` is a lower-case canonical segment in the form `v{major}p{minor}` with an optional additional `p{patch}` when needed. Dots and whitespace are not allowed in the persisted segment. Alias spellings may be accepted only during input mapping, but the persisted `external_id` must always use the registered canonical token.
- `source-domain` is the normalized authority under producer control. Input may arrive as a bare authority or full URL, but canonicalization must extract the host only, lowercase it, remove exactly one leading `www.`, remove any trailing dot and port, preserve remaining subdomains exactly, and normalize IDNs to ASCII punycode. Inputs must still be rejected if the resulting authority is ambiguous or untrusted.
- `object-id` is the producer-local object identity after trimming surrounding whitespace only and applying deterministic UTF-8 percent-encoding for spaces and URI-reserved or otherwise non-unreserved characters. Internal spaces, case, separators, leading zeroes, and semantically meaningful punctuation must be preserved unless a standard-specific rule explicitly defines equivalence. Empty values after outer trim are invalid, the raw form must not exceed 256 UTF-8 bytes, and the encoded form must not exceed 1024 ASCII characters.
- `canonical_id_version` records the grammar version used to construct or validate the canonical URI. This feature introduces `v1` as the first explicit grammar version.
- If a third-party or standard payload ID is accepted, the original ID must be preserved separately as provenance metadata and must not overwrite or masquerade as canonical `external_id`.
- Replay, conflict, and idempotency decisions for newly governed writes must be defined from canonical `external_id` plus semantic canonical payload hash. Raw formatting differences alone must not change identity semantics, and any retained raw-body hash is audit-only rather than authoritative.
- Direct-standard family support in this feature remains limited to the Open Badges and CLR boundary profiles inherited from `001-memory-ingest`; other families may exist in the registry but are not accepted directly at the ingest boundary here.
- Prohibited behavior includes destructive character stripping, silent domain guessing, collapsing internal and external identifiers, and storing raw third-party IDs as canonical `external_id` once canonicalization rules apply.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST treat `external_id` as a canonical source identity under the project-owned namespace `https://api.cherry-pick.net/` and MUST reject new writes whose persisted `external_id` would not conform to the grammar `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}`.
- **FR-002**: The system MUST enforce a standard-segment policy in which `standard` is a registered lower-case family code and `version` is a registered lower-case canonical segment formatted as `v{major}p{minor}` with optional additional `p{patch}` groups. The authoritative family/version registry MUST live in the versioned repository artifact `specs/002-canonical-source-external-id/contracts/canonical-vocabulary.yaml`, shared by spec, contracts, and implementation. Canonical/manual ingest must provide these segments already encoded in `external_id`; direct-standard ingest must derive them deterministically from supported payload family and version rules. Input aliases may be mapped to canonical tokens, but only the canonical tokens may be persisted in `external_id`. Any registry change MUST ship with synchronized updates to spec examples, public contracts, normalization tests, and replay fixtures.
- **FR-003**: The system MUST normalize `source-domain` by trimming whitespace, accepting either a bare authority or URL-like input, removing any scheme, extracting the host only, lower-casing it, removing exactly one leading `www.`, removing any trailing dot and port, and normalizing IDNs to ASCII punycode. The system MUST preserve meaningful subdomains and MUST reject inputs whose resulting authority is untrusted, ambiguous, missing, or dependent on path, query, fragment, or userinfo semantics.
- **FR-004**: The system MUST normalize `object_id` through outer-trim-plus-encode rules that preserve semantic identity. It MUST reject empty values after outer trim, preserve internal whitespace, case, leading zeroes, and meaningful punctuation by default, percent-encode spaces plus reserved or otherwise non-unreserved characters deterministically from UTF-8 bytes, enforce a maximum raw length of 256 UTF-8 bytes and a maximum normalized length of 1024 ASCII characters, and MUST NOT adopt any destructive stripping, punctuation dropping, case-folding, or whitespace-collapsing rule beyond outer trim unless a standard-specific equivalence rule is explicitly documented.
- **FR-005**: The system MUST preserve the raw standard payload identifier separately as `source_metadata.system.original_standard_id` in the server-managed `source_metadata.system` namespace. That field MUST be populated for direct-standard ingest, MUST remain distinct from canonical `external_id`, and MUST be surfaced only as provenance rather than replacing the canonical identifier in retrieval or display semantics.
- **FR-006**: The system MUST extend `source_metadata.system` to include, at minimum, `canonical_id_version`, `ingest_kind`, `original_standard_id` when applicable, the semantic payload hash used for replay and conflict decisions, and any documented audit fields such as raw-body hash that support diagnostics without changing identity semantics. These fields are server-managed and may not be overwritten by caller-provided metadata.
- **FR-007**: Direct-standard ingest MUST stop persisting the payload `id` directly as canonical `external_id`. Instead, it MUST construct canonical `external_id` from the resolved `standard`, `version`, `source-domain`, and `object_id`, preserve the original standard `id` separately, and reject the request if any canonical component cannot be determined confidently. In this feature, direct-standard ingest support remains limited to Open Badges AchievementCredential-style and CLR credential-style JSON payloads inherited from `001-memory-ingest`. For each supported family, `standard` and `version` MUST come from the vocabulary-registry entry pinned to that boundary profile, `object_id_raw` MUST come from the top-level payload `id`, and `source_domain` MUST come from a trusted issuer or publisher URL host in payload metadata or from the configured producer domain bound to the ingest context; otherwise the request MUST be rejected.
- **FR-008**: Canonical/manual ingest MUST continue to accept a caller-supplied `external_id`, but only when that value already matches the canonical URI grammar and the supplied identifier is consistent with the request's declared or implied source context.
- **FR-009**: Replay and idempotency semantics MUST use the persisted canonical `external_id` together with the canonical semantic payload hash. A repeated request with the same canonical `external_id` and equivalent semantic payload MUST return the existing authoritative identifiers even if the raw payload `id` spelling or raw-body formatting differs; a repeated request with the same canonical `external_id` and a semantically different payload MUST fail as a conflict.
- **FR-010**: The feature MUST preserve backward compatibility by leaving previously stored non-canonical `external_id` values untouched. Existing records remain readable and are grandfathered without automatic migration, backfill, or identifier rewrite. Replay and conflict rules introduced by this feature govern new writes within the rollout scope rather than depending on legacy-row normalization. No optional legacy backfill is part of this feature; any future backfill proposal requires a separate spec, plan, and audit strategy.
- **FR-011**: Public contracts and product-facing documentation MUST be updated so they no longer imply that direct-standard payload `id` is the canonical external identifier. Registration, retrieval, and source metadata contracts must show the canonical URI in `external_id` as the primary identity and the preserved raw standard identifier in `source_metadata.system.original_standard_id` as reserved provenance metadata where applicable.
- **FR-012**: The follow-up changes to spec, contracts, tests, and domain language MUST use aligned canonical examples and terminology for grammar, provenance, replay, and conflict semantics. At minimum, the updated artifact set must cover OpenAPI, contract tests, replay and conflict integration tests, normalization edge cases, and domain model descriptions.
- **FR-013**: The feature MUST explicitly preserve the existing identifier-role boundaries from `001-memory-ingest`: `source_id` remains a server UUID v4, memory item URNs remain deterministic derived identifiers, and neither role may be replaced by canonical `external_id`.

### Non-Functional Constraints

- Canonicalization must be deterministic, documented, and versioned so the same valid input always produces the same canonical `external_id` under the same `canonical_id_version`.
- Provenance must remain auditable: the system must make it possible to tell which grammar version produced the canonical URI and what original standard identifier, if any, was preserved.
- Validation failures for invalid canonical identity inputs must fail fast before authoritative state is created.
- Compatibility risk must be bounded by a no-forced-migration rollout posture for legacy rows.

### Key Entities *(include if feature involves data)*

- **Canonical Source Identifier**: The authoritative external identity for a source, stored in `external_id` as a project-owned URI composed of `standard`, `version`, `source-domain`, and `object_id`.
- **Source Identity Provenance**: Reserved server-managed metadata describing how canonical identity was formed, including `canonical_id_version`, `ingest_kind`, `source_metadata.system.original_standard_id` when the source originated from a standard payload, semantic payload hash, and any audit-only raw-body hash fields.
- **Canonical Vocabulary Registry**: The versioned authoritative artifact that lists the allowed standard-family codes, version tokens, and any accepted input aliases that normalize to those persisted canonical values.
- **Canonicalization Component Set**: The validated identity components used to build or validate `external_id`, including normalized standard family, normalized version, trusted source domain, and encoded object identifier.
- **Object Identifier Canonicalization Rule**: The non-lossy policy for `object_id` that trims only outer whitespace, rejects empties, preserves semantic characters and case, percent-encodes reserved or non-unreserved bytes, and applies bounded length validation to both raw and normalized forms.
- **Direct-Standard Mapping Rule**: The family-specific rule set for supported Open Badges and CLR payloads that resolves canonical `standard` and `version` from the pinned registry entry, takes `object_id_raw` from top-level `id`, derives `source_domain` from trusted issuer or publisher authority or configured producer domain, and rejects the request when trustable provenance is unavailable.

### Acceptance Criteria

- **AC-001**: New canonical/manual registrations succeed only when `external_id` already conforms to the canonical URI grammar; non-conforming values are rejected.
- **AC-002**: New direct-standard registrations persist canonical URI `external_id` values and retain the raw standard payload `id` separately in reserved provenance metadata.
- **AC-003**: The normative examples below are accepted as valid canonical identifiers in documentation and contract examples:
  - `https://api.cherry-pick.net/qti/v3p0/kice.re.kr:20240621`
  - `https://api.cherry-pick.net/cc/v1p3/nebooks.co.kr:eng3-ch01`
  - `https://api.cherry-pick.net/case/v1p0/moe.go.kr:6ma01-01`
- **AC-004**: Formatting-only replays and raw-ID representation differences that normalize to the same canonical source identity do not create duplicate authoritative records.
- **AC-005**: Semantic payload changes for the same canonical `external_id` are rejected as conflicts.
- **AC-006**: Source retrieval surfaces the canonical `external_id`, `canonical_id_version`, and direct-standard provenance fields without collapsing them into one identifier.
- **AC-007**: Legacy records created before this feature remain retrievable without automatic rewrite, while new writes enforce the canonical grammar.
- **AC-008**: Updated contracts, regression tests, and product documentation use the same identifier meaning and no longer assert that payload `id` equals canonical `external_id`.

### Assumptions / Open Questions

- This feature aligns identity semantics for the direct-standard families already accepted by `001-memory-ingest`; it does not itself broaden direct-standard family support.
- Retrieval and display may still show provenance details for operators, but this feature now fixes the identity precedence: canonical `external_id` first, original standard `id` second.
- Different raw payload identifiers or raw-body formatting do not by themselves define conflict; semantic payload difference under the same canonical identity does.
- Current direct-standard family scope remains Open Badges and CLR from `001-memory-ingest`; registry examples such as `qti`, `cc`, and `case` remain valid canonical vocabulary examples without expanding direct-standard ingest scope here.
- This feature's rollout is complete once canonical/manual validation and direct-standard derivation rules ship together for new writes; any later ingest-mode expansion or backfill is explicitly out of scope for this spec.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of sources created after this feature is enabled persist `external_id` values that conform to the canonical URI grammar and record `canonical_id_version`.
- **SC-002**: In acceptance and regression validation, 100% of formatting-only replays for the same logical source resolve to the existing authoritative identifiers instead of creating duplicates.
- **SC-003**: In acceptance and regression validation, 100% of semantically conflicting replays for the same canonical `external_id` are rejected as conflicts.
- **SC-004**: 100% of direct-standard sources created after rollout preserve the original standard payload identifier separately from canonical `external_id`.
- **SC-005**: Reviewers can compare the spec, public contract, domain description, and regression examples and find no contradictory definitions of `external_id`, `original_standard_id`, or `canonical_id_version`.

## Known Unknowns

- The registry process for adding new `standard` and `version` codes beyond the currently planned families will be defined during planning.

## Recommended Next Command

- `/speckit.plan` to turn this identifier-governance spec into a technical plan covering API contract changes, domain model updates, migration boundaries, and regression sequencing.
