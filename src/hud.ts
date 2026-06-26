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

/**
 * Extract the TIME part from a reset phrase like "Jun 26 at 11:59pm (Asia/Shanghai)".
 * Returns the text after " at " with any trailing " (…)" timezone removed.
 * Falls back to the whole phrase (timezone stripped) when there is no " at ".
 */
function compactTime(reset: string): string {
  const atIdx = reset.indexOf(" at ");
  let part = atIdx !== -1 ? reset.slice(atIdx + 4) : reset;
  // Remove trailing " (…)" timezone annotation.
  part = part.replace(/\s*\(.*\)\s*$/, "").trim();
  return part;
}

/**
 * Extract the DATE part from a reset phrase like "Jun 30 at 3pm (Asia/Shanghai)".
 * Returns the text before " at " with any trailing " (…)" timezone removed.
 * Falls back to the whole phrase (timezone stripped) when there is no " at ".
 */
function compactDate(reset: string): string {
  const atIdx = reset.indexOf(" at ");
  let part = atIdx !== -1 ? reset.slice(0, atIdx) : reset;
  // Remove trailing " (…)" timezone annotation (only relevant for the fallback path).
  part = part.replace(/\s*\(.*\)\s*$/, "").trim();
  return part;
}

export function formatHud(state: HudState): HudView {
  const raw = state.contextPercent;
  const hasPct = raw !== null && raw !== undefined && !Number.isNaN(raw);
  const pct = hasPct ? Math.max(0, Math.min(100, raw as number)) : 0;

  // Build usage lines only when a payload is present.
  let usage: HudView["usage"] = null;
  if (state.usage != null) {
    const { sessionPercent, sessionReset, weekPercent, weekReset } = state.usage;
    usage = {
      fiveHour: {
        text: `5h ${Math.round(sessionPercent)}% · ${compactTime(sessionReset)}`,
        band: bandFor(sessionPercent),
      },
      sevenDay: {
        text: `7d ${Math.round(weekPercent)}% · ${compactDate(weekReset)}`,
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
