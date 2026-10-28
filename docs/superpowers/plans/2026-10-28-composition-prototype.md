# Native composition prototype implementation plan

**Goal:** Validate real desktop blur, adjustable shadow and an editable transparent WebView without replacing the running capture application.

**Architecture:** An independent Tauri probe first attaches Windows.UI.Composition to its HWND. Native backdrop, Gaussian blur, tint and shadow are separate visuals/effects. A small transparent HTML editor exposes parameters through probe-only IPC. If windowed WebView2 demonstrably prevents correct composition, evaluate visual WebView2 hosting rather than patching the production app.

**Spec:** docs/reviews/2026-10-28/window-composition-assessment.md (user approved 2026-10-28).

**Constraints:** Windows-only experiment, independent app identifier and local output, no reads/writes to production config or drafts, no installer, no production process termination, no Explorer hooks. Native blur sampling must remain live and separate from text. Keep system behavior limitations explicit; do not substitute screenshots or wallpaper.

## Files and tasks

- [x] Create experiments/composition-probe/Cargo.toml, build.rs, tauri.conf.json and capabilities/default.json. Independent package with cached Tauri/windows dependencies and unique application title.
- [x] Create src/effects.rs implementing a Gaussian graphics effect using the Windows SDK IGraphicsEffectD2D1Interop contract. Use PropertyValue for standard deviation, border mode and optimization.
- [x] Create src/composition.rs owning DispatcherQueue, Compositor, desktop target and visual tree on the UI thread. Enable the documented Win32 host backdrop flag; layer independently adjustable tint above blur and a shadow behind the rounded content rectangle. Recompute visual geometry on resize.
- [x] Create src/main.rs for probe-only commands/lifecycle. Expose parameter update/status, log native HRESULT failures; do not report native readiness unless initialization succeeds.
- [x] Create ui/index.html with transparent multiline editor, blur/tint/shadow controls, focus simulation, reset and JSON export. Input is temporary, never saved to note files.
- [x] Compile/check/format the independent package. Run meaningful geometry checks for shadow/content bounds if geometry helpers are needed; inspect real window for rendering, parameter changes and editor interaction.
- [x] Record actual evidence and unresolved compatibility/IME/DPI limitations in README.md and acceptance notes. Leave the working probe open if feasible; do not migrate the production capture window during this task.

## Checkpoints

After native initialization, first verify that the main glass samples live external content. Then verify controls change native blur/shadow independently of tint. Only after those pass evaluate input, focus and clipping. Three attempts without progress require a written root-cause review before another implementation attempt.

The prototype demonstrates feasibility, not the full migration or a completed two-machine regression matrix.
