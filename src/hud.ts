// Pure mapping from the Rust "hud" snapshot to the card's view model. The only
// frontend seam: decides the context-bar fill width, the colour band, the model
// badge, and the "—" degradation when a field is unavailable. No DOM here.

export interface UsagePayload {
  sessionPercent: number;
  sessionReset: string;
  weekPercent: number;
  weekReset: string;
}

export interface UsageLine { text: string; band: ColorBand; }

export interface HudState {
  sessionLabel: string;
  sessionId: string;
  model: string | null;
  contextPercent: number | null;
  activity: string;
  needsHuman: boolean;
  usage?: UsagePayload | null;
}

export type ColorBand = "green" | "amber" | "red" | "none";

export interface HudView {
  label: string;
  model: string;
  contextText: string;
  barWidthPct: number;
  colorBand: ColorBand;
  activityText: string;
  needsHuman: boolean;
  usage: { fiveHour: UsageLine; sevenDay: UsageLine } | null;
}

// Warning line shown when Claude is waiting on the user (matches the PRD).
const NEEDS_HUMAN_TEXT = "⚠ 等你输入 / 授权";

// Colour thresholds: green headroom, amber caution, red near-full.
const AMBER_AT = 70;
const RED_AT = 90;

function bandFor(pct: number): ColorBand {
  if (pct >= RED_AT) return "red";
  if (pct >= AMBER_AT) return "amber";
  return "green";
}

const MONTHS: Record<string, number> = {
  jan: 0, feb: 1, mar: 2, apr: 3, may: 4, jun: 5,
  jul: 6, aug: 7, sep: 8, oct: 9, nov: 10, dec: 11,
};

/** Parse a "11:59pm" / "3pm" / "12am" / "23:59" clock token into 24h h+min. */
function parseClock(s: string): { h: number; min: number } | null {
  const ampm = s.match(/^(\d{1,2})(?::(\d{2}))?\s*(am|pm)$/i);
  if (ampm) {
    let h = parseInt(ampm[1], 10) % 12; // 12am → 0, 12pm → 12 (after +12)
    if (/pm/i.test(ampm[3])) h += 12;
    return { h, min: ampm[2] ? parseInt(ampm[2], 10) : 0 };
  }
  const h24 = s.match(/^(\d{1,2}):(\d{2})$/);
  if (h24) return { h: parseInt(h24[1], 10), min: parseInt(h24[2], 10) };
  return null;
}

/**
 * Parse a reset phrase like "Jun 26 at 11:59pm (Asia/Shanghai)" into an epoch-ms
 * instant. The year is absent in the phrase, so it's inferred from `nowMs`
 * (rolling to next year when the date is far in the past — the Dec→Jan wrap).
 * The timezone parenthetical is dropped and the wall-clock time is interpreted
 * in the machine-local tz (assumed equal to the phrase's tz). Returns `null`
 * when the phrase has no parseable "<Mon> <day> at <time>" shape.
 */
export function parseResetToMs(phrase: string, nowMs: number): number | null {
  const p = phrase.replace(/\s*\(.*\)\s*$/, "").trim(); // drop trailing " (tz)"
  const m = p.match(/^([A-Za-z]{3,})\s+(\d{1,2})\s+at\s+(.+)$/);
  if (!m) return null;
  const mon = MONTHS[m[1].slice(0, 3).toLowerCase()];
  if (mon === undefined) return null;
  const day = parseInt(m[2], 10);
  const clock = parseClock(m[3].trim());
  if (!clock) return null;

  const year = new Date(nowMs).getFullYear();
  let target = new Date(year, mon, day, clock.h, clock.min, 0, 0);
  // Far in the past → it's next year's date (the year boundary).
  if (target.getTime() - nowMs < -2 * 24 * 3600 * 1000) {
    target = new Date(year + 1, mon, day, clock.h, clock.min, 0, 0);
  }
  return target.getTime();
}

/**
 * Format the time remaining until `targetMs`. `session` (5h window) → hours+
 * minutes; `week` (7d window) → days+hours. The larger unit is dropped when
 * zero. A non-positive remaining shows "已重置" (just reset).
 */
export function formatRemaining(targetMs: number, nowMs: number, kind: "session" | "week"): string {
  const diff = targetMs - nowMs;
  if (diff <= 0) return "已重置";
  const totalMin = Math.floor(diff / 60_000);
  if (kind === "session") {
    const h = Math.floor(totalMin / 60);
    const m = totalMin % 60;
    return h > 0 ? `还剩 ${h}h ${m}m` : `还剩 ${m}m`;
  }
  const totalH = Math.floor(totalMin / 60);
  const d = Math.floor(totalH / 24);
  const h = totalH % 24;
  return d > 0 ? `还剩 ${d}d ${h}h` : `还剩 ${h}h`;
}

/** Build a usage line "5h 31% · 还剩 2h 15m", or just "5h 31%" when the reset
 *  phrase can't be parsed into an instant. */
function usageLine(prefix: string, pct: number, reset: string, nowMs: number, kind: "session" | "week"): string {
  const base = `${prefix} ${Math.round(pct)}%`;
  const target = parseResetToMs(reset, nowMs);
  return target === null ? base : `${base} · ${formatRemaining(target, nowMs, kind)}`;
}

export function formatHud(state: HudState, nowMs: number = Date.now()): HudView {
  const raw = state.contextPercent;
  const hasPct = raw !== null && raw !== undefined && !Number.isNaN(raw);
  const pct = hasPct ? Math.max(0, Math.min(100, raw as number)) : 0;

  // Build usage lines only when a payload is present.
  let usage: HudView["usage"] = null;
  if (state.usage != null) {
    const { sessionPercent, sessionReset, weekPercent, weekReset } = state.usage;
    usage = {
      fiveHour: {
        text: usageLine("5h", sessionPercent, sessionReset, nowMs, "session"),
        band: bandFor(sessionPercent),
      },
      sevenDay: {
        text: usageLine("7d", weekPercent, weekReset, nowMs, "week"),
        band: bandFor(weekPercent),
      },
    };
  }

  return {
    label: state.sessionLabel || "—",
    model: state.model || "—",
    contextText: hasPct ? `${Math.round(pct)}%` : "—",
    barWidthPct: pct,
    colorBand: hasPct ? bandFor(pct) : "none",
    activityText: state.needsHuman ? NEEDS_HUMAN_TEXT : (state.activity || "Idle"),
    needsHuman: !!state.needsHuman,
    usage,
  };
}
