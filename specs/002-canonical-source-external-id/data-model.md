# Data Model: Canonical Source External ID and Direct-Standard Ingest Alignment

## Purpose

Define the domain and persistence implications of canonical `external_id` governance for source registration, provenance, and replay/conflict handling.

## Canonical Entities

### Source

Represents the authoritative stored source after boundary validation, canonical external-id processing, and normalization.

| Field | Type | Required | Notes |
|---|---|---|---|
| `source_id` | UUID v5 | yes | Internal deterministic immutable identifier derived from the canonical source seed |
| `external_id` | string | yes | Canonical project-owned URI only for new governed writes |
| `title` | string | yes | Non-empty after trim |
| `summary` | string | no | Optional summary |
| `document_type` | enum | yes | `text`, `markdown`, or `json` |
| `source_metadata` | JSON object | yes | User metadata plus reserved `system` namespace |
| `created_at` | timestamp | yes | First successful registration time |
| `updated_at` | timestamp | yes | Equal to `created_at` in this immutable slice |

#### Validation Rules

- `source_id` is derived as UUID v5 from a fixed project namespace plus canonical source seed.
- `external_id` for new governed writes must parse as canonical URI grammar `v1`.
- `source_metadata.system` is server-managed and cannot be overwritten by caller payloads.

### Deterministic Source Identifier

Internal identifier for authoritative source storage.

| Field | Type | Required | Notes |
|---|---|---|---|
| `source_id` | UUID v5 | yes | Derived from fixed project namespace plus canonical source seed |
| `source_seed` | string | yes | Stable canonical source identity representation used only for derivation logic |

#### Construction Rules

- `source_id` is derived only after canonical `external_id` is validated or constructed.
- The derivation seed must be stable for canonical/manual and direct-standard registrations that resolve to the same canonical source identity.
- Migration rewrites every persisted `source_id` reference to this scheme.

### Source External ID

Domain value object representing canonical source identity.

| Field | Type | Required | Notes |
|---|---|---|---|
| `standard` | string | yes | Canonical lower-case family token from the vocabulary registry |
| `version` | string | yes | Canonical lower-case version token such as `v1p3` |
| `source_domain` | string | yes | Normalized trusted authority host |
| `object_id_raw` | string | yes | Outer-trimmed producer-local id before encoding |
| `object_id_normalized` | string | yes | Deterministic percent-encoded form used in the URI |
| `canonical_uri` | string | yes | Rendered `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}` |
| `canonical_id_version` | enum | yes | `v1` for this feature |

#### Construction Rules

- Canonical/manual ingest uses `parse_canonical_uri()` and rejects any URI outside the owned namespace or violating grammar.
- Direct-standard ingest uses `from_components()` after resolving trusted domain, vocabulary entry, and object id.
- `object_id` outer trim is the only lossless trimming step. Internal spaces, case, punctuation, and leading zeroes are preserved via percent-encoding when needed.

### Source Identity Provenance

Reserved server-managed metadata stored under `source_metadata.system`.

| Field | Type | Required | Notes |
|---|---|---|---|
| `canonical_id_version` | string | yes | Persisted canonical-id grammar version, initial value `v1` |
| `ingest_kind` | enum | yes | `canonical` or `direct_standard` |
| `original_standard_id` | string | conditional | Present for direct-standard ingest only |
| `semantic_payload_hash` | string | yes | Authoritative replay/conflict comparison value |
| `raw_body_hash` | string | no | Audit-only hash of preserved raw body when retained |
| `direct_standard_profile` | string | no | Optional boundary profile identifier for operator/debug visibility |

#### Invariants

- `original_standard_id` never replaces `external_id`.
- `semantic_payload_hash` is the only authoritative replay/conflict hash.
- `raw_body_hash` may be absent for canonical/manual ingest and must not affect replay or conflict results.

### Direct-Standard Mapping Rule

Describes the family-specific mapping from supported payloads to canonical external-id components.

| Field | Type | Required | Notes |
|---|---|---|---|
| `profile` | string | yes | Boundary profile identifier, e.g. Open Badges or CLR credential envelope |
| `standard` | string | yes | Registry-defined canonical family token |
| `version` | string | yes | Registry-defined canonical version token |
| `source_domain` | string | yes | Trusted host derived from issuer/publisher or ingest context |
| `object_id_raw` | string | yes | Top-level payload `id` after outer trim |
| `object_id_normalized` | string | yes | Percent-encoded canonical object-id segment |
| `original_standard_id` | string | yes | Preserved raw payload `id` |

#### Validation Rules

- Mapping fails if the payload cannot be classified to exactly one supported profile.
- Mapping fails if no trusted source-domain can be derived.
- Mapping fails if `object_id_raw` is empty after trim or exceeds raw/encoded bounds.

## Replay and Conflict Model

### Replay Key

- Primary identity key: canonical `external_id`
- Internal source key: deterministic `source_id`
- Secondary semantic comparator: `source_metadata.system.semantic_payload_hash`

### Decision Table

| Existing `external_id` | Existing semantic hash | Incoming semantic hash | Outcome |
|---|---|---|---|
| none | n/a | n/a | create new source |
| match | match | match | replay existing source |
| match | differs | differs | conflict |

### Raw Body Preservation

- For direct-standard ingest, the first accepted raw UTF-8 body remains authoritative retrieval content.
- Replays do not overwrite stored body content.
- `raw_body_hash` exists only for audit/debug and cannot force conflict or replay by itself.

## Retrieval View Implications

### Source Retrieval View

The retrieval contract continues to return `external_id` as the primary identifier and adds governed provenance fields inside `source_metadata.system`.

| Field | Source | Notes |
|---|---|---|
| `source_id` | `Source` | Internal deterministic UUID v5 |
| `external_id` | `Source` | Canonical URI for new writes |
| `source_metadata.system.canonical_id_version` | provenance | Required for governed rows |
| `source_metadata.system.ingest_kind` | provenance | Existing field retained |
| `source_metadata.system.original_standard_id` | provenance | Present for direct-standard sources |
| `source_metadata.system.semantic_payload_hash` | provenance | Reserved server-managed field |

### Memory Item View

- No structural identifier changes.
- Direct-standard retrieval still returns one `json_document` item whose content is the first committed raw body.

## State Transitions

### Governed Registration Lifecycle

1. Request classified as canonical/manual or direct-standard
2. Canonical external-id validated or derived
3. Provenance metadata assembled
4. Semantic payload hash computed
5. Create/replay decision performed on canonical `external_id` + semantic hash
6. Authoritative source and memory items committed, preserving first raw body on direct-standard replay

## Migration Model

- Existing rows are rewritten so every persisted `source_id` uses the deterministic UUID v5 rule.
- Migration must update `source`, `memory_item`, indexing/outbox, and projection records that reference `source_id`.
- New writes must record `canonical_id_version = v1`.
- Future grammar versions must add new version values rather than reinterpret existing `v1` rows.