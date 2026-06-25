// Mood → animation key seam for future slices
export type Mood = "wake" | "listen" | "work" | "panic" | "happy" | "idle" | "sleep" | "tired";
export type AnimationKey = string;

const moodToAnimation: Record<Mood, AnimationKey> = {
  idle: "idle",
  wake: "wake",
  listen: "listen",
  work: "work",
  panic: "panic",
  happy: "happy",
  sleep: "sleep",
  tired: "tired",
};

export function animationForMood(mood: Mood): AnimationKey {
  return moodToAnimation[mood] ?? "idle";
}
