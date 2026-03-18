# Data Model: Canonical Source External ID and Direct-Standard Ingest Alignment

## Purpose

Define the authoritative entities, invariants, migration report model, and operational diagnostics model for canonical source identity and deterministic source-id rollout.

## Canonical Entities

### Source

Represents the authoritative stored source after canonical identity validation or derivation.

| Field | Type | Required | Notes |
|---|---|---|---|
| `source_id` | UUID v5 | yes | Internal deterministic immutable identifier derived from the canonical source seed |
| `external_id` | string | yes | Canonical project-owned URI for every governed row |
| `title` | string | yes | Non-empty after trim |
| `summary` | string | no | Present only when supplied |
| `document_type` | enum | yes | `text`, `markdown`, or `json` |
| `source_metadata` | JSON object | yes | User metadata plus reserved `system` namespace |
| `created_at` | timestamp | yes | First successful registration time |
| `updated_at` | timestamp | yes | Equal to `created_at` in this immutable slice |

#### Invariants

- `source_id` is derived from the exact seed string `source|{canonical_id_version}|{canonical_external_id}`.
- `external_id` for every governed row parses as canonical URI grammar `v1`.
- `source_metadata.system` is server-managed and cannot be overwritten by caller payloads.
- No authoritative row retains `canonical_payload_hash` after migration rewrite.

### Source External ID

Domain value object representing canonical source identity.

| Field | Type | Required | Notes |
|---|---|---|---|
| `standard` | string | yes | Canonical lower-case family token from the vocabulary registry |
| `version` | string | yes | Canonical lower-case version token such as `v1p3` |
| `source_domain` | string | yes | Normalized trusted authority host |
| `object_id_raw` | string | yes | Outer-trimmed producer-local identity |
| `object_id_normalized` | string | yes | Percent-encoded canonical object-id segment |
| `canonical_uri` | string | yes | `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}` |
| `canonical_id_version` | enum | yes | `v1` |

#### Construction Rules

- Canonical/manual ingest uses `parse_canonical_uri()` and rejects out-of-namespace or invalid grammar.
- Direct-standard ingest uses `from_components()` after resolving trusted domain, registry entry, and object-id.
- `object_id` outer trim is the only trimming step. Internal spaces, case, punctuation, and leading zeroes remain semantically meaningful.

### Source Identity Provenance

Reserved server-managed metadata stored under `source_metadata.system`.

| Field | Type | Required | Notes |
|---|---|---|---|
| `canonical_id_version` | string | yes | Persisted grammar version, `v1` |
| `ingest_kind` | enum | yes | `canonical` or `direct_standard` |
| `semantic_payload_hash` | string | yes | Authoritative replay and conflict comparison value |
| `original_standard_id` | string | no | Present only for direct-standard rows |
| `raw_body_hash` | string | no | Stored only when a raw body exists; internal diagnostics only |

#### Public versus internal surface

- Public API responses expose `canonical_id_version`, `ingest_kind`, `semantic_payload_hash`, and `original_standard_id` when present.
- `raw_body_hash` remains internal and is omitted from public API responses.
- Legacy `canonical_payload_hash` is not part of the public or authoritative model.

### Deterministic Source Identifier

Internal identifier contract for authoritative source storage.

| Field | Type | Required | Notes |
|---|---|---|---|
| `source_id` | UUID v5 | yes | Derived from fixed namespace plus canonical source seed |
| `source_seed` | string | yes | `source|{canonical_id_version}|{canonical_external_id}` |

#### Seed Rules

- `canonical_id_version` is always part of the seed.
- Canonical external identity is the only semantic source of seed material.
- Raw body, raw payload formatting, transport headers, and provenance aliases never enter the seed.

### Direct-Standard Mapping Rule

Defines the mapping from supported payloads to canonical URI components.

| Field | Type | Required | Notes |
|---|---|---|---|
| `profile` | string | yes | `open_badges_achievement_credential` or `clr_credential` |
| `standard` | string | yes | Registry-defined canonical family token |
| `version` | string | yes | Registry-defined canonical version token |
| `source_domain` | string | yes | Trusted host derived from payload metadata or ingest context |
| `object_id_raw` | string | yes | Top-level payload `id` after outer trim |
| `object_id_normalized` | string | yes | Percent-encoded canonical segment |
| `original_standard_id` | string | yes | Preserved raw payload `id` |

#### Validation Rules

- Mapping fails if the payload cannot be classified to exactly one supported profile.
- Mapping fails if no trusted `source_domain` can be derived.
- Mapping fails if `object_id_raw` is empty after trim or exceeds raw or encoded bounds.

## Replay and Conflict Model

### Replay Key

- Primary identity key: canonical `external_id`
- Internal source key: deterministic `source_id`
- Semantic comparator: `source_metadata.system.semantic_payload_hash`

### Decision Table

| Existing canonical `external_id` | Existing semantic hash | Incoming semantic hash | Outcome |
|---|---|---|---|
| none | n/a | n/a | create new source |
| match | match | match | replay existing source |
| match | differs | differs | conflict |

### Raw Body Retention

- The first accepted direct-standard raw body remains the authoritative retrieval body.
- Replays do not overwrite retained body content.
- `raw_body_hash` exists only for audit and diagnostics.

## Migration Model

### Legacy Row Classification

| Classification | Definition | Target action |
|---|---|---|
| `migratable` | canonical identity, deterministic seed, semantic hash, and reference rewrite are complete | rewrite row and references |
| `consolidate` | two or more rows resolve to one canonical identity and one semantic hash | repoint references to surviving target row and remove duplicates |
| `unmigratable` | canonical identity, semantic equivalence, or reference coverage is incomplete or conflicting | abort cutover |

### Migration Row Report

| Field | Type | Required | Notes |
|---|---|---|---|
| `legacy_source_id` | UUID | yes | Original source identifier |
| `candidate_source_id` | UUID v5 | yes | Deterministic target identifier |
| `legacy_external_id` | string | yes | Original stored external identifier |
| `canonical_external_id` | string | yes | Candidate canonical URI |
| `canonical_id_version` | string | yes | `v1` |
| `original_standard_id` | string | no | Preserved provenance when present |
| `semantic_payload_hash` | string | yes | Authoritative semantic comparator |
| `raw_body_hash_present` | bool | yes | Signals diagnostics-only raw-body hash availability |
| `classification` | enum | yes | `migratable`, `consolidate`, `unmigratable` |
| `decision_reason` | string | yes | One taxonomy value from the plan |
| `legacy_resolution_path` | enum | yes | `legacy_only`, `remapped_source_id`, `canonical_only`, or `shadow_duplicate` |
| `dependent_reference_counts` | object | yes | Counts for `memory_item`, `memory_index_job`, and search projections |
| `action` | enum | yes | `rewrite`, `consolidate`, or `abort` |

### Mixed-Population Lookup State

| Field | Type | Required | Notes |
|---|---|---|---|
| `legacy_source_id` | UUID | yes | Pre-migration identifier |
| `target_source_id` | UUID v5 | yes | Deterministic target identifier |
| `legacy_resolution_path` | enum | yes | Lookup path used during migration window |
| `migration_phase` | enum | yes | `dry_run`, `rewrite`, `verification`, `cutover`, or `rollback` |

#### Invariants

- Registration writes are disabled during `rewrite` and `verification`.
- Retrieval by legacy `source_id` resolves through the remap state during the migration window.
- Steady state after `cutover` has no legacy-only authoritative rows.

## Operational Diagnostics Model

### Decision Diagnostic Event

Internal structured log or trace event emitted for canonicalization, replay, conflict, migration, or legacy lookup resolution.

| Field | Type | Required | Notes |
|---|---|---|---|
| `request_id` | string | yes | Request correlation key |
| `trace_id` | string | yes | W3C trace correlation key when present |
| `handler` | string | yes | Handler or offline command name |
| `route` | string | yes | HTTP route or offline command route token |
| `method` | string | yes | HTTP method or command verb |
| `source_id` | UUID | no | Internal source identifier after resolution |
| `canonical_external_id` | string | no | Canonical identity under evaluation |
| `original_standard_id` | string | no | Direct-standard provenance |
| `canonical_id_version` | string | yes | `v1` |
| `semantic_payload_hash` | string | no | Authoritative comparator |
| `raw_body_hash` | string | no | Diagnostics-only raw-body hash |
| `raw_body_hash_present` | bool | yes | Signals hash availability |
| `migration_phase` | string | yes | Current migration phase or `steady_state` |
| `legacy_resolution_path` | string | yes | Lookup path taken |
| `decision_reason` | string | yes | One closed taxonomy value |
| `ingest_kind` | string | yes | `canonical` or `direct_standard` |

### Metrics Label Set

- `method`
- `route`
- `status_code`
- `document_type`
- `ingest_kind`
- `migration_phase`
- `decision_reason`

Hashes and identifiers are excluded from metrics labels.
