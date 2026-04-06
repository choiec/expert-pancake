---
description: "Analyze spec, plan, tasks, or implementation artifacts for consistency, risk, and readiness."
agent: speckit.analyze
---
# /speckit.analyze

Perform a structured analysis of project artifacts or implementation readiness.

## Purpose

Use this command to inspect a specification, plan, task list, or code change for risk, consistency, feasibility, missing dependencies, and readiness for the next stage.

## Scope

This command may:
- analyze gaps
- identify architectural or workflow risks
- detect missing prerequisites
- assess feasibility
- assess consistency across artifacts
- recommend remediation

This command must NOT:
- silently rewrite the artifacts unless explicitly asked
- implement code
- replace explicit stakeholder decisions with agent preference

## Analysis Dimensions

Check for:
- alignment with constitution
- requirement completeness
- plan feasibility
- dependency ordering
- contract/data-model consistency
- testing readiness
- operational considerations
- hidden assumptions
- scope creep

## Inputs

Use the artifacts relevant to the user's request, commonly:
- constitution
- spec
- clarifications
- plan
- tasks
- research/data/contracts/quickstart
- code changes (optional)

## Output Format

Produce:
1. Analysis Scope
2. Findings
3. Risks
4. Blocking Issues
5. Recommended Fixes
6. Readiness Assessment

Use explicit severity labels:
- critical
- high
- medium
- low