# Temporary focus-effect tuner

User clarification: theme means the custom-theme UI, not a source file; a temporary tuning window is also acceptable. Implement a temporary panel in Settings > Appearance with live main-window preview, focus/idle simulation, and copyable parameter JSON. Keep tuning out of saved config; reset preview when the panel is inactive/Settings closes. Native blur radius is not a control.

- [x] Centralize focus defaults and CSS variable mapping in themeEffects.ts; keep theme.css responsible for appearance.
- [x] Add isolated FocusTuner UI and preview events, integrated with Settings Appearance and main window focus classes.
- [x] Verify frontend and backend checks as needed, build release, open tuning UI, test a temporary adjustment/reset without changing saved settings or clipboard.

Preserve existing system corners, Accent Acrylic, static Settings gear and manual draft editing. Existing custom color controls remain available; do not add a second native popup lifecycle.
