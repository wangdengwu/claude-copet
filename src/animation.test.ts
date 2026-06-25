import { test, expect } from "vitest";
import { animationForMood } from "./animation";

test("animationForMood returns 'idle' for idle mood", () => {
  expect(animationForMood("idle")).toBe("idle");
});
