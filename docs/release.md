# Cutting a release

Cross-platform installers (macOS · Linux · Windows) are built by GitHub Actions
and attached to a GitHub Release. Local development still uses `pnpm tauri dev`.

## Trigger

The [`Release` workflow](../.github/workflows/release.yml) runs **only** when a
tag matching `v*` is pushed — nothing builds on ordinary pushes or PRs.

## Steps

1. Bump the version in **both** places so they match:
   - `src-tauri/tauri.conf.json` → `version`
   - `src-tauri/Cargo.toml` → `version` (then `cargo build` to refresh `Cargo.lock`)
2. Commit the bump.
3. Tag and push:
   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```
4. The workflow builds on macOS (Apple Silicon + Intel), Linux, and Windows, then
   creates a **draft** GitHub Release with the installers attached. Review the
   draft and publish it when ready.

## Artifacts produced

`tauri-action` uploads the per-OS installers it builds:

- **macOS** — `.dmg` and `.app.tar.gz` (one set per architecture)
- **Linux** — `.AppImage` and `.deb`
- **Windows** — `.msi` and NSIS `.exe`

## Notes

- **Unsigned.** Code signing and notarization are out of scope for now, so:
  - macOS: right-click → Open the first time (Gatekeeper warns on unsigned apps).
  - Windows: SmartScreen may warn; choose "More info → Run anyway".
- No auto-update channel yet (out of scope).
