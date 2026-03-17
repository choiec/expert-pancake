---
agent: speckit.checklist
---
# /speckit.checklist

Validate an artifact or workflow stage against an explicit checklist.

## Purpose

Use this command to perform a structured completeness review of one or more project artifacts, such as a constitution, specification, plan, task list, or implementation patch.

## Scope

This command may:
- assess completeness
- report pass / partial / fail outcomes
- identify missing sections
- identify contradictions
- identify risky assumptions
- suggest specific remediation steps

This command must NOT:
- silently rewrite the target artifact unless the user explicitly requests it
- implement code
- invent requirements to make the checklist pass

## Inputs

Review the relevant artifacts based on the user's request. Typical inputs include:
- `AGENTS.md`
- project constitution
- `spec.md`
- clarification notes
- `plan.md`
- `tasks.md`
- contracts, quickstart, research, or data model files
- code changes, if the checklist is for implementation readiness

## Output Format

Produce:
1. Scope
2. Checklist Used
3. Findings
4. Gaps / Risks
5. Recommended Fixes
6. Overall Status

Use statuses such as:
- PASS
- PASS WITH GAPS
- NEEDS REVISION
- BLOCKED

## Checklist Dimensions

Depending on the target artifact, assess:
- completeness
- internal consistency
- traceability
- alignment with the constitution
- actionability
- testability
- implementation readiness

## Execution Rules

- Quote or reference the exact section names that are missing or weak
- Distinguish “missing” from “unclear” from “conflicting”
- Be concise but specific
- Prefer actionable fixes over generic criticism