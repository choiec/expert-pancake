---
description: |
  Core rules every session: language, agent principles, answer format.
applyTo: "**"
---

**Language (§C)**: English only — code, comments, docstrings, UI strings, docs.
However, responses to the user should be in Korean if explicitly requested.

**Agent principles (§I)**: KISS — prefer the simplest option. Follow existing
conventions (formatting, naming, structure). No speculative implementation —
do not add modules, endpoints, or infrastructure unless in the current todo.
Boy scout rule — leave anything you touch cleaner than you found it.

**Task completion (§W)**: When finishing code changes, run `ruff format .`
and `pytest`; fix any failures before considering the task complete. If
the change touches DB-dependent code, run `pytest` with `DATABASE_URL`
and `NEO4J_*` set as well.

**Answer format (§O)**: When presenting multiple options, list at least one pro
and one con per option (one short line each). Omit only when options are
trivial or the user asks for no comparison.
