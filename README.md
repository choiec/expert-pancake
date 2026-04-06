# expert-pancake

This repository is canonically operated as `Spec Kit + Codex`.

## Start Here

1. Read `AGENTS.md`
2. Read `.specify/memory/constitution.md`
3. Use the phase prompts in `.codex/prompts/`
4. Treat `specs/` as the durable feature artifact chain

## Workflow

The expected workflow is:

1. Constitution
2. Specify
3. Clarify
4. Plan
5. Tasks
6. Analyze / Checklist
7. Implement
8. Tasks-to-Issues

Do not skip ahead unless the required upstream artifacts already exist and are coherent.

## Canonical Paths

- `AGENTS.md`: repository-wide agent operating rules
- `.codex/prompts/`: Codex command references for each workflow phase
- `.specify/`: templates, memory, and workflow scripts
- `specs/`: feature specs, plans, tasks, and supporting artifacts

## Repository Conventions

- Only the Codex operating surface is canonical
- Conventional Commits are the expected commit message format

## Spec-To-Test Loop

- `spec.md`: defines behavior and acceptance criteria that must map to executable checks
- `plan.md`: defines architecture, trait boundaries, and validation strategy
- `tasks.md`: decomposes work into `RED -> GREEN -> REFACTOR -> VERIFY`
- Code and tests: prove the current task only, then advance to the next loop

See `docs/spec-tdd-loop.md` for the repository-standard mapping between Spec Kit artifacts and the Rust TDD toolchain.

## Standard Commands

- `just fmt`
- `just lint`
- `just test-fast`
- `just test-full`
- `just verify-story 001-memory-ingest`
- `just mutants`
- `just coverage`
- `just bench`

## Automation Policy

- Pull requests run the fast gate: format, clippy, and `cargo-nextest`
- Slow gates such as mutation testing and coverage run on demand or on the protected-branch workflow
- Performance checks with `criterion` are required only for stories whose plan publishes latency or throughput gates
