// A sprite sheet is a horizontal strip of `frames` cells, each `frameW×frameH`px,
// frame 0 leftmost; played left-to-right at `fps`, looping. Transparent background.

import type { AnimationKey } from "./animation";
import idleUrl from "./sprites/idle.png";

export interface SpriteSheet {
  src: string;
  frameW: number;
  frameH: number;
  frames: number;
  fps: number;
}

export const SHEETS: Record<AnimationKey, SpriteSheet> = {
  idle: { src: idleUrl, frameW: 32, frameH: 32, frames: 4, fps: 6 },
};
