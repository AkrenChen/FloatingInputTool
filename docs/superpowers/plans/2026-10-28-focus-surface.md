# Focus Surface Implementation Plan

> Execute the approved design in this session with executing-plans. User has authorized implementation; preserve existing uncommitted work.

**Goal:** Focused readable surface and unobtrusive inactive surface, with controls centralized in theme.css.
**Architecture:** Existing Tauri focus state drives CSS classes; CSS theme tokens determine surface tint alpha and background-color transition. Keep native Acrylic; use Accent compositing for the main window to avoid the inactive system-backdrop fallback, following the user-requested TranslucentTB investigation.
**Tech Stack:** React, CSS, Tauri.
**Spec:** docs/superpowers/specs/2026-10-28-focus-surface-design.md

- [x] Add theme.css with focused/idle surface and editor alpha plus transition duration; support reduced motion.
- [x] Wire main surface colors and custom configured focus alpha; preserve fallback, dock handle and Settings behavior.
- [x] Run lint, existing tests, production build and diff check; open release and compare focused/unfocused states, record limits.

## Execution progress
- Theme tokens and CSS focus transitions implemented; custom configured opacity wired as focused value.
- lint, 5 frontend tests, tsc/Vite production build and diff check passed.
- Release packaging and actual focused/unfocused comparison in progress.


Final verification: 5 frontend and 9 Rust tests, lint/fmt/check, production build and NSIS packaging passed. Opened actual release; the gear opens Settings. While Settings owns focus, the inactive main window visibly retains background blur at the lighter tint; closing Settings restores the denser focused surface. Other DPI/multimonitor/long-cycle scenarios are not part of this spot check.
