# ADR 0002: Schema-Native Authoritative Credential API

**Status**: Accepted
**Date**: 2026-03-21
**Feature**: `001-memory-ingest`

## Context

The repository already moved the persisted direct-standard credential record toward a schema-exact shape. The remaining mismatch was the public API, which still exposed wrapper-era concepts such as `source_id`, `external_id`, `memory_items`, and provenance envelopes even though the authoritative payload was already a standard credential.

## Decision

### 1. Authoritative request and response bodies are schema-native

- `POST /credentials/register` and `GET /credentials/{credential-id}` use the supported credential document itself as the authoritative body.
- Successful authoritative responses do not wrap the credential in service-owned envelopes.

### 2. The official credential `id` is the public authority key

- The service retrieves credentials by the official standard `id`.
- Compatibility routes based on source or memory-item identifiers are removed.

### 3. Authoritative credential documents are schema-exact at the top level

- Only official top-level keys defined by the pinned supported family schema are stored and returned.
- Official schema key names remain unchanged.
- Unsupported top-level keys are rejected at the HTTP boundary.

### 4. Replay uses normalized authoritative credential content

- Replay equality is computed from normalized authoritative credential JSON.
- Formatting-only changes replay successfully.
- Semantic differences for the same credential `id` return `409 Conflict`.

## Consequences

- Implementation must delete wrapper-era public response fields and route families instead of preserving compatibility shims.
- Storage, domain models, tests, quickstart flows, and OpenAPI contracts must all center on schema-native credential documents.
