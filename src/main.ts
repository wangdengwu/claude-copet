import { listen } from "@tauri-apps/api/event";
import { animationForMood } from "./animation";
import type { Mood, Stage } from "./animation";
import { SHEETS } from "./sprites";
import { startRenderLoop } from "./render";
import { createBubble } from "./bubble";
import { mountSettingsPanel } from "./settings";

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
listen<unknown>("pet_state", (event) => {
  console.log("[copet] pet state loaded", event.payload);
}).catch(() => {
  /* not running inside Tauri */
});
