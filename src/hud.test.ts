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

const usagePayload = {
  sessionPercent: 31,
  sessionReset: "Jun 26 at 11:59pm (Asia/Shanghai)",
  weekPercent: 77,
  weekReset: "Jun 30 at 3pm (Asia/Shanghai)",
};

test("usage renders two compact lines: 5h shows the time, 7d shows the date", () => {
  const v = formatHud({ ...base, model: "x", contextPercent: 10, usage: usagePayload });
  expect(v.usage).not.toBeNull();
  expect(v.usage!.fiveHour.text).toBe("5h 31% · 11:59pm");
  expect(v.usage!.sevenDay.text).toBe("7d 77% · Jun 30");
});

test("usage band follows the same thresholds as context (amber ≥70, red ≥90)", () => {
  const v = formatHud({ ...base, model: "x", contextPercent: 10, usage: usagePayload });
  expect(v.usage!.fiveHour.band).toBe("green"); // 31%
  expect(v.usage!.sevenDay.band).toBe("amber"); // 77%
  const high = formatHud({
    ...base, model: "x", contextPercent: 10,
    usage: { ...usagePayload, weekPercent: 95 },
  });
  expect(high.usage!.sevenDay.band).toBe("red"); // 95%
});

test("usage is null when the payload is absent → frontend hides the block (non-Claude)", () => {
  expect(formatHud({ ...base, model: "x", contextPercent: 10 }).usage).toBeNull();
  expect(formatHud({ ...base, model: "x", contextPercent: 10, usage: null }).usage).toBeNull();
  expect(formatHud({ ...base, model: "x", contextPercent: 10, usage: undefined }).usage).toBeNull();
});

test("usage reset formatting degrades gracefully when the phrase has no ' at '", () => {
  const v = formatHud({
    ...base, model: "x", contextPercent: 10,
    usage: { sessionPercent: 5, sessionReset: "tomorrow", weekPercent: 12, weekReset: "Monday" },
  });
  // No " at " separator → fall back to the whole phrase rather than dropping it.
  expect(v.usage!.fiveHour.text).toBe("5h 5% · tomorrow");
  expect(v.usage!.sevenDay.text).toBe("7d 12% · Monday");
});

test("usage percent is rounded for display", () => {
  const v = formatHud({
    ...base, model: "x", contextPercent: 10,
    usage: { ...usagePayload, sessionPercent: 30.6 as unknown as number },
  });
  expect(v.usage!.fiveHour.text).toBe("5h 31% · 11:59pm");
});
