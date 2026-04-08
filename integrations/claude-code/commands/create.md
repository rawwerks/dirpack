---
description: One-shot dirpack pack of the current working directory at the given token budget
argument-hint: "<N>"
---

The user wants a fresh one-shot dirpack snapshot of the current working directory at `$1` tokens. This is a shortcut for `dirpack pack . -t $1 -f pipe` — it does NOT change the persistent plugin config and fires even if `/dirpack:off` is set.

If `$1` is missing, empty, or not a positive integer, tell the user to supply one (e.g. `/dirpack:create 4000`) and do not run anything.

Otherwise run the Bash tool:

```bash
dirpack pack . --target-tokens $1 --format pipe --root-label .
```

Show the raw stdout in a fenced block and do not add any commentary, summary, or analysis. The user wants the pipe-format pack output verbatim so they can feed it to another agent.
