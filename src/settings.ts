// Settings panel for toggling LLM, selecting provider, and storing the API key.
// Calls Tauri commands get_settings / set_settings; guards invoke so plain
// `vite dev` (no Tauri runtime) does not crash.

interface Settings {
  llm_enabled: boolean;
  provider: string;
  model: string;
  api_key: string;
}

async function invokeOrNull<T>(cmd: string, args?: unknown): Promise<T | null> {
  try {
    // Dynamic import so the bundle doesn't break in a plain browser context.
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(cmd, args as Record<string, unknown>);
  } catch {
    return null;
  }
}

// Exposed so external callers (e.g. the context menu) can open the panel.
let _openSettingsFn: (() => void) | null = null;
export function openSettings(): void {
  _openSettingsFn?.();
}

/** Build and attach the settings panel to the given container element. */
export function mountSettingsPanel(container: HTMLElement): void {
  const panel = document.createElement("div");
  panel.id = "settings-panel";
  panel.style.cssText = [
    "position:fixed;bottom:8px;right:8px;background:rgba(0,0,0,0.75);",
    "color:#eee;font-family:monospace;font-size:11px;padding:8px;",
    "border-radius:4px;width:220px;display:none;z-index:100;",
  ].join("");

  panel.innerHTML = `
    <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:6px">
      <span style="font-weight:bold">Settings</span>
      <button id="s-close" title="Close (Esc)"
        style="background:none;border:none;color:#eee;font-family:monospace;font-size:13px;line-height:1;cursor:pointer;padding:0 2px">✕</button>
    </div>
    <label style="display:flex;align-items:center;gap:4px;margin-bottom:4px">
      <input type="checkbox" id="s-llm-enabled"> Enable LLM voice
    </label>
    <label style="display:block;margin-bottom:4px">
      Provider
      <select id="s-provider"
        style="width:100%;box-sizing:border-box;margin-top:2px;background:#222;color:#eee;border:1px solid #555;padding:2px">
        <option value="claude-cli">claude-cli (uses local Claude Code login — no key)</option>
        <option value="anthropic">anthropic (API key)</option>
      </select>
    </label>
    <label style="display:block;margin-bottom:4px">
      API Key
      <input id="s-api-key" type="password" placeholder="(stored locally)"
        style="width:100%;box-sizing:border-box;margin-top:2px;background:#222;color:#eee;border:1px solid #555;padding:2px">
    </label>
    <button id="s-save"
      style="width:100%;background:#444;color:#eee;border:1px solid #666;padding:3px;cursor:pointer">
      Save
    </button>
    <div id="s-status" style="margin-top:4px;font-size:10px;color:#aaa"></div>
  `;

  container.appendChild(panel);

  const chkEnabled = panel.querySelector<HTMLInputElement>("#s-llm-enabled")!;
  const inpProvider = panel.querySelector<HTMLSelectElement>("#s-provider")!;
  const inpApiKey = panel.querySelector<HTMLInputElement>("#s-api-key")!;
  const btnSave = panel.querySelector<HTMLButtonElement>("#s-save")!;
  const statusEl = panel.querySelector<HTMLElement>("#s-status")!;

  // Load current settings when the panel first becomes visible.
  async function loadSettings(): Promise<void> {
    const s = await invokeOrNull<Settings>("get_settings");
    if (!s) return;
    chkEnabled.checked = s.llm_enabled;
    inpProvider.value = s.provider;
    // Never pre-fill the key field — let the user re-enter.
    inpApiKey.placeholder = s.api_key ? "(key stored)" : "(not set)";
  }

  btnSave.addEventListener("click", async () => {
    const current = await invokeOrNull<Settings>("get_settings");
    const model = current?.model ?? "claude-haiku-4-5";

    const s: Settings = {
      llm_enabled: chkEnabled.checked,
      provider: inpProvider.value || "claude-cli",
      model,
      // Only update the key if the user typed something; preserve existing otherwise.
      api_key: inpApiKey.value.trim()
        ? inpApiKey.value.trim()
        : (current?.api_key ?? ""),
    };

    const ok = await invokeOrNull<null>("set_settings", { s });
    statusEl.textContent = ok !== null ? "Saved." : "Saved (offline mode).";
    setTimeout(() => { statusEl.textContent = ""; }, 2000);
  });

  // Toggle visibility via a small gear button.
  const toggle = document.createElement("button");
  toggle.textContent = "gear";
  toggle.style.cssText = [
    "position:fixed;bottom:8px;right:8px;background:rgba(0,0,0,0.5);",
    "color:#eee;font-family:monospace;font-size:10px;border:none;",
    "padding:3px 6px;border-radius:3px;cursor:pointer;z-index:101;",
  ].join("");
  container.appendChild(toggle);

  function onOutsideClick(e: MouseEvent): void {
    if (!panel.contains(e.target as Node) && e.target !== toggle) {
      closePanel();
    }
  }

  function openPanel(): void {
    if (panel.style.display === "none") {
      panel.style.display = "block";
      toggle.style.display = "none";
      loadSettings();
      // Attach the outside-click closer on the NEXT tick, so the very click that
      // opened the panel (e.g. the right-click menu's Settings item) doesn't
      // immediately bubble into it and close the panel again.
      setTimeout(() => document.addEventListener("click", onOutsideClick), 0);
    }
  }

  function closePanel(): void {
    panel.style.display = "none";
    toggle.style.display = "block";
    document.removeEventListener("click", onOutsideClick);
  }

  // Wire the module-level export so the context menu can call openSettings().
  _openSettingsFn = openPanel;

  toggle.addEventListener("click", openPanel);
  panel.querySelector<HTMLButtonElement>("#s-close")!.addEventListener("click", closePanel);

  // Close on Escape.
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && panel.style.display !== "none") closePanel();
  });
}
