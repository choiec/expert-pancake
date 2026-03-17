---
agent: speckit.taskstoissues
---
# /speckit.taskstoissues

Convert a task list into issue-ready work items for project tracking.

## Purpose

Use this command when a task list is approved and you want a clean, tracker-friendly representation suitable for GitHub Issues, Azure Boards, Jira, or another work item system.

## Scope

This command may:
- convert tasks into issue-ready entries
- group tasks into epics/stories/subtasks
- preserve dependency notes
- preserve acceptance or done conditions
- create implementation-ready issue bodies

This command must NOT:
- change the underlying technical plan
- invent new tasks not justified by the source artifacts
- implement code

## Inputs

Use:
- the task list
- the plan
- the spec
- the constitution if needed for labels or quality gates

## Output Format

For each issue-ready item, include:
- title
- summary
- scope
- dependencies
- acceptance / done conditions
- labels / tags (optional)
- implementation notes (brief, if useful)

## Conversion Rules

- Preserve task identifiers where possible
- Keep issue titles concise and actionable
- Split overlarge tasks into tracker-friendly units when the source tasks are clearly too broad
- Do not lose traceability back to the original tasks