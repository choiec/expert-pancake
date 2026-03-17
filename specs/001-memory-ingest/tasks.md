# Tasks: Source Document Ingestion and Memory Item Normalization

**Input**: Design documents from `/specs/001-memory-ingest/`
**Status**: IMPLEMENT-READY
**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/memory-ingest.openapi.yaml`, `quickstart.md`

## Task Generation Assumptions

- This task list targets the first implementation-ready vertical slice for the feature branch `001-memory-ingest`.
- Tests are included because the spec and constitution explicitly require integration, API contract, and adapter contract coverage.
- The current repository is skeletal, so early tasks intentionally normalize the Rust workspace and add the missing `mod_memory` crate boundary described in the plan.
- Search remains non-authoritative: Meilisearch work is required for this slice, but write-path success still depends only on SurrealDB.
- Every published public endpoint and published status code in the OpenAPI contract must map to an explicit contract-testing task; implicit coverage is not sufficient for this slice.
- Artifact-level ambiguity is closed. Remaining risk is implementation verification only, and task coverage must make standard-payload validation, replay hashing, outbox mapping, and performance gates explicit rather than implied.

## Dependency Notes

- Phase 1 establishes a usable Rust workspace and local infra defaults.
- Phase 2 is blocking for all user stories because config, bootstrap, error mapping, client setup, and health probes are shared prerequisites.
- User Story 1 is the MVP slice and must complete before validating retrieval stories against real persisted data.
- User Story 2 depends on the repositories and persisted model introduced by User Story 1.
- User Story 3 depends on User Story 1 persistence and can proceed in parallel with late User Story 2 work once shared query repositories exist.
- User Story 4 depends on the indexing outbox written during User Story 1 and on app bootstrap hooks from Phase 2.
- Polish tasks depend on all implemented routes and adapters being present.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Normalize the Rust workspace and create the crate boundaries required by the plan.

- [X] T001 Align Rust workspace manifest and scaffold the memory ingest crate in Cargo.toml, crates/mod_memory/Cargo.toml, crates/mod_memory/src/lib.rs, crates/mod_memory/src/application/mod.rs, crates/mod_memory/src/domain/mod.rs, crates/mod_memory/src/infra/mod.rs
Outcome: The repository becomes a proper Cargo workspace with a dedicated `mod_memory` crate matching the plan's application/domain/infra split.
Dependencies: None.
Relevant inputs: plan.md Architecture / Components; research.md Decision 1; user requirement `Rust workspace / crate 구조 정리`.
Constraints: Preserve existing crate boundaries; do not move unrelated feature crates; use edition 2024 throughout.
Done-when: `cargo metadata` can resolve the workspace members and the new `mod_memory` crate is an addressable package target.
Traceability: Foundation for US1-US4; FR-014; constitution Architecture Boundaries.

- [X] T002 Configure shared crate dependencies and local infrastructure defaults in crates/app_server/Cargo.toml, crates/core_shared/Cargo.toml, crates/core_infra/Cargo.toml, crates/mod_memory/Cargo.toml, docker-compose.yaml
Outcome: All participating crates declare the dependencies from the plan, and local SurrealDB/Meilisearch startup matches the quickstart environment.
Dependencies: T001.
Relevant inputs: plan.md Technical Context; quickstart.md Proposed Environment Variables and Start Infrastructure.
Constraints: Keep dependency additions limited to the stack named in the plan; Meilisearch must remain optional for write-path success.
Done-when: Cargo manifests declare the required libraries, and `docker-compose.yaml` exposes SurrealDB and Meilisearch with the documented local ports and credentials.
Traceability: US1-US4 enablement; FR-008; FR-011; NC-007.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Implement shared runtime concerns that every story depends on.

**Critical**: No user-story endpoint work should begin until this phase is complete.

- [X] T003 Implement environment/config loading in crates/app_server/src/config.rs and crates/core_infra/src/setup.rs
Outcome: Runtime configuration is loaded from environment variables into typed config structs for HTTP, SurrealDB, Meilisearch, limits, and timeouts.
Dependencies: T002.
Relevant inputs: quickstart.md Proposed Environment Variables; plan.md Storage / State / API Decisions.
Constraints: Secrets must remain environment-injected; body size and timeout settings must expose the 10 MB and 30 second limits from the spec.
Done-when: The app can construct validated config objects without hard-coded credentials, and missing required settings produce structured startup failures.
Traceability: FR-002; FR-011; NC-001; constitution Secrets/config separation.

- [X] T004 [P] Create the common domain error model in crates/core_shared/src/error.rs and crates/core_shared/src/lib.rs
Outcome: Canonical error types exist for validation, conflict, timeout, not-found, storage, and search-degraded cases, ready for HTTP mapping and service composition.
Dependencies: T001.
Relevant inputs: spec.md FR-007, FR-012; plan.md Error Contract; repo memory fact about explicit error types.
Constraints: Errors must be structured and protocol-neutral; no HTTP-specific status codes in domain errors.
Done-when: Shared error enums/structs cover all contract-level failures needed by the slice and are exported from `core_shared`.
Traceability: US1-US4; FR-007; FR-012; constitution Explicit error types.

- [X] T005 Implement app bootstrap, router assembly, and shared state wiring in crates/app_server/src/main.rs and crates/app_server/src/state.rs
Outcome: The Axum server boots with typed application state, config, repository/indexing dependencies, and background-task handles.
Dependencies: T002, T003.
Relevant inputs: plan.md Request Flow and Handler sections; research.md Decision 1 and Decision 10.
Constraints: Handlers must stay thin; no business logic in `main.rs`; bootstrap must support later worker startup.
Done-when: `main.rs` builds the app from injected dependencies instead of printing a placeholder message.
Traceability: US1-US4; G1; G6; constitution Layered handler/service/repository separation.

- [X] T006 Implement request-id, trace-context, and HTTP error mapping middleware in crates/app_server/src/middleware.rs
Outcome: Every request gets request correlation, W3C trace propagation, structured logging hooks, and canonical JSON error responses.
Dependencies: T003, T004, T005.
Relevant inputs: spec.md FR-010, FR-012, FR-013; plan.md Handler responsibilities and Error Contract.
Constraints: Do not log raw content payloads or arbitrary metadata; preserve protocol-neutral core errors and map them only at the HTTP boundary.
Done-when: Route handlers can return service errors and receive consistent JSON error bodies with `error_code`, `message`, `details`, `timestamp`, and `request_id`.
Traceability: US1-US4; FR-010; FR-012; FR-013; NC-011.

- [X] T007 [P] Implement SurrealDB and Meilisearch client bootstrap plus readiness probes in crates/core_infra/src/surrealdb.rs, crates/core_infra/src/meilisearch.rs, crates/core_infra/src/setup.rs
Outcome: Infrastructure adapters can connect with bounded timeouts, perform lightweight readiness checks, and expose reusable clients to the application.
Dependencies: T002, T003.
Relevant inputs: plan.md Repository and Indexing Adapter sections; research.md Decision 3, Decision 7, Decision 8.
Constraints: SurrealDB readiness must represent write-path availability; Meilisearch availability must be reportable as degraded without blocking startup.
Done-when: The app can create typed SurrealDB and Meilisearch clients and independently report database/search component status.
Traceability: US1-US4; FR-008; FR-011; NC-007.

- [X] T008 Implement health and readiness handlers with router wiring in crates/app_server/src/handlers/health.rs, crates/app_server/src/handlers/mod.rs, crates/app_server/src/router.rs
Outcome: `/health` is available as a local-only liveness probe, and `/ready` is available as a dependency-aware readiness probe with explicit service/database/search status.
Dependencies: T005, T006, T007.
Relevant inputs: spec.md User Story 4 context, FR-011; plan.md REST Endpoints and Readiness semantics.
Constraints: `/health` must stay fast, return 200 without probing external dependencies, and expose service-local status only; `/ready` must fail when the SurrealDB write path is unavailable and may report degraded search while still returning 200.
Done-when: The router exposes both endpoints with the JSON shape defined in the OpenAPI contract and plan.
Traceability: G6; FR-011; AC-O2.

- [X] T034 [P] Add OpenAPI-backed contract validation for GET /health and GET /ready in tests/contract/health_readiness_contract.rs
Outcome: The probe endpoints' distinct schemas and status-code matrices are pinned to the published contract before implementation proceeds.
Dependencies: T008.
Relevant inputs: contracts/memory-ingest.openapi.yaml `/health`, `/ready`; spec.md FR-011, NC-010.
Constraints: Assert `200` for `/health`, assert `200/503` for `/ready`, and verify `/health` uses the local-only schema while `/ready` uses the dependency-aware readiness schema.
Done-when: Contract tests fail until probe semantics, payload shapes, and status codes match the published OpenAPI document exactly.
Traceability: FR-011; NC-010; constitution API contract tests.

**Checkpoint**: Foundation complete. Story work can now proceed with stable config, bootstrap, shared errors, and readiness probes.

---

## Phase 3: User Story 1 - Source Document Registration (Priority: P1) 🎯 MVP

**Goal**: Accept canonical/Open Badges/CLR JSON, normalize it into canonical domain entities, persist authoritative source and memory items transactionally, and enqueue indexing work.

**Independent Test**: Call `POST /sources/register` with valid canonical, Open Badges, and CLR payloads plus replay/conflict/error variants, then verify the returned identifiers, `indexing_status`, and authoritative persistence state.

### Tests for User Story 1

- [ ] T009 [P] [US1] Add OpenAPI-backed contract validation for POST /sources/register in tests/contract/register_source_contract.rs and tests/fixtures/register_source/*.json
Outcome: The registration endpoint's accepted payload families, success codes, and error responses are pinned to the published API contract.
Dependencies: T008.
Relevant inputs: contracts/memory-ingest.openapi.yaml `/sources/register`; spec.md FR-001, FR-002.
Constraints: Cover canonical payload, Open Badges shape, CLR shape, `200`, `201`, `400`, `408`, `409`, `413`, and `503` response contracts without depending on implementation internals; assert `document_type = json`, `unit_type = json_document`, and the public `indexing_status` vocabulary for direct standard success responses.
Done-when: Contract tests fail against the current placeholder app and define the exact registration surface, direct-standard response shapes, and status-code matrix that implementation must satisfy.
Traceability: US1; FR-001; FR-002; constitution API contract tests.

- [ ] T010 [P] [US1] Add integration coverage for create, idempotent replay, and conflict paths in tests/integration/register_source_flow.rs
Outcome: End-to-end registration behavior is specified against real app wiring and authoritative persistence semantics.
Dependencies: T008.
Relevant inputs: spec.md User Story 1 acceptance scenarios; plan.md Idempotency and Transactions decisions.
Constraints: Use isolated fixtures/test databases; verify no duplicate rows are visible after replay or conflict attempts for canonical requests and that the persisted registration response includes the expected public `indexing_status`.
Done-when: Integration tests demonstrate failing expectations for `201 Created`, `200 OK` replay, and `409 Conflict` behavior for canonical requests before implementation.
Traceability: US1; AC-F1; AC-F2; NC-006.

- [ ] T042 [P] [US1] Add integration coverage for Open Badges and CLR success, replay, and conflict paths in tests/integration/register_source_standard_flow.rs and tests/fixtures/register_source/standards/*.json
Outcome: End-to-end direct-standard ingest behavior is specified against real app wiring for the supported payload families.
Dependencies: T008.
Relevant inputs: spec.md FR-001, FR-002, FR-003, AC-F7; plan.md Standard-payload canonicalization and Idempotency decisions.
Constraints: Cover Open Badges success, CLR success, formatting-only replay success, conflicting replay `409`, `document_type = json`, a single `json_document` memory item, exact preserved-content retrieval, and public `indexing_status` behavior.
Done-when: Integration tests fail until supported standard payloads persist one authoritative `json_document` item, replay through normalized JSON hashing, and reject semantic conflicts without duplicate state.
Traceability: US1; FR-001; FR-002; FR-003; AC-F2; AC-F3; AC-F7.

- [ ] T043 [P] [US1] Add boundary adapter tests for pinned-schema failure and shape-valid-but-unmappable payload handling in crates/app_server/src/handlers/source_register.rs and tests/contract/register_source_standard_errors.rs
Outcome: The HTTP boundary's supported-standard validation and canonical mapping rules are pinned independently from storage behavior.
Dependencies: T008.
Relevant inputs: spec.md FR-001, FR-002; plan.md Request / Response Shape and Failure Modes; contracts/memory-ingest.openapi.yaml `/sources/register`.
Constraints: Cover pinned JSON Schema validation failure -> `400`, shape-valid but unmappable standard payload -> `400 INVALID_STANDARD_PAYLOAD`, and keep these checks at the HTTP adapter boundary without invoking persistence.
Done-when: Unit/contract coverage fails until schema-invalid and unmappable standard payloads are distinguished correctly and returned as structured `400` responses.
Traceability: US1; FR-001; FR-002; AC-F7; constitution API contract tests.

- [ ] T035 [P] [US1] Add SurrealDB storage-adapter contract coverage in tests/contract/surreal_source_store_contract.rs, tests/contract/surreal_memory_store_contract.rs, and tests/contract/surreal_retention_contract.rs
Outcome: The authoritative storage adapter's guarantees are pinned before implementation details diverge.
Verification note: The current checked-in contract tests pin fixture-level semantics via `InMemorySurrealDb`; runtime client-backed SurrealDB semantics are implemented in the production adapters and documented in `tests/contract/README.md`.
Dependencies: T007.
Relevant inputs: constitution Storage adapter verification; plan.md Storage adapter contract tests; spec.md FR-002, FR-004, NC-006, NC-016.
Constraints: Assert uniqueness constraints, transactional rollback, idempotent replay semantics, write-path readiness probe behavior, and the no-TTL/no-purge retention baseline for authoritative records.
Done-when: Adapter contract tests fail until the SurrealDB implementation proves the authoritative persistence guarantees required by the spec and constitution.
Traceability: US1; FR-002; FR-004; NC-006; NC-016; constitution Storage adapter verification.


### Implementation for User Story 1

- [ ] T011 [P] [US1] Define the Source aggregate and canonical registration command in crates/mod_memory/src/domain/source.rs and crates/mod_memory/src/application/register_source.rs
Outcome: Canonical source types, idempotency-relevant fields, and registration command structures exist independently of HTTP DTOs.
Dependencies: T004.
Relevant inputs: data-model.md Source; spec.md FR-001, FR-014; plan.md Domain Model and Application / Service sections.
Constraints: Keep protocol-specific Open Badges/CLR concerns out of canonical domain types while still representing `document_type = json`, ingest provenance, and canonical payload hashes for direct-standard ingest.
Done-when: The domain model can represent a validated canonical source registration, including direct-standard `json` sources and replay hashes, without referencing Axum, OpenAPI, or storage DTOs.
Traceability: US1; FR-001; FR-014; constitution Canonical domain model first.

- [ ] T012 [P] [US1] Define the MemoryItem aggregate and normalization rules in crates/mod_memory/src/domain/memory_item.rs and crates/mod_memory/src/domain/normalization.rs
Outcome: Canonical memory-item entities and deterministic normalization logic exist for text, markdown, and empty-content placeholder cases.
Dependencies: T004.
Relevant inputs: data-model.md Memory Item; spec.md FR-003; research.md Decision 5 and Decision 6.
Constraints: Preserve stable `sequence`, deterministic URN seeds, immutable content, placeholder behavior for empty content, and a single full-body `json_document` path for direct-standard ingest with UTF-8 byte offsets.
Done-when: Domain normalization can derive ordered `MemoryItem` values from canonical source content with offsets, hashes, and unit types for `text`, `markdown`, `json_document`, and placeholder cases.
Traceability: US1; FR-003; AC-F1; AC-F6.

- [ ] T013 [US1] Define repository and indexing port traits in crates/mod_memory/src/infra/repo.rs and crates/mod_memory/src/infra/indexer.rs
Outcome: Application services depend on explicit source, memory-item, query, and indexing ports instead of concrete storage clients.
Dependencies: T011, T012.
Relevant inputs: plan.md Repository and Indexing Adapter sections; research.md Decision 1 and Decision 7.
Constraints: Ports must separate authoritative persistence from non-authoritative indexing; design for transactional writes plus asynchronous projection.
Done-when: `mod_memory` exposes traits for source persistence, memory-item persistence, query access, and indexing/outbox interaction.
Traceability: US1; FR-004; FR-008; constitution Layered handler/service/repository separation.

- [ ] T036 [US1] Define GraphProjectionPort and a NoopGraphProjectionAdapter in crates/mod_memory/src/infra/graph.rs, crates/mod_memory/src/domain/event.rs, and crates/core_infra/src/falkordb.rs
Outcome: The future FalkorDB boundary is executable in this slice without adding graph runtime behavior or coupling current retrieval/write paths to graph concerns.
Dependencies: T013.
Relevant inputs: spec.md FR-015; plan.md Future Graph Boundary; research.md Decision 9.
Constraints: Use canonical identifiers only, keep the adapter no-op in this slice, and avoid introducing FalkorDB-specific identifiers or runtime dependencies into the canonical model.
Done-when: The application can emit graph projection events to a no-op boundary that preserves the additive expansion contract for future graph work.
Traceability: US1; FR-015; constitution Storage responsibility separation.

- [ ] T014 [US1] Implement the SurrealDB source repository and uniqueness bootstrap in crates/core_infra/src/surrealdb.rs and crates/mod_memory/src/infra/surreal_source_repo.rs
Outcome: The system can create-or-return an authoritative source record keyed by `external_id` and canonical payload hash.
Dependencies: T007, T011, T013.
Relevant inputs: plan.md Source model, Repository section, Idempotency decision; data-model.md Source validation rules.
Constraints: Enforce uniqueness in SurrealDB, not in handler memory; preserve immutable `source_id` and reserved system metadata.
Done-when: A repository adapter can persist or replay `Source` rows transactionally and distinguish same-payload replay from conflicting payloads.
Traceability: US1; FR-002; FR-004; AC-F2.

- [ ] T015 [US1] Implement the SurrealDB memory-item repository and indexing outbox persistence in crates/core_infra/src/surrealdb.rs and crates/mod_memory/src/infra/surreal_memory_repo.rs
Outcome: Derived memory items and durable indexing jobs are written in the same authoritative transaction as the source.
Dependencies: T007, T012, T013, T014.
Relevant inputs: data-model.md Memory Item and MemoryIndexJob; plan.md Transactions and Repository responsibilities.
Constraints: `(source_id, sequence)` and `urn` uniqueness must be enforced; no partial authoritative state may remain after failure.
Done-when: A transactional write can commit source, ordered memory items, and an outbox job atomically or roll back all of them.
Traceability: US1; FR-003; FR-004; NC-006.

- [ ] T016 [US1] Implement RegisterSourceService in crates/mod_memory/src/application/register_source.rs
Outcome: A use-case service validates canonical commands, normalizes content, enforces timeout/idempotency, persists authoritative state, and returns registration results with indexing status.
Dependencies: T011, T012, T013, T014, T015.
Relevant inputs: plan.md RegisterSourceService; spec.md User Story 1 and edge cases; research.md Decision 4, Decision 6, Decision 7.
Constraints: Meilisearch failures must not fail the write path; normalization must time out at 30 seconds; empty content must still yield a memory item; standard replay must compare normalized JSON hashes while preserving the first authoritative raw body unchanged.
Done-when: The service can produce the success and failure states required by the registration contract, including `queued`/`indexed`/`deferred` indexing status and direct-standard replay/conflict handling, without direct HTTP or database client coupling.
Traceability: US1; FR-002; FR-003; FR-004; AC-R1; AC-R2.

- [ ] T017 [US1] Implement the source registration endpoint and request canonicalizers in crates/app_server/src/handlers/source_register.rs, crates/app_server/src/handlers/mod.rs, crates/app_server/src/router.rs
Outcome: `POST /sources/register` accepts canonical, Open Badges, and CLR JSON bodies, maps them to the service command, and returns OpenAPI-compliant success/error responses.
Dependencies: T006, T016.
Relevant inputs: contracts/memory-ingest.openapi.yaml `/sources/register`; spec.md FR-001, FR-014; research.md Decision 2.
Constraints: Boundary DTO validation must stay at the HTTP layer; handlers must not call SurrealDB or Meilisearch directly; supported standard payloads must map to `document_type = json`, preserve the accepted raw UTF-8 body, and expose only the public `indexing_status` vocabulary.
Done-when: The route returns `201`, `200`, `400`, `408`, `409`, `413`, and `503` responses with the expected JSON shapes, direct-standard canonicalization semantics, and service-driven behavior.
Traceability: US1; G1; FR-001; FR-014.

**Checkpoint**: User Story 1 is functional and testable as an MVP ingest slice.

---

## Phase 4: User Story 2 - Memory Item Retrieval (Priority: P1)

**Goal**: Retrieve authoritative memory-item content and metadata by URN from SurrealDB.

**Independent Test**: Register a source, capture a returned memory-item URN, call `GET /memory-items/{urn}`, and verify content and metadata match the authoritative write.

### Tests for User Story 2

- [ ] T018 [P] [US2] Add OpenAPI-backed contract validation for GET /memory-items/{urn} in tests/contract/get_memory_item_contract.rs
Outcome: The retrieval route's path parameter, success response, and not-found/storage error shapes are fixed against the published contract.
Dependencies: T017.
Relevant inputs: contracts/memory-ingest.openapi.yaml `/memory-items/{urn}`; spec.md FR-005, FR-007.
Constraints: Assert authoritative response shape only; do not use search-projection data.
Done-when: Contract tests fail until the endpoint returns the documented `MemoryItemResponse` and structured 404/503 errors.
Traceability: US2; FR-005; FR-007; constitution API contract tests.

- [ ] T019 [P] [US2] Add integration coverage for successful and missing memory-item retrieval in tests/integration/get_memory_item_flow.rs
Outcome: End-to-end retrieval behavior is specified for an existing URN and a missing URN.
Dependencies: T017.
Relevant inputs: spec.md User Story 2 acceptance scenarios; quickstart.md Smoke Test: Retrieval.
Constraints: Register test data through the public API or service path first; verify byte-accurate stored content for the positive case.
Done-when: The integration suite contains failing assertions for `200 OK` content fidelity and `404` not-found behavior.
Traceability: US2; AC-F3; AC-F6.

### Implementation for User Story 2

- [ ] T020 [US2] Implement the memory-item query repository and GetMemoryItemService in crates/mod_memory/src/infra/surreal_memory_query.rs and crates/mod_memory/src/application/get_memory_item.rs
Outcome: The application can load a single authoritative memory item plus optional source context from SurrealDB.
Dependencies: T013, T015.
Relevant inputs: plan.md GetMemoryItemService and Retrieval View; data-model.md Memory Item; spec.md FR-005.
Constraints: Query path must use authoritative storage only; no Meilisearch fallback.
Done-when: A service returns the canonical retrieval view or a not-found/shared storage error for a requested URN.
Traceability: US2; FR-005; NC-002.

- [ ] T021 [US2] Implement the memory-item retrieval endpoint in crates/app_server/src/handlers/memory_item_get.rs, crates/app_server/src/handlers/mod.rs, crates/app_server/src/router.rs
Outcome: `GET /memory-items/{urn}` is exposed with correct path parsing, response DTO mapping, and structured errors.
Dependencies: T006, T020.
Relevant inputs: contracts/memory-ingest.openapi.yaml `/memory-items/{urn}`; spec.md User Story 2.
Constraints: Handler output must preserve authoritative content exactly as stored; error mapping must reuse the common JSON error model.
Done-when: The route satisfies the contract and integration tests for existing and missing URNs.
Traceability: US2; G4; FR-005; FR-007.

**Checkpoint**: User Story 2 is independently testable against authoritative persistence.

---

## Phase 5: User Story 3 - Source Metadata and Relationship Access (Priority: P1)

**Goal**: Retrieve a source and all associated memory items ordered by `sequence`.

**Independent Test**: Register a multi-item source, call `GET /sources/{source-id}`, and verify source metadata plus ordered child items.

### Tests for User Story 3

- [ ] T022 [P] [US3] Add OpenAPI-backed contract validation for GET /sources/{source-id} in tests/contract/get_source_contract.rs
Outcome: The source retrieval endpoint's path parameter, response fields, and error semantics are pinned to the contract.
Dependencies: T017.
Relevant inputs: contracts/memory-ingest.openapi.yaml `/sources/{source-id}`; spec.md FR-006, FR-007.
Constraints: Assert ascending `sequence` ordering in the response schema usage, not just field presence, and assert the public `indexing_status` vocabulary on source responses.
Done-when: Contract tests fail until the endpoint returns the documented source-plus-items response, `indexing_status`, and structured 404/503 failures.
Traceability: US3; FR-006; FR-007.

- [ ] T023 [P] [US3] Add integration coverage for ordered source retrieval in tests/integration/get_source_flow.rs
Outcome: End-to-end behavior is specified for retrieving source metadata and all associated memory items in stable order.
Dependencies: T017.
Relevant inputs: spec.md User Story 3 acceptance scenarios; quickstart.md Smoke Test: Retrieval.
Constraints: Verify ordering by `sequence` and association to the originating `source_id`.
Done-when: Integration tests contain failing assertions for source metadata echoing and ordered memory-item lists.
Traceability: US3; AC-F4; NC-004.

### Implementation for User Story 3

- [ ] T024 [US3] Implement the source query repository and GetSourceService in crates/mod_memory/src/infra/surreal_source_query.rs and crates/mod_memory/src/application/get_source.rs
Outcome: The application can load a source aggregate view with all related memory items ordered from authoritative storage.
Dependencies: T013, T015.
Relevant inputs: plan.md GetSourceService and Retrieval View; data-model.md Source Retrieval View; spec.md FR-006.
Constraints: Query must remain SurrealDB-authoritative and scale to large item counts without changing public ordering semantics.
Done-when: The service returns source metadata plus ordered memory items or a shared not-found/storage error.
Traceability: US3; FR-006; AC-F4.

- [ ] T025 [US3] Implement the source retrieval endpoint in crates/app_server/src/handlers/source_get.rs, crates/app_server/src/handlers/mod.rs, crates/app_server/src/router.rs
Outcome: `GET /sources/{source-id}` exposes the authoritative source view through the HTTP API.
Dependencies: T006, T024.
Relevant inputs: contracts/memory-ingest.openapi.yaml `/sources/{source-id}`; spec.md User Story 3.
Constraints: The route must serialize memory items in ascending `sequence` and reuse the common error/trace middleware.
Done-when: The endpoint satisfies both the contract and ordered retrieval integration tests.
Traceability: US3; G4; FR-006; FR-007.

**Checkpoint**: User Story 3 completes the authoritative ingest-persist-retrieve slice for source context.

---

## Phase 6: User Story 4 - Search Projection for Basic Filtering (Priority: P2)

**Goal**: Build the non-authoritative search projection and expose the basic search API without weakening authoritative write/read guarantees.

**Independent Test**: Register a source with known content, wait for indexing, call `GET /search/memory-items`, and verify hits or degraded `503` behavior when search is unavailable.

### Tests for User Story 4

- [ ] T026 [P] [US4] Add OpenAPI-backed contract validation for GET /search/memory-items in tests/contract/search_memory_items_contract.rs
Outcome: Query parameters, result shape, and degraded-search error behavior are pinned to the contract.
Dependencies: T008.
Relevant inputs: contracts/memory-ingest.openapi.yaml `/search/memory-items`; spec.md FR-008, FR-009.
Constraints: Contract coverage must include filters for `source-id`, `document-type`, `limit`, and `offset`, including `document-type = json`, and must assert that the `200` response returns projection hits rather than authoritative memory-item payloads.
Done-when: Contract tests fail until the endpoint returns `SearchResponse` projection hits or structured `503` for unavailable search.
Traceability: US4; FR-008; FR-009.

- [ ] T027 [P] [US4] Add integration coverage for indexing success and degraded search in tests/integration/search_projection_flow.rs
Outcome: The projection pipeline is specified for both healthy indexing and Meilisearch-unavailable scenarios.
Dependencies: T017.
Relevant inputs: spec.md User Story 4 acceptance scenarios; edge case `Search projection failures`; research.md Decision 8.
Constraints: Registration must still succeed when search is unavailable; the test should observe `queued` and `deferred` indexing states rather than require synchronous search writes, and should cover search filtering for `document_type = json`.
Done-when: Integration tests contain failing assertions for searchable projection hits, direct-standard search projections, and degraded `503` search behavior.
Traceability: US4; AC-F5; AC-R2; NC-007.

- [ ] T037 [P] [US4] Add Meilisearch storage-adapter contract coverage in tests/contract/meilisearch_projection_contract.rs
Outcome: The non-authoritative search adapter's projection guarantees are pinned to the published slice semantics.
Dependencies: T007.
Relevant inputs: constitution Storage adapter verification; plan.md Storage adapter contract tests; spec.md FR-008, FR-009.
Constraints: Assert index settings, projection schema, filter behavior, sort behavior, and degraded availability semantics without treating Meilisearch as authoritative storage.
Done-when: Adapter contract tests fail until the Meilisearch implementation proves the projection guarantees required by the spec and constitution.
Traceability: US4; FR-008; FR-009; constitution Storage adapter verification.

### Implementation for User Story 4

- [ ] T028 [US4] Implement the Meilisearch indexing adapter and index settings bootstrap in crates/core_infra/src/meilisearch.rs and crates/mod_memory/src/infra/meili_indexer.rs
Outcome: Canonical memory items can be translated into `memory_items_v1` projection documents with idempotent index settings.
Dependencies: T007, T013, T015.
Relevant inputs: data-model.md Search Projection; plan.md Indexing Adapter; spec.md FR-008.
Constraints: The adapter must remain non-authoritative and prepare content preview, filters, and sortable timestamps exactly as planned.
Done-when: A concrete adapter can upsert projection documents and configure index settings for search/filter/sort behavior.
Traceability: US4; FR-008; research.md Decision 7.

- [ ] T029 [US4] Implement the outbox-driven indexing worker and startup hook in crates/mod_memory/src/application/index_memory_items.rs, crates/app_server/src/main.rs, crates/app_server/src/state.rs
Outcome: Background processing can consume durable indexing jobs, invoke the Meilisearch adapter, and mark success or retryable/dead-letter states.
Dependencies: T005, T015, T028.
Relevant inputs: data-model.md MemoryIndexJob lifecycle; plan.md Background indexing worker; research.md Decision 7.
Constraints: Worker failures must never roll back authoritative writes; retries must be bounded and observable; retry exhaustion must promote jobs to `dead_letter`; backlog state must remain inspectable via authoritative outbox records.
Done-when: App bootstrap starts a background worker that can process queued indexing jobs from the outbox lifecycle, promote exhausted retries to `dead_letter`, preserve authoritative backlog visibility, and map internal job states to external `queued`/`indexed`/`deferred` responses without leaking internal vocabulary.
Traceability: US4; FR-008; NC-007; constitution Indexing vs query separation.

- [ ] T030 [US4] Implement SearchMemoryItemsService and the search endpoint in crates/mod_memory/src/application/search_memory_items.rs, crates/app_server/src/handlers/search_memory_items.rs, crates/app_server/src/handlers/mod.rs, crates/app_server/src/router.rs
Outcome: `GET /search/memory-items` exposes Meilisearch-backed query, filtering, pagination, and degraded-search error mapping.
Dependencies: T006, T028, T029.
Relevant inputs: contracts/memory-ingest.openapi.yaml `/search/memory-items`; plan.md SearchMemoryItemsService; spec.md FR-009.
Constraints: Search must not read from SurrealDB as a hidden fallback; degraded behavior must remain explicit.
Done-when: The route returns search hits when Meilisearch is healthy and a structured `503` when it is not.
Traceability: US4; G5; FR-009; AC-R2.

**Checkpoint**: User Story 4 completes the first search projection slice without weakening authoritative consistency.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Validate the full vertical slice, lock the contract, and update operational docs.

- [X] T038 Implement Prometheus-compatible latency metrics and histogram buckets in crates/app_server/src/middleware.rs, crates/app_server/src/state.rs, and crates/app_server/src/main.rs
Outcome: Endpoint latency is emitted with the histogram buckets required by the spec, and tracing/metrics share the same request correlation model.
Dependencies: T005, T006.
Relevant inputs: spec.md FR-013, NC-009; plan.md Observability requirements.
Constraints: Instrument all public endpoints, use the histogram buckets defined in the spec, and do not log or emit metric labels containing sensitive request content; labels must distinguish canonical versus direct-standard ingest only through bounded dimensions such as `document_type` and `ingest_kind`.
Done-when: The runtime exposes the latency measurements needed to calculate p95/p99 for all public endpoints using the configured histogram buckets and can segment registration metrics by canonical versus direct-standard ingest.
Traceability: FR-013; NC-009; constitution Observability.

- [ ] T039 [P] Add observability validation for trace propagation and latency metrics in tests/integration/observability_tracing_flow.rs and tests/integration/observability_metrics.rs
Outcome: The required request tracing and latency observability guarantees are verified end-to-end.
Dependencies: T017, T021, T025, T030, T038.
Relevant inputs: spec.md FR-013, NC-009; plan.md Testing / Validation Strategy.
Constraints: Assert W3C trace-context propagation, span/trace coverage for registration and search indexing, and the presence of histogram-backed latency observations for all public endpoints, including direct-standard registration.
Done-when: Integration tests fail until trace propagation and metrics behavior satisfy the published observability requirements for canonical and direct-standard ingest paths.
Traceability: FR-013; NC-009; constitution API contract tests.

- [ ] T044 [P] Add benchmark and load-validation coverage for registration, retrieval, and search in benches/memory_ingest_latency.rs, tests/perf/memory_ingest_slo.rs, and tests/fixtures/perf/*.json
Outcome: The feature has an executable performance validation plan tied to the published p95/p99 goals before implementation is declared complete.
Dependencies: T017, T021, T025, T030, T038.
Relevant inputs: spec.md AC-P1, AC-P2, AC-P3, NC-001, NC-002, NC-003, NC-004; plan.md Performance validation strategy; quickstart.md Suggested Local Validation Sequence.
Constraints: Measure p95/p99 latency, throughput, and error rate for representative payloads and data shapes: canonical markdown under 100 KB, direct-standard Open Badges/CLR JSON under 100 KB, retrieval of a 10k-item source, and search against a representative 1M-item projection corpus or replayable synthetic fixture. Use Prometheus-compatible metrics emitted by T038 as the primary capture path and define pass/fail thresholds for each published performance requirement.
Done-when: The repository contains an automatable benchmark/load suite that fails the release gate when AC-P1, AC-P2, AC-P3, AC-V4, NC-001, NC-002, NC-003, or NC-004 are exceeded and emits a reproducible report of p95/p99 latency, throughput, and error rate.
Traceability: AC-P1; AC-P2; AC-P3; AC-V4; NC-001; NC-002; NC-003; NC-004; NC-009.

- [ ] T045 [P] Add explicit standard-payload validation correctness verification in tests/contract/register_source_standard_validation_matrix.rs and tests/fixtures/register_source/validation_matrix/*.json
Outcome: The documented allow and reject rules for Open Badges and CLR direct ingest are pinned as executable verification coverage rather than left implicit across existing contract tests.
Dependencies: T017.
Relevant inputs: spec.md FR-001, FR-002, FR-014, AC-F7, AC-V1; plan.md Remaining Implementation Risks; contracts/memory-ingest.openapi.yaml `/sources/register`.
Constraints: Cover accepted payloads, pinned-schema failures, and shape-valid-but-unmappable payloads for both supported standard families; assert that only canonical-mappable payloads can create authoritative state and that rejected payloads produce the documented `400` semantics.
Done-when: Verification coverage fails until the implementation proves standard-payload validation behavior matches the documented allow and reject matrix exactly.
Traceability: FR-001; FR-002; FR-014; AC-F7; AC-V1; NC-012.

- [ ] T046 [P] Add replay hashing determinism and idempotency verification in tests/unit/normalized_json_hash.rs, tests/integration/register_source_replay_hashing.rs, and tests/fixtures/register_source/replay_hashing/*.json
Outcome: Replay behavior for supported standard payloads is verified directly against the normalized-hash rule and preserved-content retrieval guarantee.
Dependencies: T016, T017, T021.
Relevant inputs: spec.md FR-002, FR-005, AC-F2, AC-F3, AC-V2; plan.md Remaining Implementation Risks; data-model.md Canonicalization Rules.
Constraints: Prove that formatting-only variants of the same validated payload produce the same canonical payload hash and authoritative identifiers, that the first committed raw body remains the retrieval payload, and that semantic changes still yield `409 Conflict`.
Done-when: Verification coverage fails until replay hashing is demonstrably deterministic for equivalent payload semantics and conflict-producing for semantic divergence.
Traceability: FR-002; FR-005; AC-F2; AC-F3; AC-V2.

- [ ] T047 [P] Add authoritative outbox-to-projection mapping verification in tests/contract/indexing_outbox_mapping_contract.rs and tests/integration/indexing_status_mapping_flow.rs
Outcome: The durable indexing path is verified as a semantic mapping from authoritative rows to projection inputs, with public status translation checked independently from worker internals.
Dependencies: T015, T029, T030.
Relevant inputs: spec.md FR-006, FR-008, FR-009, AC-F1, AC-R2, AC-V3; plan.md Remaining Implementation Risks; data-model.md MemoryIndexJob and Public `indexing_status` Mapping.
Constraints: Assert that committed outbox records contain the authoritative keys needed to reconstruct projection documents, that projection inputs can be rehydrated without semantic loss from `Source` and `MemoryItem` rows, and that external responses expose only `queued`, `indexed`, or `deferred`.
Done-when: Verification coverage fails until outbox persistence, projection rehydration, and public `indexing_status` mapping all match the documented contract.
Traceability: FR-006; FR-008; FR-009; AC-F1; AC-R2; AC-V3; NC-007.

- [ ] T040 [P] Add concurrency and multi-instance validation in tests/integration/register_source_concurrency.rs and tests/integration/multi_instance_consistency.rs
Outcome: Horizontal-scale and duplicate-registration guarantees are verified explicitly rather than inferred from unit behavior.
Dependencies: T017, T021, T025.
Relevant inputs: spec.md AC-F6, AC-R3, NC-005; plan.md Concurrency tests.
Constraints: Cover duplicate registration races, conflicting concurrent payloads, and consistent retrieval behavior across multiple stateless app instances backed by the same SurrealDB database.
Done-when: Integration tests fail until the slice proves stateless multi-instance consistency and concurrent idempotency behavior.
Traceability: NC-005; AC-F6; AC-R3.

- [ ] T041 Update operational runbook coverage for indexing backlog inspection, retry exhaustion handling, and manual re-index recovery in specs/001-memory-ingest/quickstart.md, specs/001-memory-ingest/contracts/README.md, and README.md
Outcome: Operators have explicit guidance for identifying backlog growth, understanding dead-letter promotion, and rebuilding Meilisearch from authoritative data.
Dependencies: T029, T033.
Relevant inputs: quickstart.md Operational Validation Notes; plan.md Rollout / Migration Notes; spec.md edge case `Search projection failures`.
Constraints: Document only supported recovery paths for this slice; recovery must rebuild from authoritative SurrealDB data and outbox identifiers rather than from stale projection state.
Done-when: The documentation set includes concrete guidance for backlog inspection, retry exhaustion, and manual re-index/recovery before rollout.
Traceability: NC-007; G6; constitution Documentation & Runbooks.

- [ ] T031 Add full-slice integration coverage for health, ready, register, retrieve, and search flows in tests/integration/memory_ingest_vertical_slice.rs
Outcome: A single end-to-end validation covers the complete vertical slice from service bootstrap through ingest, retrieval, and search behavior.
Dependencies: T008, T017, T021, T025, T030.
Relevant inputs: quickstart.md Suggested Local Validation Sequence; spec.md G7 and success criteria.
Constraints: Keep the test isolated and deterministic; verify authoritative retrieval even when search is degraded.
Done-when: One integration suite exercises the documented smoke path and catches cross-story regressions.
Traceability: G7; AC-F1; AC-F3; AC-F4; AC-F5.

- [ ] T032 Validate the published API contract against the implemented routes in tests/contract/openapi_smoke.rs and specs/001-memory-ingest/contracts/README.md
Outcome: The repository has an explicit contract-validation checkpoint that ties route behavior back to the OpenAPI document and documents how to run it.
Dependencies: T009, T018, T022, T026, T034, T017, T021, T025, T030.
Relevant inputs: contracts/memory-ingest.openapi.yaml; constitution API contracts; user requirement `contract validation`.
Constraints: Do not duplicate endpoint logic; treat the contract as the source of truth for request/response semantics.
Done-when: There is a documented and automatable way to validate the implementation against the published contract set.
Traceability: US1-US4; FR-001 through FR-013; constitution API contract tests.

- [ ] T033 Update developer documentation and smoke-test instructions in specs/001-memory-ingest/quickstart.md and README.md
Outcome: Local setup, environment variables, bootstrap commands, and manual verification steps reflect the implemented slice instead of placeholders.
Dependencies: T002, T008, T017, T021, T025, T030.
Relevant inputs: quickstart.md; README.md; user requirement `문서화 / quickstart 업데이트`.
Constraints: Document only behavior actually shipped in this slice; keep instructions aligned with docker-compose and route contracts.
Done-when: A developer can follow the docs to boot dependencies, run the app, hit health/readiness, register a source, retrieve it, and test search behavior.
Traceability: G6; G7; constitution Documentation & Runbooks.

---

## Dependencies & Execution Order

### Phase Dependencies

- Setup (Phase 1): start immediately.
- Foundational (Phase 2): depends on T001-T002 and blocks all user stories.
- User Story 1 (Phase 3): depends on T003-T008.
- User Story 2 (Phase 4): depends on T013-T015 and a working US1 registration path.
- User Story 3 (Phase 5): depends on T013-T015 and a working US1 registration path.
- User Story 4 (Phase 6): depends on T015 and Phase 2 bootstrap because indexing is outbox-driven.
- Polish (Phase 7): depends on all targeted routes and tests being implemented.

### User Story Dependencies

- US1 is the MVP and has no dependency on later stories.
- US2 depends on US1 authoritative persistence but not on US3 or US4.
- US3 depends on US1 authoritative persistence but not on US2 or US4.
- US4 depends on US1 outbox persistence and Phase 2 bootstrap but does not block authoritative retrieval.

### Suggested Dependency Graph

- T001 -> T002 -> {T003, T005, T007}
- T001 -> T004
- {T003, T004, T005} -> T006
- {T005, T006, T007} -> T008 -> T034
- {T004, T007, T008} -> {T009, T010, T011, T012, T035, T042, T043}
- {T011, T012} -> T013 -> T036
- {T013, T036} -> T014 -> T015 -> T016 -> T017
- T017 -> {T018, T019, T022, T023, T027, T039, T040, T044}
- T015 -> {T020, T024, T028}
- {T006, T020} -> T021
- {T006, T024} -> T025
- T007 -> T037
- {T005, T006} -> T038 -> {T039, T044}
- {T005, T015, T028} -> T029 -> T030
- {T008, T017, T021, T025, T030} -> T031
- {T009, T018, T022, T026, T034, T017, T021, T025, T030} -> T032
- {T002, T008, T017, T021, T025, T030} -> T033 -> T041
- T017 -> T045
- {T016, T017, T021} -> T046
- {T015, T029, T030} -> T047

## Parallel Opportunities

- Phase 2: T004 and T007 can run in parallel once T001-T003 are complete enough for shared types/config assumptions.
- Phase 2: T034 can be authored in parallel with late foundational work once the probe contract is stable.
- US1: T009, T010, T035, T042, and T043 can run in parallel; T011 and T012 can run in parallel.
- US2 and US3 test authoring can start in parallel after T017 because both rely on a working registration path for fixtures.
- US4: T026, T027, and T037 can run in parallel while adapter work is being defined.
- Cross-cutting validation T039, T040, T044, T045, T046, and T047 can run in parallel once the relevant runtime hooks exist.
- Documentation T033 and T041 can begin once the route set and outbox behavior are stable, in parallel with final validation T031-T032.

## Parallel Example: MVP Track

```bash
# After Phase 2 is complete, these can be split across implementers:
T009  POST /sources/register contract validation
T010  registration integration flow
T011  Source aggregate and command model
T012  MemoryItem aggregate and normalization rules
```

## Implementation Strategy

### MVP First

1. Complete Phase 1 and Phase 2.
2. Complete all US1 tasks through T017 plus T035-T036.
3. Run T009-T010 and validate the registration slice before adding retrieval work.
4. Do not declare the slice complete until T044, T045, T046, and T047 close the remaining implementation verification risks called out in the spec and plan.

### Incremental Delivery

1. Add US2 to validate authoritative item retrieval.
2. Add US3 to validate source-to-item relationship access.
3. Add US4 to attach non-authoritative search projection and degraded behavior.
4. Finish with T031-T033 to lock the contract and developer workflow.

### Suggested MVP Scope

- Strict MVP: Phase 1, Phase 2, and Phase 3 (US1) only.
- First production-shaped vertical slice requested here: Phase 1 through Phase 7, with US4 included because indexing/search projection is an explicit requirement.
