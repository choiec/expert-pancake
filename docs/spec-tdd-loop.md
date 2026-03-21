# Spec Kit To Rust TDD Mapping

## Purpose

This repository treats Spec Kit artifacts as the control plane for Rust implementation and testing. The goal is not to list tools, but to bind each artifact to an executable proof step.

## Artifact Mapping

- `constitution.md`
  - Defines non-negotiable rules: test-first, RED -> GREEN -> REFACTOR -> VERIFY, trait-boundary isolation, and minimum merge gates.
  - Governs which checks are mandatory before a change can merge.
- `spec.md`
  - Supplies the behavior to prove.
  - Functional requirements and acceptance criteria should map to unit, integration, contract, property, snapshot, or performance checks.
- `plan.md`
  - Defines architecture boundaries and verification strategy.
  - Ports, adapters, and external I/O boundaries from the plan determine where `mockall` and storage-adapter contract tests belong.
- `tasks.md`
  - Turns the spec and plan into executable loops.
  - Each story should have explicit `RED`, `GREEN`, `REFACTOR`, and `VERIFY` sections.

## Tool Mapping

- `cargo-nextest`
  - Default runner for local fast loops and CI fast gates.
  - Executes unit, integration, contract, property, and snapshot tests once those tests exist.
- `proptest`
  - Use for spec-level invariants that must hold across many inputs.
  - In this repository, normalization, canonical identity derivation, and replay hashing are the first targets.
- `insta`
  - Use for stable serialized outputs that come from published contracts or deterministic normalization.
  - Good fits are HTTP JSON bodies, OpenAPI-backed response shapes, and structured error payloads.
- `mockall`
  - Use at plan-defined trait boundaries.
  - Keeps domain and application tests off the network and out of live databases.
- `cargo-mutants`
  - Use on touched crates after the fast gate is green.
  - Focus first on domain and application logic where surviving mutants usually signal weak assertions.
- `cargo-llvm-cov`
  - Produces coverage evidence for story verification and protected-branch gates.
  - Coverage is evidence of exercised behavior, not a substitute for requirement traceability.
- `criterion`
  - Optional unless the plan publishes performance gates.
  - Use when registration, retrieval, or search latency thresholds must be protected from regression.

## Directory Conventions

- Crate-local unit tests: `crates/<crate>/src/**/*.rs` under `#[cfg(test)]`
- Crate-local integration tests: `crates/<crate>/tests/*.rs`
- Workspace-level contract and vertical-slice tests: `tests/contract/`, `tests/integration/`, `tests/perf/`
- Snapshot files: next to the owning test under `snapshots/`
- Shared `proptest` strategies: `crates/core_shared/src/test_support/` or `crates/<owner>/tests/support/` once the first reusable strategies exist
- Optional benchmarks: `crates/<crate>/benches/*.rs`

## Standard Loops

### Local Inner Loop

```bash
just test-fast
```

Use while moving a story from RED to GREEN to REFACTOR.

### Story Verification Loop

```bash
just verify-story 001-memory-ingest
```

Use before closing the story's VERIFY step.

### Release Gate

```bash
just fmt
just lint
just test-full
just mutants
just coverage
```

Use before merge when the touched area requires the full gate.

## CI Entry Points

- Pull request
  - `fmt`
  - `lint`
  - `test-fast`
- Push to protected branch or manual run
  - `test-full`
  - `coverage`
- Scheduled or manual slow verification
  - `mutants`
  - `bench` when a benchmark artifact exists

## Failure Recovery Order

1. Fix formatting and lint errors first.
2. Re-run the smallest failing `RED` or `VERIFY` scope with `cargo nextest`.
3. If snapshots changed intentionally, review and accept them explicitly.
4. If mutation testing fails, strengthen assertions before widening implementation.
5. If coverage or benchmarks regress, inspect the exact changed story path rather than adding broad unrelated tests.