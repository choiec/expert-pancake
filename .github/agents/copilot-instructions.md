# rust Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-03-18

## Active Technologies
- Rust edition 2024 + `axum`, `tokio`, `serde`, `serde_json`, `uuid`, `sha2`, `validator`, `surrealdb`, `meilisearch-sdk`, `tower`, `tracing` (002-canonical-source-external-id)
- SurrealDB as authoritative source storage and replay/conflict gate; Meilisearch as non-authoritative search projection (002-canonical-source-external-id)
- Rust 2024 + axum 0.8.1, tokio 1.44.2, tower-http 0.6.2 (`request-id`, `trace`), tracing 0.1.41, tracing-subscriber 0.3.19 JSON formatter, serde 1.0.219, serde_json 1.0.140, uuid 1.16.0 with v5 support, surrealdb 2.3.3, meilisearch-sdk 0.28.0 (002-canonical-source-external-id)
- SurrealDB authoritative tables `memory_source`, `memory_item`, `memory_index_job`; Meilisearch search projection; FalkorDB unaffected by this feature (002-canonical-source-external-id)

- Rust stable, edition 2024 + `axum`, `tokio`, `tower`, `tower-http`, `serde`, `serde_json`, `validator`, `uuid`, `sha2`, `tracing`, `tracing-subscriber`, `thiserror`, `surrealdb`, `meilisearch-sdk` (001-memory-ingest)

## Project Structure

```text
backend/
frontend/
tests/
```

## Commands

cargo test [ONLY COMMANDS FOR ACTIVE TECHNOLOGIES][ONLY COMMANDS FOR ACTIVE TECHNOLOGIES] cargo clippy

## Code Style

Rust stable, edition 2024: Follow standard conventions

## Recent Changes
- 002-canonical-source-external-id: Added Rust 2024 + axum 0.8.1, tokio 1.44.2, tower-http 0.6.2 (`request-id`, `trace`), tracing 0.1.41, tracing-subscriber 0.3.19 JSON formatter, serde 1.0.219, serde_json 1.0.140, uuid 1.16.0 with v5 support, surrealdb 2.3.3, meilisearch-sdk 0.28.0
- 002-canonical-source-external-id: Added Rust edition 2024 + `axum`, `tokio`, `serde`, `serde_json`, `uuid`, `sha2`, `validator`, `surrealdb`, `meilisearch-sdk`, `tower`, `tracing`
- 002-canonical-source-external-id: Added Rust edition 2024 + `axum`, `tokio`, `serde`, `serde_json`, `uuid`, `sha2`, `validator`, `surrealdb`, `meilisearch-sdk`, `tower`, `tracing`


<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
