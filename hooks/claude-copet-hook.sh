#!/bin/sh
# claude-copet event hook — fire-and-forget. Appends exactly one JSON line to the
# event log and exits immediately. NEVER blocks or fails Claude Code: every step is
# best-effort and the script always exits 0, even if the pet is not running.
#
# Usage (from a Claude Code hook `command`):
#   sh /path/to/claude-copet-hook.sh <EVENT_TYPE>
# where <EVENT_TYPE> is one of: SessionStart UserPromptSubmit PreToolUse
# PostToolUse Stop Notification (or "error").
#
# Claude Code passes the hook payload as JSON on stdin; we best-effort pull
# tool_name / session_id from it (no jq dependency).

type="$1"
dir="$HOME/.claude-copet"
log="$dir/events.jsonl"

mkdir -p "$dir" 2>/dev/null

ts=$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null)
input=$(cat 2>/dev/null)

tool=$(printf '%s' "$input" | sed -n 's/.*"tool_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
session=$(printf '%s' "$input" | sed -n 's/.*"session_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)

printf '{"ts":"%s","type":"%s","tool":"%s","session":"%s"}\n' \
  "$ts" "$type" "$tool" "$session" >> "$log" 2>/dev/null

exit 0
