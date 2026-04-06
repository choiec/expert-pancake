---
description: "Use when working in this Rust workspace, especially for Cargo manifests, crate code, integration tests, or repository workflow changes. Covers architecture boundaries, validation commands, and delivery conventions."
applyTo: "Cargo.toml, crates/**/Cargo.toml, crates/**/*.rs, repo_tests/**/*.rs, tests/**/*.rs"
---

# Project Instructions

Last updated: 2026-04-06

## Active Technologies
- Rust edition 2024 workspace with `axum`, `tokio`, `tower`, `serde`, `uuid`, `sha2`, `validator`, `surrealdb`, `meilisearch-sdk`, and `tracing`
- SurrealDB is the authoritative persistence layer; Meilisearch is a non-authoritative search projection; FalkorDB remains the graph adapter boundary

## Architecture
- Read `AGENTS.md`, `.specify/memory/constitution.md`, and the active feature artifacts in `specs/` before implementing spec-driven work
- Keep handlers thin, domain logic in services/application layers, and storage-specific behavior behind repository or adapter boundaries
- Preserve identifier separation: `source_id` is the internal deterministic UUID, `external_id` is the canonical project-owned URI, and memory-item URNs remain separate deterministic identifiers
- Treat direct-standard payload IDs as provenance metadata, not as canonical external IDs

## Project Structure
```text
crates/
  app_server/
  core_infra/
  core_shared/
  mod_*/
repo_tests/
specs/
```

## Commands
- Primary validation: `cargo test --workspace`
- Contract and integration closeout: `cargo test --tests`
- SLO gate: `cargo test --test memory_ingest_slo -- --nocapture`
- Benchmark smoke: `cargo bench --bench memory_ingest_latency --no-run`
- Local server: `cargo run -p app_server`

## Code Style
- Follow the repository constitution for identifier governance, replay semantics, and cross-artifact synchronization
- Update specs, plans, tasks, contracts, tests, and docs together when changing authoritative data behavior or canonical identifier rules
- Prefer crate-local changes with minimal public API expansion; preserve established tracing and error-mapping conventions

## Recent Changes
- 002-canonical-source-external-id: normalized canonical source identity, deterministic `source_id` derivation, and authoritative read/write semantics
- 001-memory-ingest: established the authoritative ingest, indexing, and observability flow across SurrealDB and Meilisearch

## References
- See `README.md` for runtime setup and validation commands
- See `AGENTS.md` for the spec-driven workflow and artifact reading order
