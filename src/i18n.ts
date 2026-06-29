// Shared TypeScript string table. Single source of truth for all chrome strings
// shown in the pet window and settings window. Data-derived strings (session
// labels, model names, activity text from Claude) are NOT translated here.

export type Locale = "en" | "zh";

// Derive the key union from the en map so both maps are forced to share keys.
const _en = {
  // HUD card
  needsHuman: "⚠ Waiting for input / approval",
  idle: "Idle",
  // Settings window. Brand / product names route through the table for a single
  // source of truth but stay identical across locales (proper nouns).
  appName: "claude-copet",
  claudeCode: "Claude Code",
  usageRefresh: "Usage Refresh",
  connected: "● Connected",
  notConnected: "○ Not connected",
  interval: "Interval",
  minUnit: "min",
} as const;

export type MessageKey = keyof typeof _en;

export const messages: Record<Locale, Record<MessageKey, string>> = {
  en: _en,
  zh: {
    needsHuman: "⚠ 等你输入 / 授权",
    idle: "空闲",
    appName: "claude-copet",
    claudeCode: "Claude Code",
    usageRefresh: "用量刷新",
    connected: "● 已连接",
    notConnected: "○ 未连接",
    interval: "刷新间隔",
    minUnit: "分钟",
  },
};

/** Look up a localised string. Falls back silently to the key when missing. */
export function t(locale: Locale, key: MessageKey): string {
  return messages[locale][key];
}
