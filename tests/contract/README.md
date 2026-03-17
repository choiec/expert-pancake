# Storage Contract Scope

The current contract files split authoritative persistence evidence into two layers:

- `surreal_source_store_contract.rs`, `surreal_memory_store_contract.rs`, `surreal_retention_contract.rs`, and `indexing_outbox_mapping_contract.rs` are fixture-level proofs. They run against `core_infra::surrealdb::InMemorySurrealDb`, which intentionally mirrors the authoritative uniqueness, replay, rollback, retention, and outbox semantics required by the feature spec.
- Production/runtime code uses `core_infra::surrealdb::SurrealDbService` plus the runtime adapters in `mod_memory::infra`:
  - `RuntimeSurrealSourceRepository`
  - `RuntimeSurrealMemoryRepository`
  - `RuntimeSurrealSourceQueryRepository`
  - `RuntimeSurrealMemoryQueryRepository`

Runtime adapters perform real SurrealDB reads and a transactional source/items/outbox commit path. They rely on bootstrapped Surreal schema and unique indexes created by `SurrealDbService::ensure_memory_ingest_schema()`.

This means:

- Fixture contracts prove the required semantics deterministically in CI.
- Runtime adapters provide the real Surreal-backed execution path used by production bootstrap.
- Live-database verification remains a separate environment-backed concern and is not implied by the fixture-only tests above.
