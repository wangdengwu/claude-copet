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
    <div class="s-row">
      <button id="s-connect" class="s-btn">Connect</button>
      <button id="s-disconnect" class="s-btn">Disconnect</button>
    </div>
    <div id="s-hook-note" class="s-note"></div>
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
const hookNoteEl = root.querySelector<HTMLElement>("#s-hook-note")!;
const intervalEl = root.querySelector<HTMLSelectElement>("#s-interval")!;
const btnConnect = root.querySelector<HTMLButtonElement>("#s-connect")!;
const btnDisconnect = root.querySelector<HTMLButtonElement>("#s-disconnect")!;

btnConnect.addEventListener("click", async () => {
  const result = await invokeOrNull<null>("install_hooks");
  hookNoteEl.textContent = result !== null ? "Restart Claude Code to apply." : "Could not install hooks (offline mode).";
  await refreshHookStatus(hookStatusEl);
  setTimeout(() => { hookNoteEl.textContent = ""; }, 4000);
});

btnDisconnect.addEventListener("click", async () => {
  const result = await invokeOrNull<null>("uninstall_hooks");
  hookNoteEl.textContent = result !== null ? "Restart Claude Code to apply." : "Could not remove hooks (offline mode).";
  await refreshHookStatus(hookStatusEl);
  setTimeout(() => { hookNoteEl.textContent = ""; }, 4000);
});

intervalEl.addEventListener("change", async () => {
  const minutes = Number(intervalEl.value);
  const current = await invokeOrNull<Record<string, unknown>>("get_settings");
  const updated = { ...(current ?? {}), usage_refresh_minutes: minutes };
  await invokeOrNull("set_settings", { s: updated });
});

refreshHookStatus(hookStatusEl);
refreshUsageInterval(intervalEl);
