# ADR 0001: Direct Standard Ingest as the Only Authoritative Ingest Surface

**Status**: Accepted
**Date**: 2026-03-21
**Feature**: `001-memory-ingest`

## Context

The original vertical slice mixed a service-owned canonical wrapper model with direct standard ingest. The redesign removes that wrapper model and keeps only direct Open Badges and CLR credential ingest as the authoritative public write surface.

## Decision

1. The authoritative write API accepts supported Open Badges 3.0 and CLR 2.0 credential payloads only.
2. Canonical/manual document ingest is removed from the public authoritative contract.
3. The official standard credential `id` replaces public `external_id` and internal `source_id` as the public authority surface.
4. Search remains a separate non-authoritative projection concern.

## Consequences

- The implementation no longer needs compatibility behavior for wrapper-era registration or retrieval endpoints.
- All downstream artifacts must treat the supported credential document as the primary business object.
