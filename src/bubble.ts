// Pixel speech bubble above the sprite. Shows a line, then fades after a few
// seconds. Pure DOM — the line text comes from the Rust speaker via a "speech" event.

const VISIBLE_MS = 3500; // how long the line stays fully shown before fading

export function createBubble(el: HTMLElement) {
  let hideTimer: number | undefined;

  return {
    show(text: string): void {
      el.textContent = text;
      el.dataset.show = "true";
      if (hideTimer !== undefined) clearTimeout(hideTimer);
      // Fade out after a beat; the CSS opacity transition does the actual fade.
      hideTimer = window.setTimeout(() => {
        el.dataset.show = "false";
      }, VISIBLE_MS);
    },
  };
}
