---
agent: speckit.specify
---
# /speckit.specify

Create or update a feature specification from a high-level request.

## Purpose

Use this command to transform product intent into a durable, reviewable specification that describes what the feature should do and how success will be judged.

## Scope Guard

This command may:
- create a new feature specification
- refine an existing specification
- organize requirements into user-facing scenarios
- define acceptance criteria
- identify assumptions and edge cases

This command must NOT:
- produce a full technical implementation plan
- generate code
- create a task list
- hide ambiguity instead of recording it

## Inputs

Use:
1. the user’s request
2. the constitution
3. `AGENTS.md`
4. existing product docs or related specs
5. relevant repository context

## Required Sections

The generated specification should include, at minimum:
- Title
- Problem / Context
- Goals
- Non-Goals
- Users / Actors
- User Stories or Use Cases
- Functional Requirements
- Non-Functional Constraints (if already known)
- Acceptance Criteria
- Edge Cases
- Assumptions / Open Questions

## Writing Rules

- Focus on behavior and value, not implementation
- Use clear “shall / must / should” language where helpful
- Separate mandatory requirements from optional ideas
- Keep acceptance criteria testable
- Identify uncertainty explicitly

## Output Format

Produce a feature specification that is ready for:
- clarification (if needed)
- planning
- review by stakeholders

At the end, include:
- a short “Known Unknowns” section
- a short “Recommended Next Command” section
