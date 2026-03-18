<!--
Sync Impact Report:
- Version change: 1.1.0 → 2.0.0
- Modified principles:
  - 3. Identifier Governance & Canonicalization (deterministic UUID v5 source_id required)
  - 5. Testing Discipline (expanded for source_id derivation and migration alignment)
  - Definition of Done (expanded for source_id migration alignment)
  - Non‑Negotiables (expanded for deterministic source_id guardrails)
  - Governance (expanded for source-id seed and migration review)
- Added sections: None
- Removed sections: None
- Templates requiring updates:
  - ✅ .specify/templates/plan-template.md
  - ✅ .specify/templates/spec-template.md
  - ✅ .specify/templates/tasks-template.md
  - ✅ README.md
  - ✅ specs/001-memory-ingest/quickstart.md
  - ⚠ pending review skipped: .specify/templates/commands/ (directory absent)
- Follow-up TODOs:
  - Refresh active source-ingest specs, plans, contracts, tests, and implementation artifacts for deterministic source_id plus canonical external_id governance before code changes are approved.
-->

# Rust AI Memory Architecture Constitution

## Core Principles

### 1. Mission & Non‑functional Goals
The project exists to build a durable, agent-friendly AI memory architecture that enables reliable, auditable, and interoperable long‑term memory for AI systems.

Non‑functional goals:
- **Reliability**: The system must be resilient under realistic load and recover cleanly from failures.
- **Observability**: All behavior must be traceable through structured logs, traces, and metrics.
- **Maintainability**: Code must be modular, well‑tested, and aligned with a clear domain model.
- **Interoperability**: Components must expose stable contracts and support standard integrations (e.g., 1EdTech, REST/JSON). 

### 2. Architecture Boundaries (Axum / Domain / Persistence)
- **Layered handler/service/repository separation**: Axum handlers are thin HTTP adapters that validate input, enforce auth, and delegate to services; services implement domain use cases using plain domain types; repositories abstract storage and query implementations.
- **Canonical domain model first**: Domain types and invariants live in `core_shared` and are treated as the source of truth; storage and integration adapters map to/from this canonical model.
- **Identifier role separation across layers**: Internal server identifiers, external canonical identifiers, and derived memory-item identifiers must remain distinct concerns in handlers, services, repositories, and projections; no layer may collapse them into a single interchangeable field.
- **Storage responsibility separation**:
  - **SurrealDB** is the authoritative persistence layer for structured memory state and application metadata.
  - **FalkorDB** is the graph/relationship engine for exploring connections and traversals across entities.
  - **Meilisearch** provides text search indexing and retrieval; it is not an authoritative source of truth.
  - Each adapter must clearly document its consistency model and failure modes.
- **1EdTech boundary/adapter rule**: 1EdTech (LTI/QTI/etc.) integration code is an adapter layer sitting on top of the core domain model; it must not pollute core domain types with protocol-specific concepts.

### 3. Identifier Governance & Canonicalization
- **Canonical external identity**: Canonical source identity must be represented as a stable URI under the project-owned namespace rooted at `https://api.cherry-pick.net/...`. Project governance owns that namespace and its grammar.
- **Role separation**:
  - `source_id` is the internal deterministic identifier. It must remain distinct from `external_id`, and all persisted source rows must use a UUID v5 derived from a project-governed canonical source seed and fixed namespace.
  - `external_id` is the canonical external identifier used for reconciliation, replay classification, and idempotency. It must persist the canonical URI, not a raw third-party identifier.
  - `memory item URN` is the deterministic immutable identifier for normalized memory items. This amendment does not change memory-item URN generation rules.
- **Source-id derivation**: Deterministic `source_id` generation must be explicit, documented, and based on canonical source identity inputs only. Mutable raw-body formatting, non-canonical aliases, transport metadata, or ambiguous provenance fields must never participate in the UUID v5 seed.
- **Canonicalization rules**: Canonicalization must be explicit, deterministic, documented, and versioned. Server-managed metadata must record the applied `canonical_id_version` or an equivalently documented authoritative field.
- **Source domain normalization**: Canonical URI generation must normalize source-domain inputs through validation, case/format normalization, and deterministic encoding under the project-owned namespace. Unowned or ambiguous domains must be rejected rather than silently rewritten.
- **Object identifier normalization**: Object identity components must preserve semantic information through validation and encoding. Destructive character stripping, lossy collapsing, or aggressive sanitization that can merge distinct identifiers is prohibited.
- **Direct-standard provenance**: When ingest accepts standard payloads, the payload's original standard `id` must be preserved separately in server-managed metadata. It must not overwrite or masquerade as the system canonical `external_id`.
- **Replay and conflict semantics**: Replay, conflict, and idempotency decisions must be defined against the canonical `external_id` plus semantic canonical payload hash. Raw formatting differences alone must not redefine source identity semantics.

Rationale: The existing architecture already separates internal UUIDs, external identifiers, and deterministic memory-item URNs. Elevating canonical external identity to constitution-level governance keeps that separation intact while making boundary identity stable, auditable, namespace-owned, and safe to evolve through explicit versioning rather than ad hoc normalization.

### 4. Code Quality Standards
- **Modular boundaries**: Each crate and module must have a single responsibility; cross‑crate dependencies must be explicit and minimal.
- **No excessive global state**: Global mutable state is prohibited; share state through explicit context objects passed via constructors or request context.
- **Explicit error types**: All fallible operations must return domain/error types that are structured, composable, and mappable to HTTP responses.
- **Logging and tracing consistency**: Use structured logging and tracing consistently (e.g., `tracing` crate conventions); all public request handlers must emit request IDs and key context.
- **Dependency discipline**: Avoid transitive dependency bloat; prefer small, well‑maintained crates.

### 5. Testing Discipline
- **Unit vs integration vs contract**: Unit tests validate isolated logic; integration tests validate end‑to‑end behavior across module boundaries; contract tests validate external API contracts (HTTP/GraphQL/Storage) and adapters.
- **Storage adapter verification**: Each storage adapter (SurrealDB, FalkorDB, Meilisearch) must have a suite of contract tests that verify the adapter meets its declared guarantees (e.g., schema expectations, indexing behavior, query semantics).
- **API contract tests**: Public API surfaces (Axum routes, external integrations) must have automated contract tests asserting request/response shape, status codes, and error semantics.
- **Identifier-governance verification**: Any change to canonical identifier grammar, deterministic `source_id` derivation, replay/conflict semantics, or direct-standard identifier preservation must update and pass aligned spec examples, API/storage contracts, replay tests, provenance tests, normalization edge-case coverage, and migration verification.
- **Test data hygiene**: Tests must avoid flaky global state by using isolated fixtures, in‑memory instances, or disposable test databases.

### 6. Performance & Operational Principles
- **Indexing vs query separation**: Write paths (persistence) are responsible for correctness; read paths (search and query) are responsible for latency and scale. Avoid coupling them such that a read‑path failure blocks writes.
- **Minimize blocking work**: Avoid synchronous/blocking calls in async contexts; guard blocking operations behind executors or background tasks.
- **Backpressure, timeouts, retries**: All external calls (DBs, search, HTTP services) must use timeouts and retry policies appropriate to their failure modes; avoid unbounded retries.
- **Resource awareness**: Code must avoid unbounded memory growth and excessive CPU work per request; prefer streaming and bounded buffers for large payloads.

### 7. Security & Privacy Principles
- **Minimal sensitive storage**: Persist only the data required for the product; avoid storing raw user secrets, PII, or large unencrypted payloads unless explicitly justified.
- **Auditability**: Access to sensitive operations and data must be logged at a level that supports post‑incident analysis without exposing secrets.
- **Secrets/config separation**: Secrets must never be checked into source control; configuration and secrets must be injected via environment/config management with clear boundaries.
- **Fail safe**: The system must fail closed for authentication/authorization failures and avoid leaking sensitive information in errors.

### 8. Documentation & Runbooks
- **Spec/Plan/Tasks/ADR discipline**: Every change must be justified by a spec and plan; significant architectural decisions must be captured in ADRs; implementation work must be tracked with tasks.
- **API contracts**: Public APIs must have machine‑readable contract documents (e.g., OpenAPI, JSON Schema) and human‑readable summaries.
- **Canonicalization governance docs**: The authoritative canonicalization grammar, namespace ownership, `canonical_id_version` policy, provenance preservation rules, and normalization examples must be documented in repository artifacts. Examples and runbooks must use project-owned canonical URI forms when they reference `external_id`.
- **Onboarding & runbooks**: Key operational playbooks (setup, local development, deployments, incident response) must be documented and kept current.

## Definition of Done
A change is done when all of the following are true:
- Code is implemented in accordance with the relevant spec and plan.
- Automated tests (unit + integration + contract where applicable) pass.
- Code is reviewed and approved via the repository’s standard PR process.
- Documentation is updated (spec, plan, tasks, README, ADRs) for the change.
- Performance regressions have been evaluated for non‑trivial changes affecting runtime behavior.
- Changes that affect canonical identifiers, deterministic `source_id` derivation, normalization grammar, or replay/conflict semantics update the relevant spec, plan, contracts, tests, migration artifacts, and implementation artifacts together.
- Dependencies are updated responsibly (no unchecked upgrades that bypass review).

## Non‑Negotiables
- No changes are merged without automated tests covering the behavior.
- No secret or credential is committed into version control.
- No shortcut around the established handler/service/repository boundary.
- No canonical external identifier may be stored outside the project-owned URI namespace once a canonicalization rule applies.
- No destructive identifier normalization policy may strip characters until distinct source identities collapse.
- No change may collapse `source_id` and `external_id` into one field, use non-canonical or mutable seed material for deterministic `source_id`, or alter memory-item URN generation without synchronized artifact review.
- Security and privacy expectations are enforced by default; exceptions require explicit documented risk acceptance.

## Governance
This constitution is the authoritative source for project governance. Every change in scope, architecture, or process must be evaluated against it.

- **Amendments**: Amendments require a documented rationale, a PR with changes to this constitution, and approval by the project maintainers.
- **Versioning**: Governance versions follow semantic versioning: major bumps for breaking governance changes, minor for new principles or policy expansions, patch for clarifications.
- **Identifier-rule changes**: Any amendment that changes canonical identifier grammar, namespace ownership, deterministic `source_id` seed rules, replay/conflict semantics, provenance retention rules, or legacy-row migration posture must include synchronized review of the relevant spec, plan, contracts, tests, migration strategy, and implementation backlog before code changes are approved.
- **Compliance review**: Every PR should include a short statement describing how it complies with the relevant constitution principles; reviewers should verify compliance.

**Version**: 2.0.0 | **Ratified**: 2026-03-17 | **Last Amended**: 2026-03-18
