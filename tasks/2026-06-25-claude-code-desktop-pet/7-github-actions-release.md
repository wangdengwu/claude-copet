---
id: 7
slug: github-actions-release
prd: docs/prds/2026-06-25-claude-code-desktop-pet.md
state: ready-for-agent
category: enhancement
blocked_by: [1]
---

## What to build
Hands-off cross-platform distribution. A GitHub Actions workflow builds the Tauri app
for macOS, Windows, and Linux and attaches the installers to a GitHub Release when a
release is cut (e.g. on a version tag). Local development stays on `tauri dev`; this
slice is purely the release pipeline.

Demo: pushing a version tag produces a GitHub Release with macOS, Windows, and Linux
artifacts built by CI.

## Key interfaces
- Release workflow — triggered on tag/release; uses `tauri-apps/tauri-action` to build and upload artifacts for the three OS targets via a build matrix.
- Trigger contract — a documented convention (which tag pattern / release event) that starts a build.
- Artifact outputs — the per-OS installer formats `tauri-action` produces, attached to the Release.

## Acceptance criteria
- [ ] A workflow exists that builds the app on macOS, Windows, and Linux runners.
- [ ] Cutting a release (per the documented trigger) attaches installers for all three OSes to the GitHub Release.
- [ ] The trigger and how to cut a release are documented in the repo.
- [ ] The workflow does not run on every push — only on the release trigger (no wasted CI).

## Out of scope
- Code signing / notarization (macOS) and auto-update — later, not in this PRD.
- Publishing to package managers or app stores.
- Any change to app behavior — pipeline only.
