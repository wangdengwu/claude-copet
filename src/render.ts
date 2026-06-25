import type { SpriteSheet } from "./sprites";

export interface RenderController {
  /** Swap the playing sheet (e.g. on a mood change). Restarts the loop at frame 0. */
  setSheet(sheet: SpriteSheet): void;
}

export function startRenderLoop(canvas: HTMLCanvasElement, sheet: SpriteSheet): RenderController {
  const ctx = canvas.getContext("2d");
  if (!ctx) return { setSheet() {} };

  // Capture the non-null ctx in a const so the closure retains the narrowed type.
  const context: CanvasRenderingContext2D = ctx;

  // Cache one Image per src so swapping back to a seen sheet is instant.
  const images = new Map<string, HTMLImageElement>();
  function imageFor(s: SpriteSheet): HTMLImageElement {
    let img = images.get(s.src);
    if (!img) {
      img = new Image();
      img.src = s.src;
      images.set(s.src, img);
    }
    return img;
  }

  let current = sheet;
  let img = imageFor(current);
  let frameIndex = 0;
  let accumulator = 0;
  let lastTimestamp: number | null = null;

  // The canvas backing-store size is owned by main.ts (set on start + resize);
  // we just read the live canvas.width/height each frame.
  function computeDrawRect(): { dx: number; dy: number; dw: number; dh: number } {
    const cw = canvas.width;
    const ch = canvas.height;
    const scale = Math.max(1, Math.floor(Math.min(cw / current.frameW, ch / current.frameH)));
    const dw = current.frameW * scale;
    const dh = current.frameH * scale;
    const dx = Math.floor((cw - dw) / 2);
    const dy = Math.floor((ch - dh) / 2);
    return { dx, dy, dw, dh };
  }

  function tick(timestamp: number): void {
    const frameDuration = 1000 / current.fps;
    if (lastTimestamp !== null) {
      const delta = timestamp - lastTimestamp;
      accumulator += delta;
      while (accumulator >= frameDuration) {
        frameIndex = (frameIndex + 1) % current.frames;
        accumulator -= frameDuration;
      }
    }
    lastTimestamp = timestamp;

    context.clearRect(0, 0, canvas.width, canvas.height);
    context.imageSmoothingEnabled = false;

    if (img.complete && img.naturalWidth > 0) {
      const { dx, dy, dw, dh } = computeDrawRect();
      const sx = frameIndex * current.frameW;
      context.drawImage(img, sx, 0, current.frameW, current.frameH, dx, dy, dw, dh);
    }

    requestAnimationFrame(tick);
  }

  requestAnimationFrame(tick);

  return {
    setSheet(next: SpriteSheet): void {
      if (next.src === current.src) return;
      current = next;
      img = imageFor(next);
      frameIndex = 0;
      accumulator = 0;
    },
  };
}
