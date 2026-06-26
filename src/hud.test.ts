import { test, expect } from "vitest";
import { formatHud } from "./hud";

const base = { sessionLabel: "claude-copet", sessionId: "s1", activity: "Idle", needsHuman: false };

const NEEDS_HUMAN_TEXT = "⚠ 等你输入 / 授权";

test("label and model pass through; percent renders as a rounded number", () => {
  const v = formatHud({ ...base, model: "Opus 4.8", contextPercent: 61.7 });
  expect(v.label).toBe("claude-copet");
  expect(v.model).toBe("Opus 4.8");
  expect(v.contextText).toBe("62%");
  expect(v.barWidthPct).toBeCloseTo(61.7);
});

test("colour band: green below 70, amber at the 70 boundary, red at the 90 boundary", () => {
  expect(formatHud({ ...base, model: "x", contextPercent: 0 }).colorBand).toBe("green");
  expect(formatHud({ ...base, model: "x", contextPercent: 69.9 }).colorBand).toBe("green");
  expect(formatHud({ ...base, model: "x", contextPercent: 70 }).colorBand).toBe("amber");
  expect(formatHud({ ...base, model: "x", contextPercent: 89.9 }).colorBand).toBe("amber");
  expect(formatHud({ ...base, model: "x", contextPercent: 90 }).colorBand).toBe("red");
  expect(formatHud({ ...base, model: "x", contextPercent: 100 }).colorBand).toBe("red");
});

test("bar width clamps into [0,100]", () => {
  expect(formatHud({ ...base, model: "x", contextPercent: 150 }).barWidthPct).toBe(100);
  expect(formatHud({ ...base, model: "x", contextPercent: -5 }).barWidthPct).toBe(0);
});

test("null context/model degrade to em dash, zero-width bar, no colour band", () => {
  const v = formatHud({ ...base, model: null, contextPercent: null });
  expect(v.model).toBe("—");
  expect(v.contextText).toBe("—");
  expect(v.barWidthPct).toBe(0);
  expect(v.colorBand).toBe("none");
});

test("empty session label degrades to em dash", () => {
  const v = formatHud({
    sessionLabel: "",
    sessionId: "",
    model: null,
    contextPercent: null,
    activity: "Idle",
    needsHuman: false,
  });
  expect(v.label).toBe("—");
});

test("bottom row shows the activity when no human is needed", () => {
  const v = formatHud({ ...base, model: "x", contextPercent: 10, activity: "Running Bash", needsHuman: false });
  expect(v.activityText).toBe("Running Bash");
  expect(v.needsHuman).toBe(false);
});

test("empty activity falls back to Idle", () => {
  const v = formatHud({ ...base, model: "x", contextPercent: 10, activity: "", needsHuman: false });
  expect(v.activityText).toBe("Idle");
});

test("needs-human overrides the bottom row with the warning line and sets the flag", () => {
  const v = formatHud({ ...base, model: "x", contextPercent: 10, activity: "Running Bash", needsHuman: true });
  expect(v.activityText).toBe(NEEDS_HUMAN_TEXT);
  expect(v.needsHuman).toBe(true);
});

// ─────────────────────────── usage limits (5h / 7d) ──────────────────────────
// The reset phrase is parsed into an instant and shown as TIME REMAINING (not an
// absolute clock/date): 5h → h+m, 7d → d+h. A fixed `now` is injected so the
// countdown is deterministic. Both `now` and the reset are built in the runner's
// local tz, so the difference is tz-independent.

import { parseResetToMs, formatRemaining } from "./hud";

const NOW = new Date(2026, 5, 26, 9, 0, 0).getTime(); // Jun 26 2026, 09:00 local

const usagePayload = {
  sessionPercent: 31,
  sessionReset: "Jun 26 at 11:59pm (Asia/Shanghai)", // 14h59m after NOW
  weekPercent: 77,
  weekReset: "Jun 30 at 3pm (Asia/Shanghai)",        // 4d6h after NOW
};

test("usage shows time remaining with a ⏳ symbol: 5h as h+m, 7d as d+h", () => {
  const v = formatHud({ ...base, model: "x", contextPercent: 10, usage: usagePayload }, NOW);
  expect(v.usage).not.toBeNull();
  expect(v.usage!.fiveHour.text).toBe("5h 31% ⏳ 14h 59m");
  expect(v.usage!.sevenDay.text).toBe("7d 77% ⏳ 4d 6h");
});

test("usage band follows the same thresholds as context (amber ≥70, red ≥90)", () => {
  const v = formatHud({ ...base, model: "x", contextPercent: 10, usage: usagePayload }, NOW);
  expect(v.usage!.fiveHour.band).toBe("green"); // 31%
  expect(v.usage!.sevenDay.band).toBe("amber"); // 77%
  const high = formatHud({
    ...base, model: "x", contextPercent: 10,
    usage: { ...usagePayload, weekPercent: 95 },
  }, NOW);
  expect(high.usage!.sevenDay.band).toBe("red"); // 95%
});

test("usage is null when the payload is absent → frontend hides the block (non-Claude)", () => {
  expect(formatHud({ ...base, model: "x", contextPercent: 10 }, NOW).usage).toBeNull();
  expect(formatHud({ ...base, model: "x", contextPercent: 10, usage: null }, NOW).usage).toBeNull();
  expect(formatHud({ ...base, model: "x", contextPercent: 10, usage: undefined }, NOW).usage).toBeNull();
});

test("an unparseable reset phrase drops the countdown suffix (shows just the percent)", () => {
  const v = formatHud({
    ...base, model: "x", contextPercent: 10,
    usage: { sessionPercent: 5, sessionReset: "soon", weekPercent: 12, weekReset: "later" },
  }, NOW);
  expect(v.usage!.fiveHour.text).toBe("5h 5%");
  expect(v.usage!.sevenDay.text).toBe("7d 12%");
});

test("a reset already in the past clamps to zero (not a negative countdown)", () => {
  const v = formatHud({
    ...base, model: "x", contextPercent: 10,
    usage: { sessionPercent: 31, sessionReset: "Jun 26 at 8am", weekPercent: 77, weekReset: "Jun 30 at 3pm" },
  }, NOW);
  expect(v.usage!.fiveHour.text).toBe("5h 31% ⏳ 0m");
});

test("usage percent is rounded for display", () => {
  const v = formatHud({
    ...base, model: "x", contextPercent: 10,
    usage: { ...usagePayload, sessionPercent: 30.6 as unknown as number },
  }, NOW);
  expect(v.usage!.fiveHour.text).toBe("5h 31% ⏳ 14h 59m");
});

// ── parseResetToMs / formatRemaining (pure helpers) ──

test("parseResetToMs handles am/pm, 24h, and the year-wrap into January", () => {
  const noon = parseResetToMs("Jun 30 at 12pm (Asia/Shanghai)", NOW)!;
  expect(new Date(noon).getHours()).toBe(12);
  const midnight = parseResetToMs("Jun 30 at 12am", NOW)!;
  expect(new Date(midnight).getHours()).toBe(0);
  const h24 = parseResetToMs("Jun 30 at 23:59", NOW)!;
  expect(new Date(h24).getHours()).toBe(23);
  expect(new Date(h24).getMinutes()).toBe(59);

  // Late December → a January reset must roll to next year (positive remaining).
  const dec = new Date(2026, 11, 30, 10, 0, 0).getTime();
  const jan = parseResetToMs("Jan 2 at 9am", dec)!;
  expect(new Date(jan).getFullYear()).toBe(2027);
  expect(jan).toBeGreaterThan(dec);
});

test("parseResetToMs returns null for a phrase with no parseable date/time", () => {
  expect(parseResetToMs("tomorrow", NOW)).toBeNull();
  expect(parseResetToMs("in 2 hours", NOW)).toBeNull();
  expect(parseResetToMs("", NOW)).toBeNull();
});

test("formatRemaining: ⏳ prefix, session=h+m, week=d+h, drops a zero high unit, past clamps to 0", () => {
  const min = 60_000, hr = 60 * min, day = 24 * hr;
  expect(formatRemaining(NOW + 2 * hr + 15 * min, NOW, "session")).toBe("⏳ 2h 15m");
  expect(formatRemaining(NOW + 45 * min, NOW, "session")).toBe("⏳ 45m");
  expect(formatRemaining(NOW + 3 * day + 8 * hr, NOW, "week")).toBe("⏳ 3d 8h");
  expect(formatRemaining(NOW + 8 * hr, NOW, "week")).toBe("⏳ 8h");
  expect(formatRemaining(NOW - min, NOW, "session")).toBe("⏳ 0m");
  expect(formatRemaining(NOW - min, NOW, "week")).toBe("⏳ 0h");
});
