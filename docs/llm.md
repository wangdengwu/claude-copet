# LLM voice (optional)

The pet always speaks via handwritten **template** lines — instant, offline, free.
On **special moments** it can optionally use an **LLM** for a richer, contextual
line. With the LLM off, the pet behaves exactly as the template-only build and
makes **zero network calls**.

> "Avoid HTTP" isn't literally possible — the model runs in Anthropic's cloud, so
> any LLM line is ultimately a network request. What the providers below change is
> *who* makes that request and whether you need to manage an API key.

## Providers

Selected by the `provider` field in settings (or the Settings panel dropdown).
Both implement the same internal `LlmClient` seam.

| Provider | Default | API key | How it calls | Notes |
|---|---|---|---|---|
| `claude-cli` | ✅ | **Not needed** | Spawns the local Claude Code CLI: `claude -p <prompt> --output-format text` | Reuses your existing Claude Code login; counts against your Claude Code usage. Requires `claude` on `PATH`. |
| `anthropic` | | Required | Direct Messages API call (`claude-haiku-4-5`, `ureq`) | Billed to your Anthropic API key. $1 / $5 per MTok (in/out). |

The `claude-cli` provider is the default because the pet's audience already runs
Claude Code — it works out of the box with no key to paste.

## Settings

Stored at `~/.claude-copet/settings.json` (outside the repo — **never committed**):

```jsonc
{
  "llm_enabled": false,        // off by default
  "provider": "claude-cli",    // "claude-cli" | "anthropic"
  "model": "claude-haiku-4-5", // used by the anthropic provider
  "api_key": ""                // anthropic only; stored locally
}
```

Edit it from the in-app **Settings panel** (gear toggle) or by hand. API-key
resolution order: `settings.api_key` (if non-empty) → `ANTHROPIC_API_KEY` env →
none. The key is never written to any committed file.

## Privacy (hard constraint)

Only a **de-sensitized summary** is ever sent to the LLM. It contains:

- event type (e.g. `Stop`, `SessionStart`)
- current mood, pet level, pet evolution stage
- today's aggregate counts (sessions / tool_calls / turns / errors)

It **never** contains: the raw `session` id, the `tool` string, or any code,
command, or file path. (The hook log doesn't capture code/paths in the first
place; the summary builder additionally strips the session id and tool name.)

## When it speaks, and failure behavior

- **Special moments only**, gated by a cooldown so the LLM is never spammed:
  evolution/level-up, `Happy` entered right after `Panic` (a hard-won success),
  and reunion after a long quiet gap.
- The call runs on a **background thread** — it never blocks the pet's event loop.
- On any error, timeout, disabled LLM, or missing `claude` CLI, the pet falls
  back to a **template line**. It never goes silent and never blocks Claude Code.
