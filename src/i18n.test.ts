import { test, expect } from "vitest";
import { t, messages } from "./i18n";

test("t() returns the locale-specific string", () => {
  expect(t("en", "idle")).toBe("Idle");
  expect(t("zh", "idle")).toBe("空闲");
  expect(t("en", "needsHuman")).toBe("⚠ Waiting for input / approval");
  expect(t("zh", "needsHuman")).toBe("⚠ 等你输入 / 授权");
});

// A key present in one locale but missing in the other is a shippable bug: the
// UI would show a blank or fall through. Pin that the two maps stay in lockstep.
test("en and zh expose the exact same key set", () => {
  const en = Object.keys(messages.en).sort();
  const zh = Object.keys(messages.zh).sort();
  expect(zh).toEqual(en);
});
