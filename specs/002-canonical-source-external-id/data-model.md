# Data Model: Canonical Source External ID and Direct-Standard Ingest Alignment

## Purpose

Define the authoritative entities and invariants for the simplified 002-only system.

## Source

Represents the authoritative stored source after canonical identity validation or derivation.

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

- `source_id` is derived from `source|v1|{canonical_external_id}`.
- `external_id` always parses as canonical URI grammar `v1`.
- `source_metadata.system` is server-managed and cannot be overwritten by caller payloads.
- No authoritative row uses alternate identifier fields or transition-only metadata.

## Canonical Source External ID

Domain value object representing canonical source identity.

| Field | Type | Required | Notes |
|---|---|---|---|
| `standard` | string | yes | Canonical lower-case standard token |
| `version` | string | yes | Canonical lower-case version token |
| `source_domain` | string | yes | Normalized trusted authority host |
| `object_id_raw` | string | yes | Outer-trimmed producer-local identity |
| `object_id_normalized` | string | yes | Percent-encoded canonical object-id segment |
| `canonical_uri` | string | yes | `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}` |
| `canonical_id_version` | string | yes | `v1` |

### Construction rules

- Canonical/manual ingest uses `parse_canonical_uri()` and rejects out-of-namespace or invalid grammar.
- Direct-standard ingest uses `from_components()` after resolving trusted domain and normalized object id.
- `object_id` outer trim is the only trimming step. Internal spaces, case, punctuation, and leading zeroes remain semantically meaningful.

## Source provenance

Reserved server-managed metadata stored under `source_metadata.system`.

| Field | Type | Required | Notes |
|---|---|---|---|
| `canonical_id_version` | string | yes | Persisted grammar version, `v1` |
| `ingest_kind` | enum | yes | `canonical` or `direct_standard` |
| `semantic_payload_hash` | string | yes | Authoritative replay and conflict comparison value |
| `original_standard_id` | string | no | Present only for direct-standard rows |
| `raw_body_hash` | string | no | Diagnostics-only field, never public |

### Provenance surface rules

- Public API responses expose `canonical_id_version`, `ingest_kind`, `semantic_payload_hash`, and `original_standard_id` when present.
- `raw_body_hash` remains internal and is omitted from public responses.

## Direct-standard mapping

Defines how supported standard payloads become canonical source identities.

| Field | Type | Required | Notes |
|---|---|---|---|
| `profile` | string | yes | `open_badges_achievement_credential` or `clr_credential` |
| `standard` | string | yes | Canonical registry token |
| `version` | string | yes | Canonical registry token |
| `source_domain` | string | yes | Trusted domain derived from payload metadata |
| `object_id_raw` | string | yes | Top-level payload `id` after outer trim |
| `original_standard_id` | string | yes | Preserved raw payload `id` |

### Mapping rules

- Mapping fails if the payload cannot be classified to exactly one supported profile.
- Mapping fails if no trusted `source_domain` can be derived.
- Mapping fails if `object_id_raw` is empty after trim or exceeds normalization bounds.

## Replay and conflict model

| Existing canonical `external_id` | Existing semantic hash | Incoming semantic hash | Outcome |
|---|---|---|---|
| none | n/a | n/a | create new source |
| match | match | match | replay existing source |
| match | differs | differs | conflict |

### Replay rules

- `semantic_payload_hash` is the only authoritative comparator.
- `raw_body_hash` does not affect replay or conflict decisions.
- The first accepted direct-standard raw body remains the authoritative retrieval body for replays.

## Operational diagnostics

Structured logs and traces may emit the following fields when known:

- `request_id`
- `trace_id`
- `handler`
- `route`
- `method`
- `source_id`
- `canonical_external_id`
- `original_standard_id`
- `canonical_id_version`
- `semantic_payload_hash`
- `raw_body_hash_present`
- `decision_reason`
- `ingest_kind`

### Metrics label set

- `method`
- `route`
- `status_code`
- `document_type`
- `ingest_kind`
- `decision_reason`

Hashes, canonical identifiers, and raw-body values are excluded from metrics labels.

## Explicit omission

This data model does not define transition-only reports, alternate identifier lookup tables, or write-path phase artifacts. Those structures were intentionally removed for the pre-production Option A implementation.
