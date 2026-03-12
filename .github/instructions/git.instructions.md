---
description: |
  When using git, branching, committing, or renaming files. Trunk-based, solo
  dev; all work on main; exp/* branches only for local experiments.
applyTo: "**"
---

**Branch strategy (§G)**: Trunk-based development — always work directly on
`main`. Do not create feature, release, or hotfix branches.

**Experiment exception**: Short-lived `exp/<topic>` branches are allowed for
exploratory or risky work only. These branches must remain local only — do not
push `exp/*` branches to the remote. Merge or delete within the same session.
Never leave `exp/*` branches open across sessions.

**No PR workflow**: Solo dev — commit directly to `main`. No pull requests
unless pushing to a shared remote requires it.

**File renames (§R)**: When only the path or filename changes (content
unchanged), use `mv` or `git mv`; do not delete the old file and write a new
one. If the source directory is left empty after the move, remove it (e.g.
`rmdir` or delete the folder).
