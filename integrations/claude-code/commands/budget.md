---
description: Set the dirpack token budget (positive integer, e.g. /dirpack:budget 4000)
argument-hint: "<N>"
---

The user wants to change the dirpack plugin's token budget to `$1` tokens. Run the helper and show the raw output verbatim:

```bash
bash "${CLAUDE_PLUGIN_ROOT}/scripts/dirpack-cc-config.sh" budget $1
```

If `$1` is missing or not a positive integer, the helper will print an error and exit non-zero — relay that error to the user and ask them to supply a positive integer (e.g. `/dirpack:budget 4000`).

Then tell the user in one line that the change takes effect on the next SessionStart (restart Claude Code or `/clear`).
