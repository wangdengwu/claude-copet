import { listen } from "@tauri-apps/api/event";
import { animationForMood } from "./animation";
import type { Mood, Stage } from "./animation";
import { SHEETS } from "./sprites";
import { startRenderLoop } from "./render";
import { createBubble } from "./bubble";
import { mountSettingsPanel, openSettings } from "./settings";

const canvas = document.getElementById("pet") as HTMLCanvasElement;
const bubble = createBubble(document.getElementById("bubble") as HTMLElement);

function sizeCanvas(): void {
  const dpr = window.devicePixelRatio || 1;
  canvas.width = window.innerWidth * dpr;
  canvas.height = window.innerHeight * dpr;
  canvas.style.width = `${window.innerWidth}px`;
  canvas.style.height = `${window.innerHeight}px`;
}

sizeCanvas();
window.addEventListener("resize", sizeCanvas);

// Mount the settings panel (gear button → overlay).
mountSettingsPanel(document.body);

// Start idle; the Rust core emits "mood" as events flow in from Claude Code.
const controller = startRenderLoop(canvas, SHEETS.idle);
let currentStage: Stage = "egg";

// ─── Pet state (slice 4 daily stats) ────────────────────────────────────────

interface DailyStats {
  sessions: number;
  tool_calls: number;
  turns: number;
  errors: number;
}

interface PetStatePayload {
  daily_stats?: Record<string, DailyStats>;
  [key: string]: unknown;
}

let latestPetState: PetStatePayload | null = null;

// listen rejects when no Tauri runtime is present (e.g. plain `vite` in a browser);
// swallow it so the pet still renders idle outside the app shell.
listen<Mood>("mood", (event) => {
  const key = animationForMood(event.payload);
  controller.setSheet(SHEETS[key] ?? SHEETS.idle);
}).catch(() => {
  /* not running inside Tauri — stay idle */
});

// The Rust speaker emits a handwritten line on mood entry; show it in the bubble.
listen<string>("speech", (event) => {
  bubble.show(event.payload);
}).catch(() => {
  /* not running inside Tauri — no speech */
});

// Growth: the Rust core emits the full pet state on startup and stage on evolution.
listen<Stage>("stage", (event) => {
  currentStage = event.payload;
  console.log(`[copet] stage → ${currentStage}`);
}).catch(() => {
  /* not running inside Tauri */
});

// pet_state carries the full { pet, daily_stats, cursor } on load.
listen<PetStatePayload>("pet_state", (event) => {
  latestPetState = event.payload;
  console.log("[copet] pet state loaded", event.payload);
}).catch(() => {
  /* not running inside Tauri */
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
// The window is frameless; we detect pointer movement to distinguish a drag
// from a click. A drag starts the OS window move via startDragging().
// A click (no appreciable movement) invokes pet_clicked on the Rust side.

const DRAG_THRESHOLD_PX = 5;

let pointerDownX = 0;
let pointerDownY = 0;
let dragging = false;

canvas.addEventListener("mousedown", (e) => {
  if (e.button !== 0) return; // only primary button
  pointerDownX = e.clientX;
  pointerDownY = e.clientY;
  dragging = false;
});

canvas.addEventListener("mousemove", async (e) => {
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

canvas.addEventListener("mouseup", async (e) => {
  if (e.button !== 0) return;
  if (!dragging) {
    // It's a click — tell Rust to emit Happy + speech.
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

// "Settings" — always present (slice 5 is built).
const settingsItem = makeItem("Settings", () => openSettings());

// "Today's stats" overlay — always present (slice 4 is built).
const statsItem = makeItem("Today's stats", () => showStats());

// "Quit" — always present.
const quitItem = makeItem("Quit", async () => {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  } catch {
    // Fallback: use the Rust command.
    await invokeOrNull("quit_app");
  }
});

ctxMenu.appendChild(settingsItem);
ctxMenu.appendChild(statsItem);
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

canvas.addEventListener("contextmenu", (e) => {
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

// ─── Today's stats overlay ───────────────────────────────────────────────────

const statsOverlay = document.createElement("div");
statsOverlay.id = "stats-overlay";
statsOverlay.style.cssText = [
  "position:fixed;top:8px;left:8px;background:rgba(0,0,0,0.8);",
  "color:#eee;font-family:monospace;font-size:11px;padding:8px;",
  "border-radius:4px;display:none;z-index:150;min-width:140px;",
].join("");
document.body.appendChild(statsOverlay);

function showStats(): void {
  const today = todayString();
  const stats = latestPetState?.daily_stats?.[today];

  if (!stats) {
    statsOverlay.innerHTML = `<b>Today's stats</b><br>No data yet.`;
  } else {
    statsOverlay.innerHTML = [
      `<b>Today's stats</b>`,
      `Sessions:   ${stats.sessions}`,
      `Tool calls: ${stats.tool_calls}`,
      `Turns:      ${stats.turns}`,
      `Errors:     ${stats.errors}`,
    ].join("<br>");
  }

  statsOverlay.style.display = "block";

  // Auto-hide after 4 seconds.
  setTimeout(() => { statsOverlay.style.display = "none"; }, 4000);
}

// Close stats overlay on outside click.
document.addEventListener("mousedown", (e) => {
  if (
    statsOverlay.style.display !== "none" &&
    !statsOverlay.contains(e.target as Node)
  ) {
    statsOverlay.style.display = "none";
  }
});

// ─── Tiny date helper (no chrono dep) ────────────────────────────────────────
// Mirrors the Rust today_string() — YYYY-MM-DD in UTC.
function todayString(): string {
  const d = new Date();
  const y = d.getUTCFullYear();
  const m = String(d.getUTCMonth() + 1).padStart(2, "0");
  const day = String(d.getUTCDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}
