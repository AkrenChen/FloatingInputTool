# FloatingInputTool

FloatingInputTool is a lightweight Windows tray utility for capturing fleeting notes into an Obsidian markdown file without requiring Obsidian to be open.

The main application uses Tauri 2, Rust, React 19, and TypeScript. The AutoHotkey v2 prototype remains as a historical behavior reference.

Current capabilities:

- Background tray startup and global Capture, Hide, and Next Target shortcuts.
- Capture always opens the editor; closing or hiding preserves the draft.
- Dragging to a screen edge docks automatically. Hover previews do not request focus.
- Saving a dock preview collapses to its remembered side; normal saving hides the window.
- Multiple Markdown targets, configurable templates, Chinese and English UI.
- White, dark, system, and custom themes.
- Settings grouped into Capture, Shortcuts, Appearance, and Advanced; unfinished edits survive unrelated config changes.

See [experience repair acceptance](docs/reviews/2026-10-27/acceptance.md) for verified checks and remaining desktop validation.

## Why This Exists

The goal is to make quick note capture feel closer to a dedicated system tool than a full note-taking app:

- capture from anywhere
- keep the UI minimal
- avoid interrupting the current workflow
- preserve a markdown structure that is easy to style later in Obsidian

## Planned Note Format

Each capture is written to the top of the configured markdown file in this form:

```md
> [!fleeting] 2026-04-21 11:40
> First line
> Second line
>
> Third line
```

This keeps every entry as a self-contained block and makes later CSS customization much easier.

## Project Structure

```text
FloatingInputTool/
|- docs/
|  \- superpowers/
|     \- specs/
|        \- 2026-04-21-floating-input-tool-design.md
|- README.md
|- task_plan.md
|- findings.md
\- progress.md
```

Implementation files are being added incrementally. The AutoHotkey prototype remains in the repo as a behavior reference while the main UI direction moves toward Tauri.

The current scaffold already includes:

```text
FloatingInputTool/
|- FloatingInputTool.ahk
|- src/
|  |- App.ahk
|  |- Config.ahk
|  |- Constants.ahk
|  |- FileWriter.ahk
|  |- InputWindow.ahk
|  |- NoteFormatter.ahk
|  |- SettingsWindow.ahk
|  \- TrayController.ahk
\- .gitignore
```

## Current Scope

The validated product scope still covers:

- system tray app lifecycle
- hidden floating input window
- configurable global hotkey
- settings window for destination file path and hotkey
- markdown file auto-creation
- prepend write behavior
- safe multi-line callout formatting
- theme selection from both Settings and the tray menu
- side-hide interaction for preserving draft text without writing to markdown

Version 1 intentionally does not include:

- rich text editing
- advanced edge docking animations
- Obsidian plugin integration
- cross-platform support

## Development Workflow

This project uses file-based planning in the repo root:

- [task_plan.md](D:/Claude/Project/FloatingInputTool/task_plan.md) for phases and decisions
- [findings.md](D:/Claude/Project/FloatingInputTool/findings.md) for discoveries and technical notes
- [progress.md](D:/Claude/Project/FloatingInputTool/progress.md) for session-by-session execution logs

The approved design spec lives at [2026-04-21-floating-input-tool-design.md](D:/Claude/Project/FloatingInputTool/docs/superpowers/specs/2026-04-21-floating-input-tool-design.md).

## Roadmap

1. Preserve the AutoHotkey prototype as a functional reference.
2. Scaffold the Tauri app shell.
3. Port tray, hotkey, settings, and markdown writing behavior into Tauri.
4. Rebuild the input window to match the approved visual mockup.
5. Add theme switching with dark, white, and follow-system modes.
6. Implement side-hide behavior in the Tauri window layer.
7. Prepare compiled distribution.

## Future Direction

The first release is intentionally optimized for a small Windows-only utility. The project is now formally moving toward Tauri while preserving the same behavior contract:

- tray-based runtime
- global hotkey capture
- direct markdown writing
- `fleeting` callout output format

## Current Windows surface implementation

The main capture window now uses Windows.UI.Composition for desktop blur, tint, rounded clipping and shadow. Settings retains its independent native backdrop. Shared defaults are in `tauri-app/src/surfaceTheme.json`: focused/inactive interface opacity100%/34%, material coverage100%/85%, shadow opacity24%/10%, shadow blur5, vertical offset2 and corner radius24. Additional Gaussian blur and tint default to0; host backdrop retains system baseline blur.

The temporary tuning panel has been removed from Settings after parameters were finalized. Normal custom-theme controls remain. Native observation controls edge hover/collapse without relying on inactive WebView timers; completion of a drag is consumed once so the first editor click after hover expansion cannot repeat edge docking.

User desktop checks confirmed outside/inactive collapse, removal of docked gray areas and stable first-hover/editor-click behavior on the tested Windows11 machine. Transparent shadow-margin click-through, cross-machine/high-DPI/IME/endurance validation remain open. The independent experiment under `experiments/composition-probe` preserves the earlier feasibility work and its own parameter snapshot.
