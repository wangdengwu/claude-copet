import { listen } from "@tauri-apps/api/event";
import { animationForMood } from "./animation";
import type { Mood } from "./animation";
import { SHEETS } from "./sprites";
import { startRenderLoop } from "./render";
import { createBubble } from "./bubble";
import { formatHud } from "./hud";
import type { HudState } from "./hud";

const card = document.getElementById("card") as HTMLElement;
const canvas = document.getElementById("pet") as HTMLCanvasElement;
const bubble = createBubble(document.getElementById("bubble") as HTMLElement);

// The pet canvas is a fixed box on the left of the card; size its backing store
// to the element's own box (× dpr) so the sprite stays crisp.
function sizeCanvas(): void {
  const dpr = window.devicePixelRatio || 1;
  const rect = canvas.getBoundingClientRect();
  canvas.width = Math.max(1, Math.round(rect.width * dpr));
  canvas.height = Math.max(1, Math.round(rect.height * dpr));
}

sizeCanvas();
window.addEventListener("resize", sizeCanvas);

// The settings panel is now a separate native window (open via right-click →
// Settings); the old HTML overlay has been removed. No gear button to mount.

// Start idle; the Rust core emits "mood" as events flow in from Claude Code.
const controller = startRenderLoop(canvas, SHEETS.idle);

// ─── HUD info column ─────────────────────────────────────────────────────────
// The Rust core emits a "hud" snapshot reflecting the most-recently-active
// session. Slice 2 renders the session label (cwd basename); the full session
// id shows on hover. Later slices add model / context % / activity rows.

const hudInfo = document.getElementById("hud-info") as HTMLElement;

// Top row: session label · model badge.
const topRow = document.createElement("div");
topRow.className = "hud-top";
const labelEl = document.createElement("span");
labelEl.className = "hud-label";
labelEl.textContent = "—";
const modelEl = document.createElement("span");
modelEl.className = "hud-model";
modelEl.textContent = "—";
topRow.append(labelEl, document.createTextNode(" · "), modelEl);

// Context bar: a coloured fill (green/amber/red) + a percent label.
const barRow = document.createElement("div");
barRow.className = "hud-bar";
const barFill = document.createElement("div");
barFill.className = "hud-bar-fill";
const barText = document.createElement("span");
barText.className = "hud-bar-text";
barText.textContent = "—";
barRow.append(barFill, barText);

// Bottom row: current activity, or the needs-human warning when waiting.
const activityRow = document.createElement("div");
activityRow.className = "hud-activity";
activityRow.textContent = "Idle";

// Usage block: a single row with the two windows (5h / 7d) side by side,
// separated by a generous gap. Within each window percent + countdown are one
// unit (no separator). Refresh is in the right-click menu, not an inline button.
const usageBlock = document.createElement("div");
usageBlock.className = "hud-usage";
usageBlock.style.display = "none";

// Each window is a cell with a band-coloured percent + a dimmer/lighter
// countdown, split from the formatted line at the ⏳ marker.
function makeUsageCell(): { cell: HTMLElement; pct: HTMLElement; cd: HTMLElement } {
  const cell = document.createElement("span");
  cell.className = "hud-usage-line";
  const pct = document.createElement("span");
  pct.className = "hud-usage-pct";
  const cd = document.createElement("span");
  cd.className = "hud-usage-cd";
  cell.append(pct, cd);
  return { cell, pct, cd };
}
const fiveHour = makeUsageCell();
const sevenDay = makeUsageCell();
usageBlock.append(fiveHour.cell, sevenDay.cell);

function setUsageCell(sub: { pct: HTMLElement; cd: HTMLElement }, view: { text: string; band: string }): void {
  const i = view.text.indexOf("⏳");
  sub.pct.textContent = i === -1 ? view.text : view.text.slice(0, i).trim();
  sub.pct.dataset.band = view.band;
  sub.cd.textContent = i === -1 ? "" : " " + view.text.slice(i);
}

hudInfo.append(topRow, barRow, activityRow, usageBlock);

// Render the usage block from the latest snapshot, recomputing the remaining-time
// countdown against the current clock. The last snapshot is retained so a timer
// can re-render it between snapshots (the countdown ticks down live; /usage only
// re-fetches every few minutes).
let lastUsageState: HudState | null = null;
function renderUsage(state: HudState): void {
  const usage = formatHud(state).usage;
  if (usage === null) {
    usageBlock.style.display = "none";
    return;
  }
  usageBlock.style.display = "";
  setUsageCell(fiveHour, usage.fiveHour);
  setUsageCell(sevenDay, usage.sevenDay);
}
// Tick the countdown roughly once a minute (it shows minute granularity).
setInterval(() => {
  if (lastUsageState) renderUsage(lastUsageState);
}, 30_000);

listen<HudState>("hud", (event) => {
  const view = formatHud(event.payload);
  labelEl.textContent = view.label;
  labelEl.title = event.payload.sessionId || "";
  modelEl.textContent = view.model;
  barFill.style.width = `${view.barWidthPct}%`;
  barText.textContent = view.contextText;
  barRow.dataset.band = view.colorBand;
  activityRow.textContent = view.activityText;
  // The whole card turns amber and pulses when Claude is waiting on the user.
  card.classList.toggle("needs-human", view.needsHuman);

  // Usage limits block — hidden entirely for non-Claude/API-key setups.
  lastUsageState = event.payload;
  renderUsage(event.payload);
}).catch(() => {
  /* not running inside Tauri — no live session */
});

// listen rejects when no Tauri runtime is present (e.g. plain `vite` in a browser);
// swallow it so the pet still renders idle outside the app shell.
listen<Mood>("mood", (event) => {
  const key = animationForMood(event.payload);
  controller.setSheet(SHEETS[key] ?? SHEETS.idle);
}).catch(() => {
  /* not running inside Tauri — stay idle */
});

// The speech bubble is an optional surface, off by default for the HUD product.
// Kept wired so a future build can emit "speech" again without frontend changes.
listen<string>("speech", (event) => {
  bubble.show(event.payload);
}).catch(() => {
  /* not running inside Tauri — no speech */
});

// ─── Safe Tauri invoke ───────────────────────────────────────────────────────

async function invokeOrNull<T>(cmd: string, args?: unknown): Promise<T | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(cmd, args as Record<string, unknown>);
  } catch {
    return null;
  }
}

// ─── Drag + click-to-pet ─────────────────────────────────────────────────────
// The window is frameless; the whole card is the drag surface. We detect pointer
// movement to distinguish a drag from a click. A drag starts the OS window move
// via startDragging(); a click on the pet invokes pet_clicked on the Rust side.

const DRAG_THRESHOLD_PX = 5;

let pointerDownX = 0;
let pointerDownY = 0;
let downTarget: EventTarget | null = null;
let dragging = false;

card.addEventListener("mousedown", (e) => {
  if (e.button !== 0) return; // only primary button
  pointerDownX = e.clientX;
  pointerDownY = e.clientY;
  downTarget = e.target;
  dragging = false;
});

card.addEventListener("mousemove", async (e) => {
  if (e.buttons !== 1) return; // primary button must be held
  if (dragging) return;
  const dx = e.clientX - pointerDownX;
  const dy = e.clientY - pointerDownY;
  if (Math.sqrt(dx * dx + dy * dy) >= DRAG_THRESHOLD_PX) {
    dragging = true;
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().startDragging();
    } catch {
      /* no Tauri runtime — ignore */
    }
  }
});

card.addEventListener("mouseup", async (e) => {
  if (e.button !== 0) return;
  // A click (no drag) on the pet itself emits Happy + speech.
  if (!dragging && downTarget === canvas) {
    await invokeOrNull("pet_clicked");
  }
  dragging = false;
});

// ─── Context menu ────────────────────────────────────────────────────────────
// Native OS menu popped from Rust — never clipped by the tiny card window.
card.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  void invokeOrNull("show_context_menu");
});
