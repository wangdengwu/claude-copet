// Standalone settings window — no pet, no card, just the settings controls.
// The same invokeOrNull pattern as main.ts so a plain vite dev without Tauri
// doesn't crash. All static chrome is localized via the shared i18n table and
// re-applied live on the "locale" event, without rebuilding the DOM — so the
// selected interval and the shown hook status survive a language switch.

import { listen } from "@tauri-apps/api/event";
import { t, type Locale } from "./i18n";

async function invokeOrNull<T>(cmd: string, args?: unknown): Promise<T | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(cmd, args as Record<string, unknown>);
  } catch {
    return null;
  }
}

const root = document.getElementById("settings-root")!;

// Active locale (English until the persisted setting / a "locale" event says
// otherwise) and the last known hook-install state, retained so a language
// switch can re-render the status line without re-fetching.
let locale: Locale = "en";
let lastInstalled: boolean | null = null;

// The DOM is built ONCE; label text is filled in by applyLabels(). Values that
// are user state (the select's value, the status) are never reset by relabeling.
root.innerHTML = `
<div class="settings-panel">
  <div class="s-header">
    <span class="s-title" id="s-title"></span>
  </div>
  <div class="s-section">
    <div class="s-section-title" id="s-cc-title"></div>
    <div id="s-hook-status" class="s-status"></div>
  </div>
  <div class="s-section">
    <div class="s-section-title" id="s-usage-title"></div>
    <div class="s-row">
      <label for="s-interval" class="s-label" id="s-interval-label"></label>
      <select id="s-interval" class="s-select">
        <option value="5"></option>
        <option value="10"></option>
        <option value="15"></option>
      </select>
    </div>
  </div>
</div>
`;

const titleEl = root.querySelector<HTMLElement>("#s-title")!;
const ccTitleEl = root.querySelector<HTMLElement>("#s-cc-title")!;
const usageTitleEl = root.querySelector<HTMLElement>("#s-usage-title")!;
const intervalLabelEl = root.querySelector<HTMLElement>("#s-interval-label")!;
const hookStatusEl = root.querySelector<HTMLElement>("#s-hook-status")!;
const intervalEl = root.querySelector<HTMLSelectElement>("#s-interval")!;

// Apply all static labels in the current locale. Idempotent and state-preserving:
// it sets only textContent, never the select's value.
function applyLabels(): void {
  titleEl.textContent = t(locale, "appName");
  ccTitleEl.textContent = t(locale, "claudeCode");
  usageTitleEl.textContent = t(locale, "usageRefresh");
  intervalLabelEl.textContent = t(locale, "interval");
  for (const opt of Array.from(intervalEl.options)) {
    opt.textContent = `${opt.value} ${t(locale, "minUnit")}`;
  }
  applyHookStatus();
}

// Render the hook-status line from the retained install state in the current
// locale. Until a status has been fetched, leave it blank.
function applyHookStatus(): void {
  if (lastInstalled === null) return;
  hookStatusEl.textContent = t(locale, lastInstalled ? "connected" : "notConnected");
  hookStatusEl.style.color = lastInstalled ? "#7ec" : "#aaa";
}

async function refreshHookStatus(): Promise<void> {
  const installed = await invokeOrNull<boolean>("hooks_status");
  lastInstalled = !!installed;
  applyHookStatus();
}

intervalEl.addEventListener("change", async () => {
  const minutes = Number(intervalEl.value);
  const current = await invokeOrNull<Record<string, unknown>>("get_settings");
  const updated = { ...(current ?? {}), usage_refresh_minutes: minutes };
  await invokeOrNull("set_settings", { s: updated });
});

// Re-apply labels live when the user switches language from the menu. The select
// value and the shown status are untouched.
listen<string>("locale", (e) => {
  if (e.payload === "en" || e.payload === "zh") {
    locale = e.payload;
    applyLabels();
  }
}).catch(() => {
  /* not running inside Tauri — no locale events */
});

// Initial load: read locale + interval in one call, paint, then fetch status.
(async () => {
  const s = await invokeOrNull<{ locale?: string; usage_refresh_minutes?: number }>("get_settings");
  if (s?.locale === "en" || s?.locale === "zh") locale = s.locale;
  intervalEl.value = String(s?.usage_refresh_minutes ?? 5);
  applyLabels();
  refreshHookStatus();
})();
