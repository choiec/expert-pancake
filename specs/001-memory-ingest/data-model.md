# Data Model: Memory Ingest with Canonical Source Identity

**Status**: IMPLEMENT-READY

## Purpose

This data model merges the original `001-memory-ingest` authoritative ingest slice with the canonical identity rules that were introduced in `002-canonical-source-external-id` on `main`.

## Source

Represents the authoritative stored source after canonical/manual validation or direct-standard canonical derivation.

| Field | Type | Required | Notes |
|---|---|---|---|
| `source_id` | UUID v5 | yes | Deterministic identifier derived from canonical identity |
| `external_id` | string | yes | Canonical project-owned URI |
| `title` | string | yes | Non-empty after trim |
| `summary` | string | no | Present only when supplied |
| `document_type` | enum | yes | `text`, `markdown`, or `json` |
| `source_metadata` | JSON object | yes | User metadata plus reserved `system` namespace |
| `created_at` | timestamp | yes | First successful registration time |
| `updated_at` | timestamp | yes | Equal to `created_at` in this immutable slice |

### Source invariants

- `source_id` is derived from canonical identity and remains distinct from `external_id`.
- `external_id` always uses the canonical URI grammar under the project-owned namespace.
- `source_metadata.system` is server-managed and carries provenance such as `canonical_id_version`, `ingest_kind`, `semantic_payload_hash`, and `original_standard_id` when present.
- For certification-oriented direct-standard ingest, `source_metadata.system.verification` may summarize envelope, schema, and proof checks that passed before persistence.
- Accepted direct-standard rows persist as `document_type = json`.

## Canonical Source Identity

Represents the canonical public source identity model for this slice.

| Field | Type | Required | Notes |
|---|---|---|---|
| `standard` | string | yes | canonical lower-case token |
| `version` | string | yes | canonical lower-case version token |
| `source_domain` | string | yes | normalized trusted authority host |
| `object_id_raw` | string | yes | outer-trimmed producer-local identity |
| `object_id_normalized` | string | yes | percent-encoded canonical object-id segment |
| `canonical_uri` | string | yes | `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}` |
| `canonical_id_version` | string | yes | `v1` |

### Construction rules

- Canonical/manual ingest validates an already-canonical URI.
- Direct-standard ingest derives canonical identity from trusted domain plus original upstream `id`.
- Semantic replay compares authoritative semantic payload hash, not the preserved raw-body bytes.

## Source Provenance

Reserved server-managed metadata stored under `source_metadata.system`.

| Field | Type | Required | Notes |
|---|---|---|---|
| `canonical_id_version` | string | yes | currently `v1` |
| `ingest_kind` | enum | yes | `canonical` or `direct_standard` |
| `semantic_payload_hash` | string | yes | authoritative replay and conflict comparator |
| `original_standard_id` | string | no | present only for direct-standard rows |
| `verification` | JSON object | no | certification-oriented envelope, schema, and proof validation summary |
| `raw_body_hash` | string | no | diagnostics-only, not public |

### Public surface rules

- Public API responses expose `canonical_id_version`, `ingest_kind`, `semantic_payload_hash`, and `original_standard_id` when present.
- Public API responses may include a `verification` object for direct-standard sources when certification-oriented validation was applied.
- `raw_body_hash` remains internal and is intentionally excluded from public response contracts.

## StandardCredentialEnvelope

Authoritative direct-standard storage record that preserves the official 1EdTech JSON-LD credential envelope without collapsing it into protocol-neutral fields only.

| Field | Type | Required | Notes |
|---|---|---|---|
| `source_id` | UUID | yes | authoritative source reference |
| `family` | enum | yes | `openbadges` or `clr` |
| `version` | string | yes | `v3p0` or `v2p0` |
| `credential_id` | string | yes | original top-level credential `id` |
| `credential_name` | string | yes | canonical mapped title |
| `issuer_id` | string | yes | top-level issuer identifier |
| `subject_id` | string | no | top-level `credentialSubject.id` when present |
| `raw_body` | string | yes | preserved authoritative UTF-8 body |
| `raw_body_hash` | string | yes | diagnostics hash for preserved body |
| `envelope` | JSON object | yes | parsed top-level JSON-LD credential as accepted |
| `normalized_envelope` | JSON object | yes | recursively key-sorted semantic form used for inspection/debugging |
| `credential_subject` | JSON object | yes | copied `credentialSubject` subtree |
| `achievement` | JSON object | no | copied Open Badges achievement subtree when present |
| `credential_schema` | JSON array | yes | copied `credentialSchema` entries |
| `credential_status` | JSON array | no | copied `credentialStatus` entries |
| `evidence` | JSON array | no | copied `evidence` entries |
| `refresh_service` | JSON array | no | copied `refreshService` entries |
| `terms_of_use` | JSON array | no | copied `termsOfUse` entries |
| `proofs` | JSON array | yes | extracted and preserved proof objects |
| `verification` | JSON object | yes | certification-oriented validation summary |
| `created_at` | timestamp | yes | commit time |
| `updated_at` | timestamp | yes | same as `created_at` in this slice |

### Standard-credential-envelope invariants

- Only direct-standard sources create a `StandardCredentialEnvelope`.
- The stored `envelope` preserves the accepted official JSON-LD credential shape as parsed from the authoritative request body.
- `credential_schema` must include the pinned official 1EdTech schema id and `1EdTechJsonSchemaValidator2019` type for the stored family.
- `proofs` must be present and preserve the accepted proof objects that satisfied the certification-oriented boundary checks.

## Memory Item

Represents one normalized content unit derived from a `Source`.

| Field | Type | Required | Notes |
|---|---|---|---|
| `urn` | string | yes | deterministic immutable identifier |
| `source_id` | UUID | yes | foreign key to `Source` |
| `sequence` | integer | yes | stable zero-based order within source |
| `unit_type` | enum | yes | `paragraph`, `section`, `json_document`, or `metadata_placeholder` |
| `start_offset` | integer | yes | inclusive UTF-8 byte offset into authoritative content |
| `end_offset` | integer | yes | exclusive UTF-8 byte offset into authoritative content |
| `version` | string | yes | canonical schema version |
| `content` | string | yes | authoritative stored content |
| `content_hash` | string | yes | derived content hash |
| `item_metadata` | JSON object | no | extension fields |
| `created_at` | timestamp | yes | commit time |
| `updated_at` | timestamp | yes | same as `created_at` in this slice |

### Memory-item invariants

- `urn` is deterministic and immutable.
- `sequence` is unique within a source.
- Accepted direct-standard ingest produces exactly one `json_document` item.
- For direct-standard ingest, the single `json_document` content is the preserved first successful raw body.

## Search Projection

Non-authoritative Meilisearch projection rebuilt from authoritative state.

| Field | Type | Required | Notes |
|---|---|---|---|
| `urn` | string | yes | projection hit id |
| `source_id` | UUID | yes | filter field |
| `sequence` | integer | yes | stable order inside source |
| `document_type` | enum | yes | `text`, `markdown`, or `json` |
| `content_preview` | string | yes | preview-only text |
| `content_hash` | string | yes | integrity/debug field |
| `created_at` | timestamp | yes | sort field |
| `updated_at` | timestamp | yes | sort field |

### Projection invariants

- Search remains non-authoritative.
- Search lag or degradation must not change authoritative replay or retrieval behavior.
- Projection rows may include canonical identity fields for diagnostics, but they do not govern authoritative outcomes.

## MemoryIndexJob

Represents the durable authoritative outbox record that bridges committed source state to non-authoritative search projection work.

| Field | Type | Required | Notes |
|---|---|---|---|
| `job_id` | UUID | yes | stable outbox identifier |
| `source_id` | UUID | yes | authoritative source reference |
| `status` | enum | yes | internal values such as `pending`, `processing`, `retryable`, `completed`, `dead_letter` |
| `retry_count` | integer | yes | bounded retry tracking |
| `created_at` | timestamp | yes | first durable acceptance time |
| `updated_at` | timestamp | yes | latest worker status update |

### Memory-index-job invariants

- A `MemoryIndexJob` is committed in the same authoritative transaction as `Source` and `MemoryItem` rows.
- Public API responses never expose raw internal outbox statuses directly; they summarize indexing state as `queued`, `indexed`, or `deferred`.
- Projection workers rehydrate projection payloads from authoritative `Source` and `MemoryItem` rows rather than treating the outbox as an alternate source of truth.
