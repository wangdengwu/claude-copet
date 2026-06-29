// Shared TypeScript string table. Single source of truth for all chrome strings
// shown in the pet window and settings window. Data-derived strings (session
// labels, model names, activity text from Claude) are NOT translated here.

export type Locale = "en" | "zh";

// Derive the key union from the en map so both maps are forced to share keys.
const _en = {
  needsHuman: "⚠ Waiting for input / approval",
  idle: "Idle",
} as const;

export type MessageKey = keyof typeof _en;

export const messages: Record<Locale, Record<MessageKey, string>> = {
  en: _en,
  zh: {
    needsHuman: "⚠ 等你输入 / 授权",
    idle: "空闲",
  },
};

/** Look up a localised string. Falls back silently to the key when missing. */
export function t(locale: Locale, key: MessageKey): string {
  return messages[locale][key];
}
