# AGENTS.md

## Purpose

This repository uses a Spec-Driven Development (SDD) workflow. Codex agents working in this repo must treat specifications as the primary source of truth for implementation, rather than inferring behavior from incomplete code or ad hoc instructions.

The canonical agent surface for this repository is:
- `AGENTS.md` for repository-wide operating rules
- `.codex/prompts/` for phase-specific Spec Kit command prompts
- `.specify/` and `specs/` for durable workflow artifacts

Only the Codex operating surface is canonical for this repository.

The canonical workflow is:

1. Constitution
2. Specify
3. Clarify (if needed)
4. Plan
5. Tasks
6. Analyze / Checklist
7. Implement
8. Tasks-to-Issues (optional)

Each phase produces durable artifacts that become inputs to later phases.

---

## Core Operating Principles

### 1. Constitution First
Before generating or modifying feature specifications, read the project constitution and follow it strictly.

The constitution defines:
- architectural rules
- code quality expectations
- testing philosophy
- non-functional requirements
- performance, privacy, and security constraints
- documentation standards
- decision-making boundaries

If a requested change conflicts with the constitution, the conflict must be surfaced explicitly.

### 2. Specs Before Code
Do not jump directly to implementation unless:
- the user explicitly requests implementation, and
- the required spec/plan/tasks artifacts already exist and are coherent enough to support code generation.

If specification artifacts are missing or obviously incomplete, prefer generating or improving them first.

### 3. One Command, One Responsibility
Each workflow command has a narrow purpose:
- `constitution` updates project-wide governance
- `specify` creates or updates a feature specification
- `clarify` resolves ambiguity in a specification
- `plan` produces a technical implementation plan
- `tasks` decomposes a plan into concrete tasks
- `analyze` audits readiness, risk, and consistency
- `checklist` validates completeness against explicit criteria
- `implement` executes selected tasks only
- `taskstoissues` converts task lists into issue-ready artifacts

Do not silently perform the work of a later phase while executing an earlier phase.

### 4. Prefer Explicitness Over Assumption
If information is missing:
- identify assumptions clearly
- separate facts from proposals
- preserve unresolved questions in artifacts
- do not fabricate implementation details

### 5. Preserve Traceability
Every important implementation decision should be traceable to one or more of:
- constitution principles
- feature requirements
- acceptance criteria
- technical plan sections
- task identifiers

### 6. Keep Artifacts Consistent
When updating one artifact, reflect necessary downstream changes in related artifacts or explicitly note that follow-up updates are required.

Examples:
- If a plan changes architecture, tasks must be refreshed
- If clarifications materially change behavior, spec and plan must be reconciled
- If the constitution changes, future planning must follow the amended version

### 7. Small, Reviewable Changes
Prefer outputs that are:
- structured
- reviewable
- incremental
- testable
- easy to compare in version control

---

## Required Reading Order

When working on a feature, read in this order:

1. `AGENTS.md`
2. Project constitution (for example: `.specify/memory/constitution.md`)
3. Relevant feature spec (for example: `specs/<feature>/spec.md`)
4. Clarification records, if any
5. Plan (`plan.md`)
6. Related research/data/contracts/quickstart artifacts
7. Tasks (`tasks.md`)
8. Existing code

If any artifact is missing, say so explicitly and proceed only within the scope of the current command.

---

## Codex Command Surface

Use the Codex prompt set in `.codex/prompts/` as the command reference:
- `speckit.constitution.md`
- `speckit.specify.md`
- `speckit.clarify.md`
- `speckit.plan.md`
- `speckit.tasks.md`
- `speckit.analyze.md`
- `speckit.checklist.md`
- `speckit.implement.md`
- `speckit.taskstoissues.md`

These prompts must preserve the artifact chain `constitution → spec → plan → tasks → implementation`.

---

## Artifact Expectations

### Constitution
A durable governance document for the whole repository.

### Specification (`spec.md`)
Must focus on:
- user value
- requirements
- acceptance criteria
- edge cases
- constraints
- assumptions

### Plan (`plan.md`)
Must focus on:
- architecture
- technical decisions
- data model
- interfaces/contracts
- sequencing
- risks
- validation strategy

### Tasks (`tasks.md`)
Must focus on:
- concrete implementation work
- dependency order
- parallelizable work
- clear done conditions
- traceability back to plan/spec

---

## Agent Behavior Rules

### Always
- Be explicit about what phase you are operating in
- Respect scope boundaries
- Maintain consistency with earlier artifacts
- Prefer deterministic, review-friendly outputs
- Surface risks and open questions

### Never
- Invent requirements not supported by source artifacts
- Collapse multiple workflow phases into one without explicitly labeling it
- Implement code during a constitution/spec-only request
- Rewrite unrelated files “for cleanup”
- Hide assumptions or trade-offs

---

## Writing Guidelines

Use:
- precise headings
- concise bullets
- stable terminology
- consistent identifiers
- implementation-neutral language in specs
- implementation-specific language only in plans/tasks/code

Avoid:
- vague wording such as “handle appropriately”
- hidden assumptions
- contradictory acceptance criteria
- task descriptions that are too broad to execute safely

---

## Completion Standard

A deliverable is considered complete only when:
- it matches the command scope
- it is internally consistent
- it is aligned with the constitution
- it is actionable for the next workflow step
- it clearly marks any assumptions, risks, or unresolved items

---

## Command Routing Guidance

If a user asks for:

- “set project rules / principles / non-negotiables” → use constitution
- “describe what the feature should do” → use specify
- “resolve questions / ambiguities / missing decisions” → use clarify
- “decide how to build it technically” → use plan
- “break it into work items” → use tasks
- “audit quality / spot gaps / verify readiness” → use analyze or checklist
- “write the code now” → use implement
- “turn tasks into tracker items” → use taskstoissues

---

## Preferred Output Style

When generating artifacts:
- include a short purpose section
- make structure obvious
- preserve placeholders where user input is still needed
- keep files easy for humans and AI to consume

When generating code:
- explain what task is being implemented
- keep changes scoped
- mention any assumptions
- include self-check notes against the governing artifacts

---

## Repository Conventions

- Use Conventional Commits for commit messages
- Keep agent-related automation Codex-specific unless the repository explicitly adopts another canonical surface in a future migration
- Treat `AGENTS.md` as the single source of truth for agent operating rules

<!-- BEGIN AUTO-GENERATED CODEX CONTEXT -->
## Codex Runtime Context

This section is maintained by `.specify/scripts/bash/update-agent-context.sh codex` during planning work.

- Last updated: not yet generated
- Active feature: not yet generated
- Language/Version: not yet generated
- Primary Dependencies: not yet generated
- Storage: not yet generated
- Testing: not yet generated
- Project Type: not yet generated
<!-- END AUTO-GENERATED CODEX CONTEXT -->

---

## Final Reminder

The repository’s “north star” is not the latest prompt. It is the chain of durable artifacts:
constitution → spec → plan → tasks → implementation.

When in doubt, strengthen the artifacts before expanding the codebase.
