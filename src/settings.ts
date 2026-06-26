// Settings now live in a separate native window (settings.html / settings-page.ts).
// This module exports only the external door the old HTML menu used; the call is
// now a thin wrapper over the Rust command that opens the settings window.

async function invokeOrNull<T>(cmd: string, args?: unknown): Promise<T | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(cmd, args as Record<string, unknown>);
  } catch {
    return null;
  }
}

export function openSettings(): void {
  void invokeOrNull("open_settings_window");
}
