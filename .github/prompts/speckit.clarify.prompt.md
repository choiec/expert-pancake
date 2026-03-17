---
agent: speckit.clarify
---
# /speckit.clarify

Resolve ambiguity in a feature specification and produce a clarified version or a clear clarification log.

## Purpose

Use this command after a specification exists but before planning or implementation if there are unresolved questions, contradictions, vague requirements, or missing decisions.

## Scope

This command may:
- identify ambiguous requirements
- surface unresolved assumptions
- propose clearly labeled options
- record clarification decisions
- update the spec or generate a clarification appendix

This command must NOT:
- generate a full technical plan unless explicitly requested later
- implement code
- replace product requirements with technical decisions prematurely

## Inputs

Use:
- the feature `spec.md`
- the constitution
- `AGENTS.md`
- any user answers already provided
- related repository context if directly relevant

## Ambiguity Types to Check

Look for:
- unclear user roles
- unclear success criteria
- missing edge-case behavior
- conflicting requirements
- vague performance expectations
- undefined data retention or privacy expectations
- uncertain out-of-scope boundaries
- missing error-handling expectations

## Output Format

Produce:
1. Clarification Summary
2. Open Questions (if any remain)
3. Resolved Decisions
4. Updated Assumptions
5. Recommended Spec Changes

If enough information exists, provide a revised `spec.md` section or patch-ready rewrite.

## Execution Rules

- Do not invent stakeholder intent
- Prefer explicit options if certainty is low
- Mark decisions as:
  - confirmed
  - inferred
  - pending
- Keep the language product-focused and implementation-neutral unless the spec explicitly demands technical constraints