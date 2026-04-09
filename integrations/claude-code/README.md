# dirpack — Claude Code plugin

Injects a fresh token-budgeted [dirpack](https://github.com/rawwerks/dirpack) index of the current repository into every Claude Code session, so your agent starts with an accurate compressed map of the project instead of blindly grepping.

## What it does

- **SessionStart hook**: runs `dirpack pack . -t <budget> -f pipe` against `$CWD` and injects the output as `additionalContext` on every new, resumed, or cleared session.
- **`/dirpack` slash command**: toggle the plugin on/off and change the token budget without leaving Claude Code.
- **On by default**, budget defaults to **3000 tokens**.

## Installation

Requires `dirpack` on `PATH` (or in `~/.local/bin`, `~/.cargo/bin`, `/usr/local/bin`, `/usr/bin`). Install with:

```bash
curl -fsSL https://raw.githubusercontent.com/rawwerks/dirpack/master/install.sh | bash
```

Then install the plugin via Claude Code's marketplace, or symlink it locally:

```bash
ln -s "$(pwd)/integrations/claude-code" ~/.claude/plugins/dirpack
```

Then from Claude Code:

```bash
claude plugin marketplace add /path/to/dirpack/integrations/claude-code
claude plugin install dirpack@rawwerks-dirpack
```

Restart Claude Code. On the next session you should see a `## dirpack repo index` block in the injected context.

## Slash commands

Claude Code namespaces plugin commands as `/<plugin>:<command>`:

```
/dirpack:status           # show current config
/dirpack:on               # enable context injection
/dirpack:off              # disable context injection
/dirpack:budget 4000      # set token budget to 4000
```

State is persisted at `${XDG_CONFIG_HOME:-$HOME/.config}/dirpack/cc-plugin.json`:

```json
{ "enabled": true, "budget_tokens": 3000 }
```

Changes take effect on the **next** session start — restart Claude Code or run `/clear` to refresh.

## Layout

```
integrations/claude-code/
├── .claude-plugin/
│   ├── plugin.json         # plugin metadata
│   └── marketplace.json    # rawwerks marketplace entry
├── commands/
│   ├── status.md           # /dirpack:status
│   ├── on.md               # /dirpack:on
│   ├── off.md              # /dirpack:off
│   └── budget.md           # /dirpack:budget <N>
├── hooks/
│   ├── hooks.json          # SessionStart wiring
│   └── dirpack-session-start.sh
└── scripts/
    └── dirpack-cc-config.sh
```

## License

MIT

