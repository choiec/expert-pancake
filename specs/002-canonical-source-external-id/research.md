# Research: Canonical Source External ID and Direct-Standard Ingest Alignment

## Purpose

Record the decisions behind the chosen Option A implementation: a pre-production simplification that deletes all migration and compatibility machinery and keeps only the canonical 002 runtime semantics.

## Decision 1: Persist only canonical project-owned URIs

- Persist governed `external_id` values only as `https://api.cherry-pick.net/{standard}/{version}/{source-domain}:{object-id}`.
- Reject manual inputs that are not already in canonical form.
- Keep canonical standard and version tokens under repository governance.

## Decision 2: Derive direct-standard identity from trusted provenance

- Supported direct-standard payloads derive canonical `external_id` from trusted institution domain plus original standard `id`.
- The original standard `id` is preserved as `source_metadata.system.original_standard_id` and never becomes authoritative `external_id`.
- Requests without a trustworthy domain are rejected instead of rewritten heuristically.

## Decision 3: Use one deterministic internal identity rule

- `source_id` is always UUID v5 derived from `source|v1|{canonical_external_id}`.
- There is no fallback, random, legacy, or migration-only seed branch.
- `canonical_id_version = v1` remains persisted on every authoritative row.

## Decision 4: Keep one authoritative replay comparator

- `semantic_payload_hash` is the only replay and conflict comparator.
- `same canonical external_id + same semantic_payload_hash = replay`.
- `same canonical external_id + different semantic_payload_hash = conflict`.
- `raw_body_hash` remains diagnostics-only and does not affect public identity semantics.

## Decision 5: Delete migration and mixed-population abstractions entirely

- The project has not been deployed to production, so there is no requirement to preserve legacy rows or support cutover tooling.
- Migration dry-run, execute, verify, rollback, remap lookup, write-freeze phases, and alias-resolution branches are intentionally removed instead of retained for hypothetical future use.
- Development and test data are expected to be reset to the canonical 002 model.

## Decision 6: Keep public provenance parity

- Registration and retrieval expose the same `source_metadata.system` shape.
- Public fields are `canonical_id_version`, `ingest_kind`, `semantic_payload_hash`, and `original_standard_id` when present.
- Internal diagnostics stay out of the public contract.

## Decision 7: Keep observability focused on current behavior

- Preserve request correlation, canonical identity context, and domain-relevant decision reasons.
- Remove migration-only diagnostics such as `migration_phase` and `legacy_resolution_path` from runtime behavior and bounded metrics labels.

## Result

The repository now optimizes for present correctness and simplicity:

- one canonical identity model
- one deterministic source identifier rule
- one replay or conflict model
- no dormant compatibility hooks
