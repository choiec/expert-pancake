# Data Model: Memory Ingest Vertical Slice

**Status**: IMPLEMENT-READY

## Purpose

This document defines the canonical data structures for the first memory-ingest slice. The model keeps authoritative persistence protocol-neutral, preserves stable identifiers for retrieval, treats search as a projection-hit surface rather than an authoritative read model, and leaves additive room for future graph projection.

## Canonical Entities

### Source

Represents the external document submitted for ingest after boundary validation and canonicalization.

| Field | Type | Required | Notes |
|---|---|---|---|
| `source_id` | UUID | yes | Server-assigned immutable identifier |
| `external_id` | string | yes | Client-supplied unique idempotency key |
| `title` | string | yes | Non-empty |
| `summary` | string | no | Optional summary text |
| `document_type` | enum | yes | `text`, `markdown`, or `json` |
| `source_metadata` | JSON object | no | User metadata plus reserved `system` namespace |
| `created_at` | timestamp | yes | Set at first successful registration |
| `updated_at` | timestamp | yes | Same as `created_at` in this immutable slice |

#### Validation Rules

- `external_id` must be globally unique.
- `title` must be non-empty after trimming.
- `document_type` must be `text`, `markdown`, or `json` in the canonical model.
- `source_metadata.system` is reserved for server-managed fields such as `canonical_payload_hash` and ingest provenance.
- accepted Open Badges and CLR payloads are persisted as `document_type = json`; the preserved request body is carried by the derived `json_document` memory item rather than a protocol-specific source field.
- every accepted `Source` must produce at least one `MemoryItem` in this slice.

#### Relationships

- One `Source` has one or more `MemoryItem` records.
- One `Source` has zero or more `MemoryIndexJob` rows over time.

### Memory Item

Represents one normalized content unit derived from a `Source`.

| Field | Type | Required | Notes |
|---|---|---|---|
| `urn` | URN string | yes | Deterministic immutable identifier |
| `source_id` | UUID | yes | Foreign key to `Source` |
| `sequence` | integer | yes | Stable zero-based order within source |
| `unit_type` | enum | yes | `paragraph`, `section`, `json_document`, or `metadata_placeholder` |
| `start_offset` | integer | yes | Inclusive UTF-8 byte offset into authoritative canonical content |
| `end_offset` | integer | yes | Exclusive UTF-8 byte offset into authoritative canonical content |
| `version` | string | yes | Initial canonical schema version, e.g. `v1` |
| `content` | string | yes | Immutable canonical content body preserved exactly as accepted |
| `content_hash` | hex string | yes | Hash of canonical content |
| `item_metadata` | JSON object | no | Extension fields plus reserved `system` namespace |
| `created_at` | timestamp | yes | Set at registration commit time |
| `updated_at` | timestamp | yes | Same as `created_at` in this slice |

#### Validation Rules

- `sequence` must be unique per `source_id`.
- `start_offset <= end_offset`.
- `content_hash` is derived and never client-supplied.
- Memory items are immutable after creation in this slice.
- Accepted content is never sanitized, truncated, or rewritten after validation; invalid UTF-8 is rejected before a `MemoryItem` exists.
- Accepted Open Badges and CLR payloads create exactly one `json_document` memory item whose content is the preserved accepted UTF-8 request body and whose offsets are `[0, content.len_utf8_bytes())`.

#### Relationships

- Many `MemoryItem` rows belong to one `Source`.
- One `MemoryItem` may appear in zero or more future graph projections, keyed only by `urn` and `source_id`.

## Canonicalization Rules

- Canonical `text` and `markdown` requests persist the submitted `content` field as authoritative content and normalize from that string directly.
- Accepted Open Badges and CLR requests persist `document_type = json`, store `external_id` from trimmed `id`, store `title` from trimmed `name`, and preserve the accepted UTF-8 request body string exactly as authoritative content.
- For direct-standard ingest, the preserved raw body is exposed through one derived `json_document` memory item rather than a protocol-specific source field.
- Idempotent replay compares a deterministic normalized JSON hash of the validated standard payload, not the preserved raw-body bytes, so formatting-only changes replay to the first authoritative record.
- Retrieval guarantees apply to the authoritative stored memory-item content. For direct-standard ingest, `GET /memory-items/{urn}` returns the preserved first-commit request body exactly as stored.

### Retrieval View / Projection

Represents API-facing read models derived from authoritative rows.

#### Source Retrieval View

| Field | Source | Notes |
|---|---|---|
| `source_id` | `Source` | Canonical identifier |
| `external_id` | `Source` | Echoed for client reconciliation |
| `title` | `Source` | Canonical title |
| `summary` | `Source` | Optional |
| `document_type` | `Source` | Canonical document type |
| `created_at` | `Source` | Registration time |
| `memory_items` | `MemoryItem[]` | Ordered by ascending `sequence` |
| `indexing_status` | derived enum | `queued`, `indexed`, or `deferred` only |

#### Search Projection

Stored in Meilisearch as a denormalized, rebuildable projection.

| Field | Source | Notes |
|---|---|---|
| `urn` | `MemoryItem` | Search hit identifier |
| `source_id` | `MemoryItem` | Filter field |
| `sequence` | `MemoryItem` | Stable ordering inside source |
| `document_type` | `Source` | Filter field; may be `text`, `markdown`, or `json` |
| `content_preview` | derived from `MemoryItem.content` | First 500 characters |
| `content_hash` | `MemoryItem` | Integrity/debug field |
| `created_at` | `MemoryItem` | Sort field |
| `updated_at` | `MemoryItem` | Sort field |

#### Search Hit Response

The public search API returns projection hits, not authoritative memory-item records.

| Field | Source | Notes |
|---|---|---|
| `urn` | `Search Projection` | Projection hit identifier |
| `source_id` | `Search Projection` | Filter field |
| `sequence` | `Search Projection` | Stable order inside source |
| `document_type` | `Search Projection` | Filter field |
| `content_preview` | `Search Projection` | Preview text only; not authoritative full content |
| `score` | Meilisearch | Optional relevance score |

## Supporting Internal Entity

### MemoryIndexJob

Tracks durable indexing work for Meilisearch without making search availability part of the write transaction. Jobs persist source identifiers and status metadata only; projection payloads are rehydrated from authoritative storage during processing.

| Field | Type | Required | Notes |
|---|---|---|---|
| `job_id` | UUID | yes | Server-assigned |
| `source_id` | UUID | yes | Groups a source batch |
| `status` | enum | yes | `pending`, `processing`, `retryable`, `completed`, `dead_letter` |
| `retry_count` | integer | yes | Starts at 0 |
| `last_error` | string | no | Sanitized failure summary |
| `available_at` | timestamp | yes | Next eligible processing time |
| `created_at` | timestamp | yes | Commit time |
| `updated_at` | timestamp | yes | Last status change |

## State Transitions

### Source Registration Lifecycle

1. `received`
2. `validated`
3. `normalized`
4. `persisted`
5. external API status becomes `queued`, `indexed`, or `deferred`

Only `queued`, `indexed`, and `deferred` are externally observable through API responses. Earlier states exist inside the request lifecycle and should emit traces and metrics rather than durable status rows.

### Index Job Lifecycle

1. `pending`
2. `processing`
3. `completed`

Retry branch:

1. `processing`
2. `retryable`
3. `processing`
4. `completed` or `dead_letter`

### Public `indexing_status` Mapping

- `queued`: the authoritative write committed successfully and at least one related outbox job is currently `pending` or `processing`.
- `indexed`: all related projection work is confirmed in Meilisearch and the related outbox job is `completed`.
- `deferred`: authoritative writes succeeded, but search confirmation is blocked by dependency degradation or retry backlog; one or more related outbox jobs are `retryable` or `dead_letter`, or Meilisearch is unavailable at response time.

## Future Graph Expansion Boundary

- Future graph nodes should be keyed by existing canonical identifiers only:
  - source node key: `source_id`
  - memory-item node key: `urn`
- Future graph edges should derive from canonical metadata such as `source_id`, `sequence`, and reserved relation hints in `item_metadata.system.relations`.
- This slice establishes the boundary only; runtime FalkorDB behavior remains a no-op adapter.
- No current entity stores FalkorDB-specific identifiers.