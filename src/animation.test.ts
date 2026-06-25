import { test, expect } from "vitest";
import { animationForMood } from "./animation";

test("animationForMood returns 'idle' for idle mood", () => {
  expect(animationForMood("idle")).toBe("idle");
});

test("animationForMood maps each of the 8 moods to its own distinct key", () => {
  const expected: Array<[Parameters<typeof animationForMood>[0], string]> = [
    ["idle",   "idle"],
    ["wake",   "wake"],
    ["listen", "listen"],
    ["work",   "work"],
    ["panic",  "panic"],
    ["happy",  "happy"],
    ["sleep",  "sleep"],
    ["tired",  "tired"],
  ];

  for (const [mood, key] of expected) {
    expect(animationForMood(mood), `mood "${mood}"`).toBe(key);
  }

  const keys = expected.map(([mood]) => animationForMood(mood));
  const uniqueKeys = new Set(keys);
  expect(uniqueKeys.size).toBe(8);
});
