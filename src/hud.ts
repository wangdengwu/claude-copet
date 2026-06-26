// Pure mapping from the Rust "hud" snapshot to the card's view model. The only
// frontend seam: decides the context-bar fill width, the colour band, the model
// badge, and the "—" degradation when a field is unavailable. No DOM here.

export interface HudState {
  sessionLabel: string;
  sessionId: string;
  model: string | null;
  contextPercent: number | null;
  activity: string;
  needsHuman: boolean;
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

export function formatHud(state: HudState): HudView {
  const raw = state.contextPercent;
  const hasPct = raw !== null && raw !== undefined && !Number.isNaN(raw);
  const pct = hasPct ? Math.max(0, Math.min(100, raw as number)) : 0;

  return {
    label: state.sessionLabel || "—",
    model: state.model || "—",
    contextText: hasPct ? `${Math.round(pct)}%` : "—",
    barWidthPct: pct,
    colorBand: hasPct ? bandFor(pct) : "none",
    activityText: state.needsHuman ? NEEDS_HUMAN_TEXT : (state.activity || "Idle"),
    needsHuman: !!state.needsHuman,
  };
}
