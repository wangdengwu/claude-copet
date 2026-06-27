// Standalone settings window — no pet, no card, just the settings controls.
// The same invokeOrNull pattern as main.ts so a plain vite dev without Tauri
// doesn't crash.

async function invokeOrNull<T>(cmd: string, args?: unknown): Promise<T | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(cmd, args as Record<string, unknown>);
  } catch {
    return null;
  }
}

const root = document.getElementById("settings-root")!;

async function refreshHookStatus(el: HTMLElement): Promise<void> {
  const installed = await invokeOrNull<boolean>("hooks_status");
  el.textContent = installed ? "● Connected" : "○ Not connected";
  el.style.color = installed ? "#7ec" : "#aaa";
}

async function refreshUsageInterval(el: HTMLSelectElement): Promise<void> {
  const s = await invokeOrNull<{ usage_refresh_minutes: number }>("get_settings");
  el.value = String(s?.usage_refresh_minutes ?? 5);
}

root.innerHTML = `
<div class="settings-panel">
  <div class="s-header">
    <span class="s-title">claude-copet</span>
  </div>
  <div class="s-section">
    <div class="s-section-title">Claude Code</div>
    <div id="s-hook-status" class="s-status"></div>
  </div>
  <div class="s-section">
    <div class="s-section-title">Usage Refresh</div>
    <div class="s-row">
      <label for="s-interval" class="s-label">Interval</label>
      <select id="s-interval" class="s-select">
        <option value="5">5 min</option>
        <option value="10">10 min</option>
        <option value="15">15 min</option>
      </select>
    </div>
  </div>
</div>
`;

const hookStatusEl = root.querySelector<HTMLElement>("#s-hook-status")!;
const intervalEl = root.querySelector<HTMLSelectElement>("#s-interval")!;

intervalEl.addEventListener("change", async () => {
  const minutes = Number(intervalEl.value);
  const current = await invokeOrNull<Record<string, unknown>>("get_settings");
  const updated = { ...(current ?? {}), usage_refresh_minutes: minutes };
  await invokeOrNull("set_settings", { s: updated });
});

refreshHookStatus(hookStatusEl);
refreshUsageInterval(intervalEl);
