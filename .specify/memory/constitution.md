<!--
Sync Impact Report:
- Version change: none → 1.0.0
- Modified principles: N/A (new constitution)
- Added sections: Definition of Done, Non‑Negotiables
- Removed sections: N/A
- Templates requiring updates: None identified (plan/spec/tasks templates are generic and align with constitution principles)
- Follow-up TODOs: None
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
- **Storage responsibility separation**:
  - **SurrealDB** is the authoritative persistence layer for structured memory state and application metadata.
  - **FalkorDB** is the graph/relationship engine for exploring connections and traversals across entities.
  - **Meilisearch** provides text search indexing and retrieval; it is not an authoritative source of truth.
  - Each adapter must clearly document its consistency model and failure modes.
- **1EdTech boundary/adapter rule**: 1EdTech (LTI/QTI/etc.) integration code is an adapter layer sitting on top of the core domain model; it must not pollute core domain types with protocol-specific concepts.

### 3. Code Quality Standards
- **Modular boundaries**: Each crate and module must have a single responsibility; cross‑crate dependencies must be explicit and minimal.
- **No excessive global state**: Global mutable state is prohibited; share state through explicit context objects passed via constructors or request context.
- **Explicit error types**: All fallible operations must return domain/error types that are structured, composable, and mappable to HTTP responses.
- **Logging and tracing consistency**: Use structured logging and tracing consistently (e.g., `tracing` crate conventions); all public request handlers must emit request IDs and key context.
- **Dependency discipline**: Avoid transitive dependency bloat; prefer small, well‑maintained crates.

### 4. Testing Discipline
- **Unit vs integration vs contract**: Unit tests validate isolated logic; integration tests validate end‑to‑end behavior across module boundaries; contract tests validate external API contracts (HTTP/GraphQL/Storage) and adapters.
- **Storage adapter verification**: Each storage adapter (SurrealDB, FalkorDB, Meilisearch) must have a suite of contract tests that verify the adapter meets its declared guarantees (e.g., schema expectations, indexing behavior, query semantics).
- **API contract tests**: Public API surfaces (Axum routes, external integrations) must have automated contract tests asserting request/response shape, status codes, and error semantics.
- **Test data hygiene**: Tests must avoid flaky global state by using isolated fixtures, in‑memory instances, or disposable test databases.

### 5. Performance & Operational Principles
- **Indexing vs query separation**: Write paths (persistence) are responsible for correctness; read paths (search and query) are responsible for latency and scale. Avoid coupling them such that a read‑path failure blocks writes.
- **Minimize blocking work**: Avoid synchronous/blocking calls in async contexts; guard blocking operations behind executors or background tasks.
- **Backpressure, timeouts, retries**: All external calls (DBs, search, HTTP services) must use timeouts and retry policies appropriate to their failure modes; avoid unbounded retries.
- **Resource awareness**: Code must avoid unbounded memory growth and excessive CPU work per request; prefer streaming and bounded buffers for large payloads.

### 6. Security & Privacy Principles
- **Minimal sensitive storage**: Persist only the data required for the product; avoid storing raw user secrets, PII, or large unencrypted payloads unless explicitly justified.
- **Auditability**: Access to sensitive operations and data must be logged at a level that supports post‑incident analysis without exposing secrets.
- **Secrets/config separation**: Secrets must never be checked into source control; configuration and secrets must be injected via environment/config management with clear boundaries.
- **Fail safe**: The system must fail closed for authentication/authorization failures and avoid leaking sensitive information in errors.

### 7. Documentation & Runbooks
- **Spec/Plan/Tasks/ADR discipline**: Every change must be justified by a spec and plan; significant architectural decisions must be captured in ADRs; implementation work must be tracked with tasks.
- **API contracts**: Public APIs must have machine‑readable contract documents (e.g., OpenAPI, JSON Schema) and human‑readable summaries.
- **Onboarding & runbooks**: Key operational playbooks (setup, local development, deployments, incident response) must be documented and kept current.

## Definition of Done
A change is done when all of the following are true:
- Code is implemented in accordance with the relevant spec and plan.
- Automated tests (unit + integration + contract where applicable) pass.
- Code is reviewed and approved via the repository’s standard PR process.
- Documentation is updated (spec, plan, tasks, README, ADRs) for the change.
- Performance regressions have been evaluated for non‑trivial changes affecting runtime behavior.
- Dependencies are updated responsibly (no unchecked upgrades that bypass review).

## Non‑Negotiables
- No changes are merged without automated tests covering the behavior.
- No secret or credential is committed into version control.
- No shortcut around the established handler/service/repository boundary.
- Security and privacy expectations are enforced by default; exceptions require explicit documented risk acceptance.

## Governance
This constitution is the authoritative source for project governance. Every change in scope, architecture, or process must be evaluated against it.

- **Amendments**: Amendments require a documented rationale, a PR with changes to this constitution, and approval by the project maintainers.
- **Versioning**: Governance versions follow semantic versioning: major bumps for breaking governance changes, minor for new principles or policy expansions, patch for clarifications.
- **Compliance review**: Every PR should include a short statement describing how it complies with the relevant constitution principles; reviewers should verify compliance.

**Version**: 1.0.0 | **Ratified**: 2026-03-17 | **Last Amended**: 2026-03-17
