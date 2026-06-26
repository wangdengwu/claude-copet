# Wiring claude-copet into Claude Code

claude-copet shows a status HUD for your active Claude Code session by reading an
append-only event log that Claude Code's hooks write to. Set the hooks up once and
the card updates in real time.

> **Easiest: one click.** Open the pet's Settings (right-click → Settings, or the
> gear) → **Claude Code → Connect**. It installs the hooks into
> `~/.claude/settings.json` for you (preserving any existing hooks, with a `.bak`
> backup) and writes the hook script to `~/.claude-copet/`. Restart Claude Code to
> apply. The rest of this doc is the manual equivalent / reference.

## The event log

- **Path:** `~/.claude-copet/events.jsonl` (`%USERPROFILE%\.claude-copet\events.jsonl` on Windows).
- **Format:** append-only JSONL — one JSON object per line:
  ```json
  {"ts":"2026-06-26T12:00:00Z","type":"PreToolUse","tool":"Bash","session":"abc123","cwd":"/Users/me/proj","transcript_path":"/Users/me/.claude/projects/.../abc123.jsonl"}
  ```
  Only `type` is required; malformed lines are skipped, and `cwd` / `transcript_path`
  are optional (older installs omit them). The HUD uses `cwd` for the session label,
  `transcript_path` to read context-% + model, and `session` to follow the active
  session. The writer (the hook) never assumes the app is running, and the app never
  assumes the writer is alive — if the app is off, events simply accumulate and are
  not lost.

## The hook script

[`hooks/claude-copet-hook.sh`](../hooks/claude-copet-hook.sh) appends one line and
exits immediately — fire-and-forget. It always exits `0`, so it can never block or
fail your Claude Code session, even when the pet isn't running.

```
sh /ABSOLUTE/PATH/TO/claude-copet/hooks/claude-copet-hook.sh <EVENT_TYPE>
```

## Claude Code settings

Add this to your Claude Code `settings.json` (`~/.claude/settings.json`), replacing
`/ABSOLUTE/PATH/TO` with the path to your clone:

```jsonc
{
  "hooks": {
    "SessionStart":     [{ "hooks": [{ "type": "command", "command": "sh /ABSOLUTE/PATH/TO/claude-copet/hooks/claude-copet-hook.sh SessionStart" }] }],
    "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "sh /ABSOLUTE/PATH/TO/claude-copet/hooks/claude-copet-hook.sh UserPromptSubmit" }] }],
    "PreToolUse":       [{ "matcher": "*", "hooks": [{ "type": "command", "command": "sh /ABSOLUTE/PATH/TO/claude-copet/hooks/claude-copet-hook.sh PreToolUse" }] }],
    "PostToolUse":      [{ "matcher": "*", "hooks": [{ "type": "command", "command": "sh /ABSOLUTE/PATH/TO/claude-copet/hooks/claude-copet-hook.sh PostToolUse" }] }],
    "Stop":             [{ "hooks": [{ "type": "command", "command": "sh /ABSOLUTE/PATH/TO/claude-copet/hooks/claude-copet-hook.sh Stop" }] }],
    "Notification":     [{ "hooks": [{ "type": "command", "command": "sh /ABSOLUTE/PATH/TO/claude-copet/hooks/claude-copet-hook.sh Notification" }] }]
  }
}
```

## Event → mood mapping (the corner pet)

The corner pet is a friendly accent driven by the event stream (with decay back to
idle/sleep when quiet):

| Event              | Mood     | What you see                       |
|--------------------|----------|------------------------------------|
| `SessionStart`     | `wake`   | pet wakes up                       |
| `UserPromptSubmit` | `listen` | pet pays attention                 |
| `PreToolUse`       | `work`   | pet looks busy while a tool runs   |
| `Notification` / `error` | `panic` | pet reacts with alarm        |
| `Stop`             | `happy`  | pet celebrates the finished turn   |
| `PostToolUse`      | —        | no change (keeps the current mood) |

To sanity-check without Claude Code, append a line by hand while the app is running
(include `cwd` / `transcript_path` to also exercise the session label and context-%):

```sh
echo '{"ts":"now","type":"PreToolUse","tool":"Bash","session":"test","cwd":"/Users/me/demo","transcript_path":""}' >> ~/.claude-copet/events.jsonl
```
