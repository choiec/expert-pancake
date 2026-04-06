---
description: "Implement approved Speckit tasks while preserving the active spec, plan, and architecture boundaries."
agent: speckit.implement
---
# /speckit.implement

Implement one or more approved tasks from the task list.

## Purpose

Use this command only when specification, planning, and task decomposition are complete enough to support safe code generation.

## Scope Guard

This command may:
- implement selected tasks
- modify code and related tests/docs
- add supporting validation, error handling, and configuration
- explain the implementation and any assumptions

This command must NOT:
- rewrite unrelated areas of the codebase
- invent new product requirements
- silently change the technical plan
- execute future workflow phases that were not requested

## Inputs

Read, in order:
1. `AGENTS.md`
2. the constitution
3. the feature spec
4. clarification notes
5. the plan
6. the tasks
7. relevant source files

## Implementation Rules

- Implement only the requested task(s) or the minimum prerequisite work needed to complete them safely
- Preserve architectural boundaries from the constitution and plan
- Respect naming, style, and error-handling conventions already established
- If required information is missing, state assumptions explicitly
- Update tests and documentation when needed for the completed task

## Output Expectations

Return:
- a concise summary of what was implemented
- the code changes
- any assumptions or deviations
- a short self-check against the governing artifacts
- any follow-up tasks discovered during implementation

## Completion Rules

A task is complete only when:
- the requested scope is implemented
- the change aligns with the plan
- related tests or validation are updated where appropriate
- the output is reviewable and coherent