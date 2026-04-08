---
description: Enable dirpack context injection at SessionStart
---

The user wants to enable the dirpack plugin's SessionStart context injection. Run the helper and show the raw output verbatim:

```bash
bash "${CLAUDE_PLUGIN_ROOT}/scripts/dirpack-cc-config.sh" on
```

Then tell the user in one line that the change takes effect on the next SessionStart (restart Claude Code or `/clear`).
