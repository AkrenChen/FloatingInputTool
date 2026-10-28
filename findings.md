# Findings

## Product Decisions

- A dedicated local tool is a better fit than continuing with Quicker because the main value is in the capture experience, not automation breadth.
- The app should work even when Obsidian is not running.
- The destination markdown file must be configurable because the user wants to reuse the tool across two Windows machines.

## UX Decisions

- The main capture surface should be a floating multi-line window opened by a global hotkey.
- The app should run in the tray and avoid a normal taskbar presence for the main capture window.
- The destination path must not appear in the main input window; it belongs in a secondary settings window.
- First version should hide back to the background instead of implementing more complex side-docked behavior.

## Data Format Decisions

- A single-file note stream is preferred over per-note files or daily-note embedding.
- Every note should use a custom Obsidian callout type: `fleeting`.
- The callout structure is chosen because it is safer for later CSS styling than heading-plus-paragraph layouts.
- Multi-line input must be converted so each line remains inside the callout block.

## Technical Notes

- The project repo is initialized at `D:\Claude\Project\FloatingInputTool`.
- GitHub remote is connected to [Jolynea/FloatingInputTool](https://github.com/Jolynea/FloatingInputTool).
- The first scaffold will use a single AutoHotkey entry script plus small `src` modules for constants, config loading, and tray lifecycle.
- Config storage is set to `%AppData%\FloatingInputTool\config.ini` for reliability in both source-run and compiled app scenarios.
- AutoHotkey is not installed in the current environment, so the scaffold could be reviewed statically but not executed locally yet.
- The next code pass replaced tray placeholders with actual controller wiring for the tray menu, dynamic hotkey registration, input window, settings window, note formatting, and prepend-based file writing.
- At the source level, the first round of core functionality is now present; the remaining uncertainty is runtime behavior under a real AutoHotkey v2 installation.
- Runtime validation now confirms the app starts into the tray correctly and the default hotkey can open the input window.
- The next packaging step is to keep both approved visual themes and expose them through Settings plus tray menu controls, with a follow-system mode tied to the Windows app theme registry value.
- The approved visual design cannot be fully reproduced with native AutoHotkey GUI controls. AutoHotkey can approximate the workflow, but not the HTML-level frosted-glass fidelity.
- Because of that constraint, the project is pivoting to Tauri/WebView for the main UI while keeping the AHK build as a working behavior prototype.
- The future side-hide interaction should be implemented directly in Tauri, where custom window behavior and animation are a much better fit.
- `tauri-app/` is now scaffolded with React + Vite on the frontend and a compiling `src-tauri/` Rust shell.
- The first Tauri frontend pass already recreates the floating-window visual shell with white/dark theme preview modes and an undecorated transparent window config.
- Theme switching is now best owned by Tauri itself: Rust persists the selected mode, updates tray menu checks, and emits frontend events so the React shell stays in sync.
- The first Tauri settings surface can stay lightweight for now; it only needs to expose the decisions that matter for continued visual review, starting with theme mode.
- Rust was installed via the official `rustup-init.exe` installer because `winget` package routes were unreliable in this environment.
- Tauri's `skipTaskbar` window option hides a window from the Windows taskbar while keeping the tray icon available.
- The redock guard depends on refs rather than React state; focus handlers must update those refs synchronously or first-click timing can still collapse the expanded window.
- Side-hide movement was previously a direct `set_size`/`set_position` jump. Native bounds interpolation gives a smoother transition without changing the React visual shell.
- Restore-from-dock should derive its horizontal target from the active dock side and monitor work area instead of trusting stale saved `normal_bounds.x`.
- Tauri global shortcut events expose the registered shortcut object; matching by its internal id is more reliable than comparing saved accelerator strings because display formatting can differ from user-entered config.
- Language switching can stay lightweight for now: a persisted enum plus local UI copy tables is enough and avoids pulling in a full i18n framework.
- Ownership watermarking should avoid modifying captured notes. Settings UI, generated help text, and Windows bundle metadata provide attribution without polluting user content.
- Approved design spec path:
  [2026-04-21-floating-input-tool-design.md](D:/Claude/Project/FloatingInputTool/docs/superpowers/specs/2026-04-21-floating-input-tool-design.md)
- Reliability review confirmed that Capture hiding an already-visible window is a direct cause of intermittent perceived shortcut failure.
- The approved reliability design is at `docs/superpowers/specs/2026-09-03-reliability-fixes-design.md`.
- Acrylic, not Mica, is required to reflect the actual application content behind the floating window.
- The current frontend draft exists only in React memory and the current main close action explicitly clears it.
- Direct `fs::write` calls for config and prepended notes create avoidable partial-overwrite risk.
- Capture now always restores and focuses the editor; Hide remains the only shortcut that hides or docks.
- Shortcut changes now validate conflicts and registration before changing persisted configuration.
- Startup repairs configured shortcuts to the actual registered fallback and exposes warnings to both windows.
- A single-instance plugin now routes subsequent launches to the existing capture window.
- Rust now owns the latest draft snapshot and atomically persists it after a 150 ms debounce.
- Main Close and Hide flush the draft; successful note save clears it; explicit discard uses a dedicated trash action.
- Draft recovery restores text, creation time, and the previously selected target.
- Native Acrylic is now applied on window creation and show, with remote-session and application-failure detection.
- Both frontend windows start in an opaque theme fallback and only reduce opacity after Rust confirms Acrylic is active.
- Config, config help, draft, and prepended Markdown writes now share the same Windows atomic replacement helper.
- Note saves log only byte count and duration, not note content.
- CSS border radius clips only the WebView content; Acrylic still fills the rectangular native HWND, which exposes material in the four corner gaps.
- The Discard button used a smaller hit area, heavier stroke, and rectangular hover fill than Hide, causing visible footer misalignment.
- Windows Acrylic changes its own active/inactive rendering, which made the unfocused window look flatter and more solid even though the frontend had no focus-state styling.
- `SetWindowRgn` conflicts with the hidden non-client surface retained for native edge resizing. Even after caption styles are removed, an unfocused translucent window can reveal cached title text, so region clipping is not a stable final approach while native resizing remains enabled.
- Reapplying Acrylic from every focus-loss event can feed back into further focus changes after native region updates. Focus appearance must remain frontend-controlled without mutating the native material on each focus transition.
- The stable Windows corner solution is `DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND`, which preserves Acrylic, native resizing, and the undecorated WebView. Windows controls the resulting radius, typically about 8-10 px.
- Native rounding is disabled while the main window is side-docked and restored for normal or expanded modes, matching the frontend's square docked edge.
- The Windows capture backend does not composite this transparent layered WebView correctly and can show only the desktop background; the accessibility tree and application logs remain usable for runtime verification.

## 2026-10-28 Composition assessment: initial evidence
Current AccentPolicy exposes state/flags/tint/animation, no blur radius. TranslucentTB custom Blur separately builds XamlBlurBrush with GaussianBlurEffect, unlike Acrylic. Tauri exposes HWND and WebView2 controller; replacing all UI is not a prerequisite for a native composition experiment. Wry currently creates a windowed WebView2 controller. Windows build read from OsBuildNumber is 26200; marketing product label is stale (reports Windows 10). Microsoft documents DWMWA_USE_HOSTBACKDROPBRUSH for non-UWP starting Windows 11 build 22000. Microsoft.UI.Composition documents external-content limits for transparent WebView2; do not assume a WinUI3 WebView2-over-custom-XAML-brush migration will work.

Architecture assessment complete: docs/reviews/2026-10-28/window-composition-assessment.md. Official WebView2 visual hosting supports Windows.UI.Composition ContainerVisual/IDCompositionVisual and requires input forwarding. Wry 0.54.4 source has no CompositionController/RootVisualTarget path. Recommendation: bounded composition prototype on current host first, then native main HWND + visual WebView2 if host blocks integration; keep React/Rust and initially retain Tauri tray/settings. WinUI3 rewrite and Electron do not automatically unlock a desktop blur radius. Shadow may fit transparent margins in one HWND but requires separate content/window geometry. No new compositor implementation or runtime verification performed.

Approved probe outcome: current Tauri HWND/windowed WebView2 coexists with Windows.UI.Composition on build 26200. Live external background, additional Gaussian, independent tint and DropShadow work. HostBackdrop retains baseline system blur at additional standard deviation 0. Actual focus transitions read back 12/60% focused and 4/20% idle. Foreground root HWND avoids Tao child-focus ambiguity. Production migration, IME, transparent-margin hit testing, docking and multi-machine/lifetime tests remain separate gates.

Production migration context: actual theme is white. Main CSS currently fills HWND and clips all outer shadow; native main still uses Accent Acrylic. Dock geometry uses outer HWND coordinates. Migration must introduce a visible-content inset, keep original 560x360 content size, use content bounds for edge detection/restore and hide native visuals while docked. Main focus must use foreground root HWND rather than WebView child-focus signals. Settings retains its independent Acrylic path.

## Dock follow-up 2026-10-28
User reports: outside + foreground another app fails to collapse until clicking capture again; docked strip has extra gray region. Running process is old target/release/app.exe (PID74952), not composition candidate. Old logs show outside cursor checks and delayed subsequent transitions; frontend timer scheduling remains a hypothesis, not established sole cause. Plan: native hover/collapse controller preserving interaction guards; transparent dock background and geometry; shadow hit-through remains separate native-host design gate. Do not operate desktop until code/build checks finish.

## First hover/editor click collapse regression
User repro: after dragging to dock, first hover expansion retains focus; first editor click collapses; next expansion/click succeeds. Code trace identifies native scheduler clears backend drag bool and syncs geometry, but frontend manualDragActiveRef remains armed until a subsequent mouseup. end_manual_window_drag unconditionally re-runs edge docking even though native drag completion already ran. Movement scheduler also syncs for programmatic/non-drag Moved events. Fix consumes one shared backend drag completion, skips duplicates/non-drag movement, and clears frontend drag ref on docked state acknowledgement. Regression test for native-settle then editor-mouseup sequence observed failing before helper implementation.
