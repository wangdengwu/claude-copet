import { listen } from "@tauri-apps/api/event";
import { animationForMood } from "./animation";
import type { Mood } from "./animation";
import { SHEETS } from "./sprites";
import { startRenderLoop } from "./render";

const canvas = document.getElementById("pet") as HTMLCanvasElement;

function sizeCanvas(): void {
  const dpr = window.devicePixelRatio || 1;
  canvas.width = window.innerWidth * dpr;
  canvas.height = window.innerHeight * dpr;
  canvas.style.width = `${window.innerWidth}px`;
  canvas.style.height = `${window.innerHeight}px`;
}

sizeCanvas();
window.addEventListener("resize", sizeCanvas);

// Start idle; the Rust core emits "mood" as events flow in from Claude Code.
const controller = startRenderLoop(canvas, SHEETS.idle);

// listen rejects when no Tauri runtime is present (e.g. plain `vite` in a browser);
// swallow it so the pet still renders idle outside the app shell.
listen<Mood>("mood", (event) => {
  const key = animationForMood(event.payload);
  controller.setSheet(SHEETS[key] ?? SHEETS.idle);
}).catch(() => {
  /* not running inside Tauri — stay idle */
});
