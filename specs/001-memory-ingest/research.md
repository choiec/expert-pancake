# Research: Schema-Native Standard Credential Registry

**Status**: IMPLEMENT-READY

## Purpose

Record the design decisions that justify replacing the wrapper-era canonical `Source` / `MemoryItem` API with a schema-native standard credential API.

## Decision 1: Keep the handler -> service -> repository split

- **Decision**: Preserve the constitution-required layered architecture and move the redesign into existing `app_server`, `mod_memory`, and `core_infra` boundaries rather than collapsing logic into handlers.
- **Rationale**: The redesign changes the public model, not the architectural rules.
- **Alternatives considered**: Handler-centric rewrites were rejected because they violate the constitution and make replay, validation, and persistence harder to test.

## Decision 2: Support standard credentials only on the authoritative write surface

- **Decision**: Remove canonical/manual document ingest from the public authoritative API and accept only supported Open Badges 3.0 and CLR 2.0 credential payloads.
- **Rationale**: The redesign goal is a schema-native credential API, not a mixed wrapper API.
- **Alternatives considered**: Keeping canonical/manual ingest alongside the new contract was rejected because it preserves the compatibility layer the redesign is meant to remove.

## Decision 3: Use the official credential `id` as the public authoritative identity

- **Decision**: Replace public `external_id` plus internal `source_id` with the official standard credential `id`.
- **Rationale**: This aligns the authoritative identity surface with the schema-native contract and removes service-owned wrapper identifiers.
- **Alternatives considered**: Retaining hidden internal wrapper identifiers while changing only response shapes was rejected because it would keep the old model at the center of the system.

## Decision 4: Persist schema-exact credential documents

- **Decision**: Store authoritative credential documents using only official top-level keys from the detected family schema, preserving the official key names exactly.
- **Rationale**: This makes the stored authoritative shape match the schema-native public contract and prevents storage-only or verification-derived fields from leaking into the canonical document.
- **Alternatives considered**: Custom persisted structs and wrapper metadata were rejected because they recreate a service-owned canonical model.

## Decision 5: Reject unsupported top-level fields at the HTTP boundary

- **Decision**: Treat unsupported top-level keys as invalid input and reject them before persistence.
- **Rationale**: A strict boundary keeps request, storage, and retrieval shapes aligned and avoids silent data loss.
- **Alternatives considered**: Silently discarding unknown top-level fields was rejected because it weakens contract clarity.

## Decision 6: Compute replay equality from normalized authoritative credential JSON

- **Decision**: Build the semantic payload hash from the authoritative schema-exact credential value rather than from the raw body.
- **Rationale**: Replay should be insensitive to formatting but precise about the stored authoritative content.
- **Alternatives considered**: Raw-body hashing was rejected because formatting-only changes would create false conflicts.

## Decision 7: Keep search as a rebuildable projection

- **Decision**: Continue using Meilisearch for search-only projection documents derived from authoritative credential rows, with durable outbox writes in the authoritative transaction.
- **Rationale**: The redesign changes the public authoritative contract, not the separation between authoritative writes and search reads.
- **Alternatives considered**: Making search authoritative or synchronously blocking writes on Meilisearch was rejected because it violates the constitution.

## Decision 8: Remove wrapper-era authoritative retrieval endpoints

- **Decision**: Replace `/sources/*` and `/memory-items/*` authoritative retrieval endpoints with `/credentials/{credential-id}`.
- **Rationale**: The redesign is intentionally breaking and should not preserve compatibility routes that keep the old public model alive.
- **Alternatives considered**: Retaining the old endpoints as aliases was rejected because the user explicitly asked not to keep compatibility paths.

## Decision 9: Keep observability and probe semantics unchanged in intent

- **Decision**: Retain request IDs, structured errors, `/health`, and `/ready`, but realign them around credential authority rather than source or memory-item authority.
- **Rationale**: The constitution still requires observability, auditability, and distinct liveness versus readiness semantics.
- **Alternatives considered**: Simplifying probes during the redesign was rejected because it would remove operational guarantees unrelated to the wrapper model.
