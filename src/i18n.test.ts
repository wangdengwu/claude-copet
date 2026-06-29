import { test, expect } from "vitest";
import { t, messages } from "./i18n";

test("t() returns the locale-specific string", () => {
  expect(t("en", "idle")).toBe("Idle");
  expect(t("zh", "idle")).toBe("空闲");
  expect(t("en", "needsHuman")).toBe("⚠ Waiting for input / approval");
  expect(t("zh", "needsHuman")).toBe("⚠ 等你输入 / 授权");
});

test("settings-window chrome strings are localized; brand names are not", () => {
  // Genuine chrome translates.
  expect(t("en", "usageRefresh")).toBe("Usage Refresh");
  expect(t("zh", "usageRefresh")).toBe("用量刷新");
  expect(t("en", "interval")).toBe("Interval");
  expect(t("zh", "interval")).toBe("刷新间隔");
  expect(t("en", "minUnit")).toBe("min");
  expect(t("zh", "minUnit")).toBe("分钟");
  expect(t("en", "connected")).toBe("● Connected");
  expect(t("zh", "connected")).toBe("● 已连接");
  expect(t("en", "notConnected")).toBe("○ Not connected");
  expect(t("zh", "notConnected")).toBe("○ 未连接");
  // Brand / product names route through the table but stay identical (proper nouns).
  expect(t("zh", "appName")).toBe(t("en", "appName"));
  expect(t("zh", "claudeCode")).toBe(t("en", "claudeCode"));
});

// A key present in one locale but missing in the other is a shippable bug: the
// UI would show a blank or fall through. Pin that the two maps stay in lockstep.
test("en and zh expose the exact same key set", () => {
  const en = Object.keys(messages.en).sort();
  const zh = Object.keys(messages.zh).sort();
  expect(zh).toEqual(en);
});
