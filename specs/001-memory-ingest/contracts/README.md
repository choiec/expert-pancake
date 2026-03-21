# API Contract

**Status**: IMPLEMENT-READY

This directory contains the merged public HTTP contract for the memory-ingest slice after folding the canonical source identity rules from `002-canonical-source-external-id` into `001-memory-ingest`.

## Contract Scope

- `POST /sources/register`
- `GET /sources/{source-id}`
- `GET /memory-items/{urn}`
- `GET /search/memory-items`
- `GET /health`
- `GET /ready`

## Contract Principles

- Authoritative source identity uses canonical project-owned `external_id` values.
- Internal `source_id` is deterministic and distinct from `external_id`.
- Direct-standard ingest returns canonical identity while preserving original upstream identifiers only as provenance under `source_metadata.system`.
- Direct-standard Open Badges and CLR payloads are pinned to the official 1EdTech 3.0 / 2.0 JSON-LD credential envelopes used by this repository.
- Certification-oriented direct-standard ingest additionally requires pinned `credentialSchema` entries and at least one supported `proof` object before authoritative persistence.
- Replay and conflict decisions use canonical identity plus semantic payload hash, not formatting-only raw-body differences.
- Retrieval is authoritative; search remains a projection-only surface.
- Public contracts do not expose migration-only aliases or compatibility fields.

## Public Provenance Envelope

Registration and retrieval responses expose canonical provenance under `source_metadata.system`:

- `canonical_id_version`
- `ingest_kind`
- `semantic_payload_hash`
- `original_standard_id` when present

Internal diagnostics such as `raw_body_hash` remain non-public.

## Contract File

- `memory-ingest.openapi.yaml`: machine-readable OpenAPI 3.1 contract for the merged slice.
