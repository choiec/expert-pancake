---
description: "Generate a technical implementation plan from an approved feature specification."
agent: speckit.plan
---
# /speckit.plan

Generate a technical implementation plan from an approved feature specification.

## Purpose

Use this command to decide how the feature should be built technically, including architecture, data flow, interfaces, risks, sequencing, and validation strategy.

## Scope Guard

This command may:
- produce a technical design
- define modules/components
- propose interfaces and contracts
- define data structures or models
- identify dependencies and phases
- call out risks and trade-offs

This command must NOT:
- implement source code
- skip over unresolved product ambiguity that should be clarified first
- create a final task list unless explicitly requested later

## Inputs

Use:
- the constitution
- the feature spec
- clarification notes
- `AGENTS.md`
- relevant repository context
- existing architecture and operational constraints

## Required Sections

A strong plan usually includes:
- Technical Summary
- Constitution Alignment Check
- Architecture / Components
- Data Model Implications
- Interface / Contract Considerations
- Storage / State / API Decisions
- Failure Modes / Edge Cases
- Security / Privacy Notes
- Performance / Scalability Notes
- Testing / Validation Strategy
- Rollout or Migration Notes (if relevant)
- Open Risks and Decisions

## Execution Rules

- Translate requirements into technical design without changing product intent
- Make trade-offs explicit
- Prefer consistent patterns already used in the repo unless the plan justifies a change
- Keep the plan actionable for task generation
- Record unresolved technical decisions rather than hiding them