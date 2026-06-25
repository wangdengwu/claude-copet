// A sprite sheet is a horizontal strip of `frames` cells, each `frameW×frameH`px,
// frame 0 leftmost; played left-to-right at `fps`, looping. Transparent background.

import type { AnimationKey } from "./animation";
import idleUrl from "./sprites/idle.png";
import wakeUrl from "./sprites/wake.png";
import listenUrl from "./sprites/listen.png";
import workUrl from "./sprites/work.png";
import panicUrl from "./sprites/panic.png";
import happyUrl from "./sprites/happy.png";
import sleepUrl from "./sprites/sleep.png";
import tiredUrl from "./sprites/tired.png";

export interface SpriteSheet {
  src: string;
  frameW: number;
  frameH: number;
  frames: number;
  fps: number;
}

// Every mood sheet shares the asset contract (4 frames of 32×32, 6fps); they
// differ only by color. Keys match `animationForMood`'s outputs.
const meta = { frameW: 32, frameH: 32, frames: 4, fps: 6 } as const;

export const SHEETS: Record<AnimationKey, SpriteSheet> = {
  idle: { src: idleUrl, ...meta },
  wake: { src: wakeUrl, ...meta },
  listen: { src: listenUrl, ...meta },
  work: { src: workUrl, ...meta },
  panic: { src: panicUrl, ...meta },
  happy: { src: happyUrl, ...meta },
  sleep: { src: sleepUrl, ...meta },
  tired: { src: tiredUrl, ...meta },
};
