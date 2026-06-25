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

const mood: Mood = "idle";
const key = animationForMood(mood);
const sheet = SHEETS[key];

startRenderLoop(canvas, sheet);
