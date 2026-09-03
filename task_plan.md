# Task Plan

## Goal

Build the first working version of FloatingInputTool as a lightweight AutoHotkey v2 Windows tray utility that captures multi-line fleeting notes into a configurable Obsidian markdown file using prepend-based `fleeting` callout blocks.

## Current Phase

Phase 24 complete - Reliability and visual fixes are ready for installation

## Phases

| Phase | Status | Description |
|-------|--------|-------------|
| 1 | complete | Create design spec, initialize git repo, connect remote, and set up planning files |
| 2 | complete | Scaffold AutoHotkey v2 project structure and shared config helpers |
| 3 | complete | Implement tray lifecycle and global hotkey registration |
| 4 | complete | Implement floating input window behavior |
| 5 | complete | Implement settings window and config persistence |
| 6 | complete | Implement markdown formatting and prepend file writing |
| 7 | in_progress | Manual verification of the end-to-end capture workflow |
| 8 | pending | Prepare packaging and usage notes for compiled distribution |
| 9 | in_progress | Add theme switching, tray theme controls, and visual packaging refinements |
| 10 | in_progress | Preserve the AutoHotkey prototype and move the main UI implementation to Tauri |
| 11 | in_progress | Scaffold Tauri app, restore the approved visual shell, and wire persistent theme switching |
| 12 | in_progress | Split Settings into a dedicated modal window and add shortcut recording |
| 13 | in_progress | Implement single-window side hide with hover-based expand and re-hide |
| 14 | in_progress | Polish packaging behavior, tray-only presence, side-hide stability, and dock animation |
| 15 | in_progress | Add configurable capture, hide, and next-target global hotkeys |
| 16 | in_progress | Add tray-based English/Chinese UI language switching |
| 17 | in_progress | Add visible and metadata ownership marks for shared builds |
| 18 | complete | Review and approve the reliability fixes design |
| 19 | complete | Make Capture idempotent, enforce one instance, and make shortcut updates transactional |
| 20 | complete | Persist drafts atomically and restore them across process restarts |
| 21 | complete | Apply native Windows Acrylic with a non-black fallback |
| 22 | complete | Make config and note writes atomic, enable release diagnostics, and add tests |
| 23 | complete | Build the installer and run the manual regression matrix |
| 24 | complete | Apply Windows native corners and align Discard with Hide |

## Decisions

- First implementation target is Windows only.
- AutoHotkey v2 is the chosen technology for version 1.
- Notes are written directly to markdown files without calling Obsidian APIs.
- The main capture UI must stay out of the taskbar and run from the system tray.
- Destination file path is configured in a separate settings window.
- Output format is a `fleeting` callout block with timestamp and multi-line-safe quoting.
- If the destination file or parent directories do not exist, the tool creates them.
- Side docking is deferred; first version hides back to the background instead.
- Initial config storage will use `%AppData%\FloatingInputTool\config.ini` to avoid write-permission issues for compiled builds.
- Theme mode should be configurable from both the Settings window and the tray menu, with options for dark, white, and follow-system.
- The AutoHotkey version is now a behavior prototype, not the final UI implementation path.
- Final visual fidelity will be implemented in Tauri/WebView so the app can match the approved mockup.
- The future side-hide behavior should be implemented directly in the Tauri window layer.
- The initial Tauri scaffold lives in `tauri-app/` to keep the migration isolated from the preserved AHK prototype.
- Side hide will use a single-window docking model rather than a second functional capture window.
- Main and settings windows should stay out of the Windows taskbar; the tray icon is the app's persistent presence.
- Global shortcuts are action-based: Capture always shows and focuses input, Hide preserves the draft while hiding or docking, and Next Target cycles the active markdown target without touching the draft.
- Drafts are stored separately from configuration and clear only after successful save or explicit discard.
- Windows Acrylic is preferred because it reflects the actual content behind the floating window; a theme-colored fallback must never expose a black transparent surface.
- Config, draft, and Markdown replacement writes must preserve the previous valid file if a write fails.
- UI language is controlled from the tray menu and persisted as `languageMode`; Chinese mode uses a SimHei-first font stack in the web windows.
- Ownership marking should be lightweight and non-invasive: Settings footer, generated config help, and package metadata only. Saved user notes should not contain watermarks.

## Risks

- AutoHotkey window style tuning may require iteration to fully hide the main window from the taskbar while keeping it usable.
- Hotkey registration conflicts may need clear validation and recovery.
- UTF-8 write behavior must be handled carefully so markdown content stays clean.
- Multi-line callout formatting must preserve blank lines without breaking structure.
- The current environment does not have AutoHotkey installed, so runtime validation is temporarily blocked.
- Tauri migration adds setup and packaging complexity compared with the AHK prototype.
- Tray, global hotkey, and filesystem integrations still need to be fully ported from the AHK prototype into Tauri.
- The new multi-window flow must re-enable the main window reliably if the Settings window closes through any path.
- Side-hide monitor detection and dock geometry must fail safely so the main window never becomes stranded off-screen.

## Errors Encountered

| Error | Attempt | Resolution |
|-------|---------|------------|
| Git push rejected because remote already had an initial commit | 1 | Fetched origin/main, rebased local work onto it, then pushed successfully |
| GitHub SSL error during pull/rebase flow | 1 | Used already-fetched origin/main locally and continued with a local rebase |
| Cargo registry query started in offline mode | 1 | Re-ran the version query with approved network access and resolved `tauri-plugin-single-instance` 2.4.4 |
| Cargo registry source search used a Windows wildcard path unsupported by `rg` | 1 | Read the exact Tauri source path returned by the successful match |
| `cargo fmt` could not write source files under the managed filesystem policy | 1 | Applied the exact formatter diff with `apply_patch` and kept `cargo fmt --check` as verification |
| Sandboxed `cargo check` could not create the target lock file | 1 | Re-ran the approved Cargo check with project write access |
| ESLint scanned generated Rust target assets and flagged React declaration order | 1 | Ignored `src-tauri/target`, removed the new synchronous effect state update, and stabilized redock helpers with `useCallback` |
| First combined lint-fix patch used a stale effect dependency context | 1 | Split the changes into smaller patches using the current line structure |
| Combining DWM corner preference with an explicit region repainted a native title strip | 1 | Removed DWM frame ownership and kept precise region clipping only |
| Region-only clipping exposed the hidden non-client caption when the unfocused surface became translucent | 2 | Preserve the resize frame but explicitly remove caption, system-menu, and caption-button styles before clipping |
| Clearing caption styles removed the buttons but a translucent inactive window still exposed cached non-client title text | 3 | Stop region-based patching; choose between standard DWM corners or custom resize handles before continuing |
| Client-area region coordinates clipped the entire WebView and focus-loss refresh caused an Acrylic event loop | 1 | Remove native non-client resizing entirely, restore full-window region coordinates, and keep focus appearance frontend-only |
| Full-window region with the native resize frame removed still produced an invisible WebView surface | 2 | Revert all region-based experiments and return to the verified visible baseline before selecting a different native architecture |
| Cargo source search used a wildcard path that Windows rejected | 1 | Resolved the exact `windows-sys-0.59.0` directory first, then searched that path |
| Windows capture returned only the desktop behind the transparent layered window | 1 | Verified the release process, accessibility tree, Acrylic log, and DWM return path instead of treating the capture limitation as an app failure |

## Next Actions

1. Install `FloatingInputTool_0.2.0_x64-setup.exe` over the existing build.
2. Confirm the Windows standard corner radius and focused/unfocused appearance on the user's desktop compositor.
