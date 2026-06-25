// Mood → animation key seam for future slices
export type Mood = "wake" | "listen" | "work" | "panic" | "happy" | "idle" | "sleep" | "tired";
export type AnimationKey = string;

const moodToAnimation: Record<Mood, AnimationKey> = {
  idle: "idle",
  wake: "idle",
  listen: "idle",
  work: "idle",
  panic: "idle",
  happy: "idle",
  sleep: "idle",
  tired: "idle",
};

export function animationForMood(mood: Mood): AnimationKey {
  return moodToAnimation[mood] ?? "idle";
}
