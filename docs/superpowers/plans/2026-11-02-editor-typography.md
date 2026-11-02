# Editor Typography Implementation Plan

Goal: Persist editor-only font candidates, font size, bold and italic controls with a live settings preview.

Approved design: Appearance tab, font select plus an add-font entry, default current font at 15px, independent bold/italic toggles. Custom candidates persist. Saving updates the editor through app-config-changed. Markdown output is unchanged.

- [x] Add default-compatible Rust typography config and a transactional save command.
- [x] Add shared frontend types/style helper and an isolated settings component with preview.
- [x] Apply styles only to the capture textarea.
- [x] Verify config migration/round-trip, lint, frontend build and Rust tests.

Validation: frontend lint/build and 5 existing tests passed; Rust formatting and 15 tests passed, including legacy config compatibility and typography serialization. Native candidate preview was checked with Computer Use for bold and italic. The user accepted the updated layout with the preview above the font and size controls.
