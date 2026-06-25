# Wiring claude-copet into Claude Code

claude-copet "feels" Claude Code by reading an append-only event log that Claude
Code's hooks write to. Set the hooks up once and the pet reacts in real time.

## The event log

- **Path:** `~/.claude-copet/events.jsonl` (`%USERPROFILE%\.claude-copet\events.jsonl` on Windows).
- **Format:** append-only JSONL — one JSON object per line:
  ```json
  {"ts":"2026-06-25T12:00:00Z","type":"PreToolUse","tool":"Bash","session":"abc123"}
  ```
  Only `type` is required; malformed lines are skipped. The writer (the hook) never
  assumes the pet is running, and the pet never assumes the writer is alive — if the
  pet is off, events simply accumulate in the file and are not lost.

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

## Event → mood mapping (this slice)

A direct, immediate mapping (decay/idle fall-back arrives in a later slice):

| Event              | Mood     | What you see                       |
|--------------------|----------|------------------------------------|
| `SessionStart`     | `wake`   | pet wakes up                       |
| `UserPromptSubmit` | `listen` | pet pays attention                 |
| `PreToolUse`       | `work`   | pet looks busy while a tool runs   |
| `Notification` / `error` | `panic` | pet reacts with alarm        |
| `Stop`             | `happy`  | pet celebrates the finished turn   |
| `PostToolUse`      | —        | no change (keeps the current mood) |

To sanity-check without Claude Code, append a line by hand while the pet is running:

```sh
echo '{"ts":"now","type":"PreToolUse","tool":"Bash","session":"test"}' >> ~/.claude-copet/events.jsonl
```
