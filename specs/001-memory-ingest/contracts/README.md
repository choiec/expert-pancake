# API Contract

**Status**: IMPLEMENT-READY

This directory contains the schema-native public HTTP contract for the memory-ingest slice after removing the wrapper-era canonical `Source` / `MemoryItem` model.

## Contract Scope

- `POST /credentials/register`
- `GET /credentials/{credential-id}`
- `GET /credentials/search`
- `GET /health`
- `GET /ready`

## Contract Principles

- The authoritative public identity is the official standard credential `id`.
- Successful authoritative write and read responses return schema-native credential documents directly.
- Authoritative credential documents use only official top-level keys from the pinned supported family schema.
- Public authoritative contracts do not expose `source_id`, `external_id`, `urn`, `memory_items`, `source_metadata`, or wrapper-era compatibility fields.
- Replay and conflict decisions use credential `id` plus semantic payload hash, not raw formatting differences.
- Retrieval is authoritative; search remains a projection-only surface.

## Contract File

- `memory-ingest.openapi.yaml`: machine-readable OpenAPI 3.1 contract for the schema-native slice.
