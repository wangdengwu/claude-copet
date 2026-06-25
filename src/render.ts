import type { SpriteSheet } from "./sprites";

export function startRenderLoop(canvas: HTMLCanvasElement, sheet: SpriteSheet): void {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  // Capture the non-null ctx in a const so the closure retains the narrowed type.
  const context: CanvasRenderingContext2D = ctx;

  const img = new Image();
  img.src = sheet.src;

  img.addEventListener("load", () => {
    const frameDuration = 1000 / sheet.fps;
    let frameIndex = 0;
    let accumulator = 0;
    let lastTimestamp: number | null = null;

    // The canvas backing-store size is owned by main.ts (set on start + resize);
    // we just read the live canvas.width/height each frame.
    function computeDrawRect(): { dx: number; dy: number; dw: number; dh: number } {
      const cw = canvas.width;
      const ch = canvas.height;
      const scale = Math.max(1, Math.floor(Math.min(cw / sheet.frameW, ch / sheet.frameH)));
      const dw = sheet.frameW * scale;
      const dh = sheet.frameH * scale;
      const dx = Math.floor((cw - dw) / 2);
      const dy = Math.floor((ch - dh) / 2);
      return { dx, dy, dw, dh };
    }

    function tick(timestamp: number): void {
      if (lastTimestamp !== null) {
        const delta = timestamp - lastTimestamp;
        accumulator += delta;
        while (accumulator >= frameDuration) {
          frameIndex = (frameIndex + 1) % sheet.frames;
          accumulator -= frameDuration;
        }
      }
      lastTimestamp = timestamp;

      context.clearRect(0, 0, canvas.width, canvas.height);
      context.imageSmoothingEnabled = false;

      const { dx, dy, dw, dh } = computeDrawRect();
      const sx = frameIndex * sheet.frameW;
      context.drawImage(img, sx, 0, sheet.frameW, sheet.frameH, dx, dy, dw, dh);

      requestAnimationFrame(tick);
    }

    requestAnimationFrame(tick);
  });
}
