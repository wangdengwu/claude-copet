import { listen } from "@tauri-apps/api/event";
import { animationForMood } from "./animation";
import type { Mood } from "./animation";
import { SHEETS } from "./sprites";
import { startRenderLoop } from "./render";
import { createBubble } from "./bubble";
import { mountSettingsPanel, openSettings } from "./settings";
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

// Mount the settings panel (gear button → overlay).
mountSettingsPanel(document.body);

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

const fiveHourEl = document.createElement("span");
fiveHourEl.className = "hud-usage-line";
const sevenDayEl = document.createElement("span");
sevenDayEl.className = "hud-usage-line";
usageBlock.append(fiveHourEl, sevenDayEl);

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
  fiveHourEl.textContent = usage.fiveHour.text;
  fiveHourEl.dataset.band = usage.fiveHour.band;
  sevenDayEl.textContent = usage.sevenDay.text;
  sevenDayEl.dataset.band = usage.sevenDay.band;
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

// Build a small HTML context menu and attach it to document.body.
const ctxMenu = document.createElement("div");
ctxMenu.id = "ctx-menu";
ctxMenu.style.cssText = [
  "position:fixed;background:rgba(20,20,20,0.92);color:#eee;",
  "font-family:monospace;font-size:12px;border-radius:4px;",
  "border:1px solid #555;padding:4px 0;min-width:140px;",
  "display:none;z-index:200;box-shadow:0 2px 8px rgba(0,0,0,0.5);",
].join("");

function makeItem(label: string, onClick: () => void): HTMLDivElement {
  const item = document.createElement("div");
  item.textContent = label;
  item.style.cssText = "padding:5px 12px;cursor:pointer;";
  item.addEventListener("mouseenter", () => { item.style.background = "rgba(255,255,255,0.1)"; });
  item.addEventListener("mouseleave", () => { item.style.background = ""; });
  item.addEventListener("mousedown", (e) => { e.stopPropagation(); onClick(); hideCtxMenu(); });
  return item;
}

const refreshUsageItem = makeItem("Refresh usage", () => {
  // The 30s throttle is enforced in Rust; the menu just requests a re-fetch.
  void invokeOrNull("refresh_usage");
});

const settingsItem = makeItem("Settings", () => openSettings());

const quitItem = makeItem("Quit", async () => {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  } catch {
    // Fallback: use the Rust command.
    await invokeOrNull("quit_app");
  }
});

ctxMenu.appendChild(refreshUsageItem);
ctxMenu.appendChild(settingsItem);
ctxMenu.appendChild(quitItem);
document.body.appendChild(ctxMenu);

function showCtxMenu(x: number, y: number): void {
  ctxMenu.style.left = `${x}px`;
  ctxMenu.style.top = `${y}px`;
  ctxMenu.style.display = "block";
}

function hideCtxMenu(): void {
  ctxMenu.style.display = "none";
}

card.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  showCtxMenu(e.clientX, e.clientY);
});

document.addEventListener("mousedown", (e) => {
  if (ctxMenu.style.display !== "none" && !ctxMenu.contains(e.target as Node)) {
    hideCtxMenu();
  }
});

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") hideCtxMenu();
});
