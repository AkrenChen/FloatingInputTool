# Footer Settings Button Plan

User request: remove the draft deletion feature and use the supplied static gear SVG for a button opening the Settings popup. Use the old discard slot. Preserve manual editing and unsaved text.

- [x] Replace footer discard button/confirmation/handler/styles and labels with static SVG settings action.
- [x] Expose existing open-or-focus Settings workflow through an async command; flush draft before opening; remove unused public clear_draft command while preserving internal post-save cleanup.
- [x] Run frontend lint/tests/build and backend fmt/check/tests; build candidate, verify settings popup by clicking gear, and package final build.

Carry forward focus-surface work: theme.css owns alpha/transition controls; Accent Acrylic remains a candidate pending actual inactive blur verification. Do not claim the focus issue fixed from API return values alone.

Actual release QA: observed static gear replacing trash, clicked Settings action and confirmed one settings window opened, closed Settings and returned to the editor. Existing empty draft remained unchanged; no note text or settings were edited during QA.
