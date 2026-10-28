# Progress

## 2026-04-21

### Session 1

- Reviewed the existing Obsidian vault and plugin state.
- Evaluated QuickAdd and Advanced URI as possible paths.
- Compared direct Obsidian integration against a dedicated local tool.
- Chose a standalone Windows utility as the better long-term fit.
- Chose AutoHotkey v2 for version 1, with a future migration path to Tauri if needed.
- Wrote and reviewed the design spec.
- Initialized the local git repository.
- Connected the GitHub remote and pushed the initial spec commit.

### Session 2

- Replaced the placeholder README with a project overview and roadmap.
- Created `task_plan.md`, `findings.md`, and `progress.md` to support step-by-step implementation.
- Recorded the current decisions, known risks, and next implementation actions.
- Switched the active plan to Phase 2 so implementation can begin from project scaffolding.

### Session 3

- Added the initial AutoHotkey v2 project scaffold.
- Created the main entry script plus `src` modules for app bootstrapping, constants, config loading, and tray lifecycle.
- Added a `.gitignore` for compiled artifacts and temporary output.
- Chose `%AppData%\FloatingInputTool\config.ini` as the first config storage location.
- Checked the local environment and confirmed AutoHotkey is not installed here, so runtime verification is still pending.

### Session 4

- Replaced the tray placeholders with real application wiring.
- Added a hidden floating input window controller and a separate settings window controller.
- Added note formatting and prepend-based markdown writing modules.
- Implemented dynamic hotkey updating so future settings changes can rebind the global shortcut.
- Advanced the task plan to the manual verification phase, with runtime testing currently blocked by the missing AutoHotkey installation in this environment.

### Session 5

- Installed AutoHotkey v2 locally with `winget`.
- Fixed `#Warn` variable-name collisions in the file writer and settings window code.
- Switched both GUIs to lazy creation so the app no longer flashes a window on startup.
- Confirmed startup now behaves correctly and the global hotkey opens the input window successfully.

### Session 6

- Added theme mode plumbing to config, application state, settings UI, and tray menu.
- Added a first implementation of `theme-dark`, `theme-white`, and `follow-system`.
- Refactored the input window toward the approved custom header layout with separate close, hide, timestamp, and Save regions.

### Session 7

- Compared the approved browser mockup against the AutoHotkey implementation and confirmed the AHK GUI cannot fully reproduce the required visual fidelity.
- Decided to preserve the current AutoHotkey code as a functional prototype and move the main UI implementation to Tauri.
- Recorded that the future side-hide behavior should be implemented in Tauri rather than forced into the native AHK window layer.

### Session 8

- Installed the Rust toolchain with the official rustup installer after `winget` proved unreliable.
- Scaffolded a new `tauri-app/` project with React + Vite and initialized `src-tauri/`.
- Replaced the default starter UI with the approved floating-window visual shell and theme preview controls.
- Configured the Tauri window as undecorated, transparent, non-resizable, and always-on-top.
- Verified the frontend with `npm run build` and the Rust shell with `cargo check`.

### Session 9

- Replaced the temporary system-theme-only frontend logic with a real persisted `themeMode` state shared between React and Rust.
- Added a Tauri tray menu with `Show`, `Settings`, `Theme`, and `Quit`, plus left-click tray behavior to reopen the main window.
- Added a lightweight settings overlay so theme mode can be changed inside the app as well as from the tray.
- Persisted theme mode to the app config directory as JSON and wired frontend updates through Tauri events.
- Re-verified the frontend with `npm run build` and the Rust shell with `cargo check`.

### Session 10

- Extended the Tauri config model to include the destination markdown file path, with a Documents-based default for first launch.
- Added Rust-side note formatting and prepend file writing so the app now produces `fleeting` callout blocks directly from the Tauri UI.
- Wired the main window buttons to their intended semantics: `Close` discards the current draft, `Hide` keeps the draft and hides the window, and `Save` writes to markdown then clears and hides.
- Expanded the in-app Settings overlay with a target file path field and persistence command, while keeping theme switching intact.
- Re-verified the new functional baseline with `npm run build` and `cargo check`.

### Session 11

- Added the Tauri v2 global shortcut plugin and registered the default `Ctrl+Alt+Space` hotkey in the desktop shell.
- Wired the shortcut to toggle the main floating window so the app can be summoned or hidden while staying in the tray.
- Re-verified the frontend with `npm run build` and the Rust shell with `cargo check` after the shortcut plugin integration.

### Session 12

- Extended the persisted Tauri config with a user-editable `hotkey` field.
- Added runtime hotkey activation logic that saves the requested shortcut even when registration fails, while preserving the last successfully registered shortcut in the current session.
- Expanded the Settings overlay with a hotkey input and save action, including red `#f54a45` warning text for conflict or invalid-hotkey cases.
- Added startup fallback behavior so invalid configured shortcuts fall back to the default hotkey instead of leaving the app without a summon shortcut.
- Re-verified the frontend with `npm run build` and the Rust shell with `cargo check`.

### Session 13

- Wrote and committed a short follow-up spec for reworking Settings into a dedicated native modal window.
- Removed the in-window Settings overlay from the main capture surface and replaced it with a dedicated `settings` Tauri window.
- Added Rust-side single-instance Settings window creation, parent-window ownership, and main-window disabling so Settings behaves like a true modal child flow.
- Added a second React view for the Settings window while keeping the same bundled frontend build.
- Replaced manual hotkey typing with a recorder-style input that captures pressed key combinations into accelerator strings.
- Expanded the capability config so the new Settings window can start dragging and close itself.
- Re-verified the updated frontend with `npm run build` and the Rust shell with `cargo check`.

### Session 14

- Extended the persisted Tauri config with side-hide fields for dock thresholds, handle width, hover delays, hotzone width, and the `debugShowHotzone` flag.
- Added Rust-side main-window runtime state so the app can dock to the left or right edge, restore from dock, and re-dock without losing the current draft.
- Changed `Hide` in the main capture UI to call the new dock-or-hide command instead of always doing a normal hide.
- Added docked-mode frontend rendering for the visible side handle plus hover-driven expand and delayed re-hide behavior.
- Added a debug hotzone overlay that appears only when `debugShowHotzone = 1`.
- Re-verified the updated frontend with `npm run build` and the Rust shell with `cargo check`.

### Session 15

- Added fixed-width markdown target chips to the main input window and aligned them with the editor left edge.
- Added a target-list cancel action in Settings that discards unsaved target edits and only appears while the target list is expanded.
- Tuned the docked side handle into a rounder folder-tab shape without changing side-hide behavior.
- Re-verified the frontend with `npm run build`.
- Added drag-handle sorting for Markdown Targets in Settings and compressed the target list into a table-like layout.
- Re-verified the frontend with `npm run build`.
- Added an explicit Markdown Targets edit mode so sorting and target edits stay local until `Save Targets`, while `Cancel` discards them.
- Re-verified the frontend with `npm run build`.

### Session 16

- Changed the default side-hide close delay from `320ms` to `100ms`.
- Added a configurable note template with `{{timestamp}}`, `{{text}}`, and `{{text.callout}}` placeholders.
- Added a Settings section for editing, saving, canceling, and restoring the default note template.
- Re-verified the frontend with `npm run build` and the Rust shell with `cargo check`.

### Session 17

- Fixed the main capture layout so `footer/save` no longer occupies the flexible editor row when there is only one markdown target.
- Replaced the app icon source and regenerated the Tauri icon set.
- Added `skipTaskbar` for the main window and `skip_taskbar(true)` for the Settings window so the app stays tray-first.
- Fixed first-click collapse after side-handle expansion by synchronously updating the focus refs used by the redock guard.
- Added a short eased bounds animation for dock, restore, and redock transitions.
- Tuned side-hide animation to a `120ms` window slide plus a separate `60ms` dock-handle appearance.
- Clamped restore-from-dock bounds to the screen interior so long-lived left/right docking cannot expand outward off-screen.
- Re-verified with `npm run build`, `cargo check`, and `npm run tauri:build`.

### Session 18

- Added configurable global hotkeys for Capture, Hide, and Next Target.
- Changed Rust shortcut routing to match registered shortcut ids instead of display strings, so recorded accelerator text does not have to match the plugin's internal formatting.
- Added Settings recorder rows for all three shortcut actions.
- Added a main-window focus event so hotkey summon focuses the textarea immediately.
- Added next-target cycling that updates the active markdown target without changing the current draft.
- Re-verified with `npm run build` and `cargo check`.

### Session 19

- Added persisted `languageMode` config with `english` and `chinese` values.
- Added a tray `Language / 语言` submenu with English and 中文 check items.
- Wired tray language changes through `app-config-changed` so main and Settings windows update without restart.
- Added English/Chinese UI copy for the main capture window and Settings window.
- Added a SimHei-first font stack for Chinese mode.
- Re-verified with `npm run build` and `cargo check`.

### Session 20

- Read current Git identity: `chenyb <chenyb1348@gmail.com>`.
- Added Settings footer ownership text for `chenyb` and Xiaohongshu `@落雨返屋企 (210601284kk)`.
- Added ownership metadata to Tauri bundle config and Cargo package metadata.
- Added copyright and creator attribution to generated `config.help.md`.
- Re-verified with `npm run build`, `cargo check`, JSON config parsing, and `npm run tauri:build`.

## 2026-09-03

### Session 21

- Reviewed the installed Tauri implementation and traced the reported shortcut, draft-loss, and black-backdrop failures.
- Wrote and committed the approved reliability fixes design as `e64575c`.
- Started Phase 19 for idempotent Capture, single-instance enforcement, transactional shortcut updates, and release diagnostics.
- Added single-instance enforcement, idempotent Capture behavior, native Windows foreground activation with retry, transactional shortcut replacement, startup fallback repair, and release log rotation.
- Exposed actual active shortcuts and startup warnings to the main and Settings windows.
- Verified Phase 19 with `cargo check` and `npm run build`.
- Started Phase 20 for atomic draft persistence and recovery.
- Added shared atomic storage and a separate `draft.json` model with Rust-side debouncing.
- Added startup draft restoration, close-as-hide behavior, explicit discard, target restoration, and exit flushing.
- Verified Phase 20 with `cargo check`, `npm run build`, and two passing Rust unit tests.
- Started Phase 21 for native Acrylic and the stable non-black fallback.
- Added native Windows Acrylic for main and Settings windows, including reapplication after show.
- Added an opaque theme fallback for remote sessions and any Acrylic initialization failure.
- Verified Phase 21 with `cargo check`, `npm run build`, and formatter checks.
- Started Phase 22 for atomic config/note writes and additional tests.
- Replaced direct config, help, draft, and Markdown writes with flushed temporary files and Windows atomic replacement.
- Added Markdown formatting/prepend tests; all four Rust unit tests pass.
- Started Phase 23 for linting, versioning, installer build, and runtime regression checks.
- Runtime regression confirmed Acrylic, repeated Capture, Hide, single-instance activation, and draft recovery across close and process restart.
- Started Phase 24 after visual QA found native Acrylic leaking outside the CSS corner radius and inconsistent Discard/Hide icon treatment.
- Added explicit focus-state material ownership: focused surfaces become more opaque and unfocused surfaces expose more Acrylic, with a delayed native backdrop refresh after focus loss.
- Paused the native corner implementation after three visual attempts: DWM plus region, region only, and region plus frameless-style enforcement all exposed non-client rendering artifacts in the translucent inactive state.
- Reframed the implementation after the three-attempt stop: clip only the client area and move resize input to internal Tauri edge handles, preserving the approved 22 px radius without rendering native frame pixels.
- User selected the lower-risk native DWM corner option after the custom 22 px region approaches proved unstable.
- Added `DWMWA_WINDOW_CORNER_PREFERENCE` handling for the main and Settings windows.
- Kept native corners enabled in normal/expanded modes and disabled them while side-docked.
- Confirmed the release process starts from `target/release/app.exe`, exposes the updated Discard/Hide UI tree, enables Acrylic, and reports no DWM corner failure.
- Rebuilt `FloatingInputTool_0.2.0_x64-setup.exe` and passed Rust tests, Rust formatting, Rust compile checks, frontend lint/build, and `git diff --check`.

## 2026-10-28 Architecture assessment
User requested evaluation of replacing the underlying window implementation; no application changes authorized in this assessment. Read current code/designs and TranslucentTB source. Official web tool unavailable (HTTP404), direct HTTPS documentation retrieval works. Comparing native Windows.UI.Composition integration, native host/UI migration and Electron.

Assessment delivered: compared four architectures, checked current Wry creation path, Windows host-backdrop support, WebView2 composition hosting/input forwarding, WinUI external-content limits, and DropShadow parameters. Saved Chinese assessment with acceptance matrix and source links. Application code and running app unchanged. User wording check: this evaluates replacing the bottom layer, migration scope and actual feasibility; it does not claim a prototype or migration was implemented. Source fetch errors: two optional Microsoft URLs returned404; supported Windows.UI APIs were verified through successful official pages instead.

## 2026-10-28 Approved native-composition probe delivered

- Added independent experiments/composition-probe with live external reference window, native Gaussian/tint/shadow and temporary transparent editor/parameter controls.
- Verified actual native focus/idle parameter transitions after switching to foreground root HWND detection; idle retains live blur.
- Passed release build, clippy with warnings denied, formatting and 2 geometry tests. Supplementary latest-round editor automation did not establish textarea focus, so multiline/IME checks remain open.
- Recorded observed behavior and migration gates in docs/reviews/2026-10-28/acceptance.md and experimental README. Left default-parameter probe open. Production configuration hash unchanged; no production migration, installation or commit.

### Probe whole-window opacity follow-up
- User chose fading the entire inactive window including text/buttons. Added focusedOpacity / idleOpacity (default 100% / 60%) using layered-window alpha and native readback; shadow concentration now displays as percent.
- Preserved latest observed tuning (blur/tint 0, shadow 10/0%/2, corner 24) in startup defaults and tuned-parameters.json. Focused 100% and idle 60% actual screenshots/native readbacks verified. User was operating the window; did not continue competing focus activation.
- Release build, clippy, formatter and JS syntax checks passed. Production app unchanged by this follow-up.

### Shadow concentration and idle-blur regression corrected
- User exposed visual failures despite correct native values. DWM nonclient-disable hypothesis failed and was reverted. Whole LayerVisual opacity also failed intermediate-value visual QA; recorded the failed path rather than treating endpoint toggles as sufficient.
- Moved rounded shadow source into an offscreen CompositionVisualSurface; a separate SpriteVisual draws only the masked shadow. Actual 0% / 49% / 100% comparison now shows continuous four-edge concentration changes.
- Removed HWND layered alpha. Root visual opacity mixed clear desktop back into inactive glass, so user explicitly approved independent interface fading and glass coverage. New focusedMaterialOpacity / idleMaterialOpacity default to 100%; existing opacity fields now mean interface opacity.
- Final actual focus-loss check on live external grid: interface 58%, glass coverage 100%, additional blur 64; grid remains visibly blurred while text/buttons fade. Preserved shadow 10/49%/2 and corner 24. Release, clippy, fmt, JS syntax checks pass. Probe open; production not migrated.

### Final user-selected tuning
- User reported tuning complete. Read full exported JSON from the running probe: focused interface/material 1/1, idle interface/material 0.34/0.85, both additional blur and tint 0, shadow blur/opacity/y 5/0.24/2, corner 24, preview follow.
- Saved tuned-parameters.json and synchronized Rust/HTML startup defaults plus README. UI defaults match saved JSON; syntax and formatting checks pass. Production material/config remain unchanged; production composition migration is a separate stage.

## Main composition migration — implementation checks
- Added shared surfaceTheme.json with final probe tuning, UI-thread native scene/effect modules, foreground-root focus detection, resize/DPI refresh, dock visibility and shutdown cleanup.
- Main retains 560x360 visible content inside 72px shadow margins; visible bounds now drive edge detection/restore. Internal resize handles and custom-theme temporary controls cover all native parameters.
- Frontend build/lint and 5 tests pass; Rust check, 11 tests and fmt pass. Candidate release is compiling with a separate identifier/config/hotkeys; production config SHA256 remains unchanged.
- Corrected one formatter invocation run from repository root by running in src-tauri; no source or test failure.
- Desktop interaction has not begun; transparent-margin input, actual focus visuals, docking and resize still require Computer Use checks. IME, multi-machine and endurance validation remain open.

### Dock regression follow-up
- Confirmed reported runtime is old app.exe, not the candidate. Read old cursor/dock logs; inactive frontend scheduling is suspected, not proven as sole cause.
- Added native 50ms observer for hover open/outside collapse, with compose/save/drag/settings guards and leave-before-reopen behavior. Windows frontend timers are disabled; non-Windows path retained.
- Native exposed width now follows visible handle width, removing the extra hidden 14px hotzone at defaults. Docked CSS explicitly transparent; main native composition root already hides while docked.
- Added native dwell restart regression test (observed expected initial failure, then pass); 12 Rust tests pass. Frontend build/lint and 5 tests pass. Candidate rebuild underway.
- Shadow-margin click-through is NOT fixed. Single HWND margin cannot be treated as solved through CSS. Separate noninteractive shadow host is an unresolved implementation/design gate; no claim of completion.

### User desktop acceptance and scope correction
- User manually docked candidate. Read-only Computer Use observed right strip width22 and native hover restore log entries.
- User explicitly confirmed: outside + focus other window now automatically collapses; no gray block remains.
- User now says only remaining request is to make strip attractive. Cancelled proposed compact-HWND geometry change before implementation; removed only its pending failing test. Existing accepted geometry/interaction remain.
- Current new scope: visual strip styling, style preference pending. Shadow companion migration not part of this newly narrowed request.
- Candidate runtime initially used devUrl because direct cargo rustc lacked tauri/custom-protocol; rebuilt with feature and confirmed actual bundled main UI is visible. No production config/process replacement.

### Dock strip visual direction approved
User selected preserving existing trapezoid and vertical text, changing only color/border/corners. Replaced clipped rectangular CSS border with an SVG trapezoid contour and rounded shoulders, theme-aware muted gradient/outline/ink, lighter typography and mild hover highlight. Hit geometry and native dock controller unchanged. Frontend build/lint and diff/fmt checks pass. One npm invocation from root failed (no package.json); reran successfully from tauri-app. Style candidate release compiling with bundled custom protocol.

### Strip visual acceptance
User accepted preserved trapezoid/vertical-text visual redesign, requesting only bolder text. Set dock label font-weight to700 (was600 in new styling); all other accepted styling/behavior unchanged. Frontend build passes. Rebuilding bundled candidate for final bold label. Formal app/config remain unchanged.

Final bold-label candidate release built successfully with tauri/custom-protocol; copied into stable FloatingInputTool-Candidate.exe after waiting for the isolated old candidate to exit. Original app.exe process and config remain unchanged. Latest requested visual scope is complete; no installation/commit.

First-expansion click regression: backend drag completion is now consumed once by native settle or frontend mouseup, and non-drag movement notifications skip edge evaluation. Frontend drag ref clears on dock acknowledgement. Rust13 tests and frontend5 tests/build/lint pass; bundled candidate release passed and stable candidate executable refreshed. No production app/config replacement. User wording check: this targets first hover after drag plus editor click; actual desktop sequence remains to verify, rather than inferring from focus colors or unit results.

Candidate fix release built and opened; Computer Use confirmed bundled main UI starts. One automated drag was interrupted by physical user input; stopped input and re-observed read-only, so no actual first-hover/editor-click pass is claimed. User may retest current candidate. Production configuration hash unchanged.

### Inactive shadow opacity
User requested inactive shadow concentration20%. Added idleShadowOpacity0.2 to shared surfaceTheme.json and native validation/application; existing shadowOpacity remains focused24%. Custom theme tuner now exposes focused/inactive shadow sliders separately. Default-value test includes0.2. Frontend build/lint pass; candidate release rebuild underway.

Inactive shadow final adjustment: user changed requested value from20% to10% before candidate deployment. Shared idleShadowOpacity and default assertion now0.1; focused remains0.24. Frontend build and targeted theme test pass. Rebuilding final10% bundled candidate;20% binary was not deployed.

Final10% inactive-shadow bundled candidate release succeeded and stable candidate executable refreshed. Focused shadow remains24%; independent focus/idle controls are in theme JSON and custom theme tuning page. No formal installation/commit.

## Final settings cleanup and Git upload authorization
User authorized Git upload, then requested removing temporary Settings tuning first. Removed panel component/styles and its preview event subscription/emission; shared final theme defaults remain. Frontend lint/build/5 tests pass; Rust formatting/13 tests verified. User authorization supersedes earlier historical do-not-commit stage notes. Upload scope includes reviewed implementation, tests, design/review records and composition probe source; generated binaries, temporary candidate configs and local runtime data remain excluded by existing ignores. Current branch codex/experience-fixes; no merge into main requested.

Final settings cleanup verification: frontend lint/build/5 tests and Rust fmt/13 tests pass; staged whitespace check clean. Bundled custom-protocol candidate release passed and stable executable refreshed without temporary tuning UI. Shared idle shadow10%/focused24% defaults retained. User-authorized upload target is origin/codex/experience-fixes; generated files remain ignored. Git commit/remote ref provide upload audit record.
