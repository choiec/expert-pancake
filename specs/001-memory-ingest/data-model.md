# Data Model: Schema-Native Standard Credential Registry

**Status**: IMPLEMENT-READY

## Purpose

Define the authoritative and projection entities for the schema-native credential redesign.

## StandardCredentialRecord

Represents the authoritative stored credential after boundary validation and schema-exact filtering.

| Field | Type | Required | Notes |
|---|---|---|---|
| `credential_id` | string | yes | Official standard credential `id` |
| `family` | enum | yes | `open_badges_v3` or `clr_v2` |
| `credential` | JSON object | yes | Authoritative schema-exact credential document |
| `semantic_payload_hash` | string | yes | Replay and conflict comparator |
| `created_at` | timestamp | yes | First successful registration time |
| `updated_at` | timestamp | yes | Equal to `created_at` in this immutable slice |

### StandardCredentialRecord invariants

- `credential_id` is the public authoritative identity.
- `credential` contains only official top-level keys for the detected supported family.
- Official key names remain unchanged, including keys such as `@context`, `credentialSchema`, `credentialStatus`, `credentialSubject`, `proof`, `refreshService`, and `termsOfUse`.
- No derived wrapper fields such as `source_id`, `external_id`, `memory_items`, `verification`, or `source_metadata` are stored inside the authoritative credential document.

## StandardCredentialEnvelope

Authoritative schema-native credential document for supported direct-standard ingest.

| Field | Type | Required | Notes |
|---|---|---|---|
| `@context` | string or JSON array | yes | official JSON-LD context value |
| `awardedDate` | string | no | stored when present |
| `credentialSchema` | JSON object or JSON array | no | stored when present |
| `credentialStatus` | JSON object or JSON array | no | stored when present |
| `credentialSubject` | JSON object | yes | official subject subtree |
| `description` | string or JSON object | no | stored when present |
| `endorsement` | JSON object or JSON array | no | stored when present |
| `endorsementJwt` | string or JSON array | no | stored when present |
| `evidence` | JSON object or JSON array | no | stored when present |
| `id` | string | yes | official credential identifier |
| `image` | string or JSON object | no | stored when present |
| `issuer` | string or JSON object | yes | official issuer value |
| `name` | string or JSON object | yes | required for CLR and supported when present for Open Badges |
| `partial` | boolean | no | CLR-only field stored when present |
| `proof` | JSON object or JSON array | yes | official proof object or objects |
| `refreshService` | JSON object or JSON array | no | stored when present |
| `termsOfUse` | JSON object or JSON array | no | stored when present |
| `type` | string or JSON array | yes | official JSON-LD type value |
| `validFrom` | string | yes | official validity start |
| `validUntil` | string | no | stored when present |

### StandardCredentialEnvelope invariants

- The authoritative public request and response body is a `StandardCredentialEnvelope`.
- Top-level keys are restricted to the supported family's official schema keys.
- Nested content under official keys is preserved as accepted after validation.

## CredentialSearchProjection

Represents a non-authoritative search document rebuilt from authoritative credential rows.

| Field | Type | Required | Notes |
|---|---|---|---|
| `credential_id` | string | yes | authoritative credential identity |
| `family` | enum | yes | projection filter and diagnostics field |
| `name` | string | no | derived search summary from official credential fields |
| `issuer` | string | no | derived search summary from official credential fields |
| `type` | JSON value | yes | official type value copied for filtering |
| `valid_from` | string | no | copied from `validFrom` when present |
| `preview` | string | no | search-only preview text |
| `created_at` | timestamp | yes | sort field |
| `updated_at` | timestamp | yes | sort field |

### CredentialSearchProjection invariants

- Search remains non-authoritative.
- Projection lag or degradation must not change authoritative replay or retrieval behavior.
- Projection documents are derived entirely from authoritative credential rows.

## CredentialIndexJob

Represents the durable authoritative outbox record that bridges committed credential state to non-authoritative search projection work.

| Field | Type | Required | Notes |
|---|---|---|---|
| `job_id` | UUID | yes | stable outbox identifier |
| `credential_id` | string | yes | authoritative credential reference |
| `status` | enum | yes | internal values such as `pending`, `processing`, `retryable`, `completed`, `dead_letter` |
| `retry_count` | integer | yes | bounded retry tracking |
| `created_at` | timestamp | yes | first durable acceptance time |
| `updated_at` | timestamp | yes | latest worker status update |

### CredentialIndexJob invariants

- A `CredentialIndexJob` is committed in the same authoritative transaction as `StandardCredentialRecord`.
- Public API responses do not expose internal outbox states directly.
- Projection workers rehydrate search documents from authoritative credential rows rather than treating the outbox as an alternate source of truth.
