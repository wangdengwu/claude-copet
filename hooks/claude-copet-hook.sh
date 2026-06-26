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

# Skip events from claude-copet's own headless probe invocations (e.g. the
# `claude -p "/usage"` / "/context" calls the watcher spawns). They set this env
# var; without this guard each probe would emit SessionStart/Stop under a fresh
# session id and the HUD would switch its active session to the probe, blanking
# the real session's model / context %.
[ -n "$CLAUDE_COPET_PROBE" ] && exit 0

type="$1"
dir="$HOME/.claude-copet"
log="$dir/events.jsonl"

mkdir -p "$dir" 2>/dev/null

ts=$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null)
input=$(cat 2>/dev/null)

# Claude Code's payload nests a "tool_input" object (always after the session-level
# keys) whose tool arguments can collide with our key names (e.g. a literal "cwd").
# Our sed prefix is greedy, so it would grab the LAST occurrence — strip everything
# from "tool_input" onward first so only the top-level session keys remain.
meta=$(printf '%s' "$input" | sed 's/"tool_input".*//')

tool=$(printf '%s' "$meta" | sed -n 's/.*"tool_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
session=$(printf '%s' "$meta" | sed -n 's/.*"session_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
cwd=$(printf '%s' "$meta" | sed -n 's/.*"cwd"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
transcript=$(printf '%s' "$meta" | sed -n 's/.*"transcript_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)

printf '{"ts":"%s","type":"%s","tool":"%s","session":"%s","cwd":"%s","transcript_path":"%s"}\n' \
  "$ts" "$type" "$tool" "$session" "$cwd" "$transcript" >> "$log" 2>/dev/null

exit 0
