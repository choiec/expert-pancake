---
description: "Create or amend the project constitution and propagate rule changes to dependent artifacts."
agent: speckit.constitution
---
# /speckit.constitution

Create or update the project constitution from user-provided principles and repository context.

## Purpose

Use this command to define or revise the permanent governance rules for the repository. The constitution is the highest-priority project artifact and must be treated as the policy layer that all later specification, planning, and implementation work follows.

## Scope Guard

This command may:
- create or update the constitution
- refine project-wide principles
- normalize governance language
- identify related downstream artifacts that should be refreshed

This command must NOT:
- implement product features
- generate application source code
- create a feature specification
- create a technical plan for a specific feature
- generate task lists for a feature

If the user mixes constitution work with product or implementation requests, complete the constitution update first and then recommend the next appropriate workflow command.

## Inputs

Use the following sources, in order of priority:
1. the user's current request
2. existing constitution content
3. `AGENTS.md`
4. repository docs such as README, architecture notes, ADRs, or contributor docs
5. obvious technical signals from the codebase (framework, language, deployment model, testing approach)

## Required Output

Update or create the constitution with clear sections such as:
- project identity or mission
- architectural rules
- code quality standards
- testing expectations
- performance expectations
- security and privacy expectations
- documentation standards
- change control / review expectations
- non-negotiable constraints
- definition of done

If the current constitution is a placeholder template, replace placeholders with concrete language.

## Execution Flow

1. Read the existing constitution if present.
2. Identify missing or placeholder sections.
3. Derive concrete principles from user input and repository context.
4. Resolve conflicts by preferring explicit user direction.
5. Write a coherent, durable constitution suitable for future AI and human contributors.
6. Summarize what changed.
7. If necessary, list follow-up commands that should be run next.

## Quality Rules

- Be specific and durable
- Favor principles over temporary implementation details
- Prefer concise, testable statements over vague aspirations
- If something is unknown, mark it clearly rather than inventing it
- Keep this artifact project-wide, not feature-specific

## Suggested Closing Section

End with:
- a brief “Change Summary”
- a brief “Next Recommended Commands” section when applicable