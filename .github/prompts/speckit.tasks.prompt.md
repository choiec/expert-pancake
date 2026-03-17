---
agent: speckit.tasks
---
# /speckit.tasks

Generate an implementation task list from the approved specification and technical plan.

## Purpose

Use this command when the feature spec and plan are sufficiently stable and you need a concrete, dependency-aware sequence of work items that can be implemented safely.

## Scope

This command may:
- produce a task list
- group tasks by phase, milestone, or user story
- mark dependencies
- identify parallelizable work
- map tasks to files, modules, or subsystems
- define done conditions

This command must NOT:
- implement code
- introduce major new architecture decisions that belong in the plan
- create vague task items like “build backend” without decomposition

## Inputs

Use:
- the constitution
- the feature spec
- clarifications
- the technical plan
- research, contracts, data model, quickstart, and related artifacts

## Task Quality Rules

Every task should have:
- an identifier
- a short title
- a clear outcome
- dependencies (if any)
- relevant inputs
- constraints
- a concrete done-when condition

Prefer tasks that are:
- small enough to review
- large enough to be meaningful
- independently testable where possible

## Output Structure

Recommended sections:
- Task Generation Assumptions
- Dependency Notes
- Tasks by Phase or Story
- Parallel Opportunities
- Risks / Sequencing Notes

## Important Rules

- Preserve traceability back to spec and plan
- Put enabling work before dependent work
- Include validation and documentation tasks where necessary
- Make it obvious what an implementer should do next
``