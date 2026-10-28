# Floating Input Tool Side Hide Design

Date: 2026-04-22

Behavior updated: 2026-10-27. Dragging to an edge automatically docks the window. Saving successfully from the expanded docked state collapses it back to the same side; saving from normal mode hides it completely.

## Goal

Add a side-hide behavior to the Tauri main window without introducing a second functional capture window.

The behavior should let the existing capture window partially dock into the left or right edge of the current screen, keep the current draft in place, and automatically expand or re-hide based on hover timing.

## Chosen Approach

Use a single-window docking model.

The existing main window remains the only capture window. When side-hide is triggered near the left or right screen edge, the window is repositioned so most of it moves off-screen while a narrow visible handle remains. Hover behavior then controls expansion and re-hide.

This approach is preferred over creating a second handle window because it keeps draft state, hotkey behavior, and tray behavior simpler and avoids cross-window synchronization.

## Trigger Rules

### Entering Side Hide

The `Hide` action only becomes a side-hide action when the main window is already close enough to a left or right screen edge.

Dragging the main window to a qualifying left or right edge also automatically enters side-hide mode after the drag ends. No separate `Hide` action is required.

The app checks the current screen that contains the window and compares the window bounds against that screen's left and right edges.

Side hide should activate only when one of these is true for the current screen:

- the window is overlapping the left or right edge
- the window is partially outside the left or right edge
- the window is within the configured snap threshold of the left or right edge

If neither side is close enough, `Hide` keeps its current behavior and simply hides the window to the background.

### Choosing a Side

If both edges are technically within range, the app chooses the side with the smaller distance from the current window position.

## Docked State

When docked:

- the same main window remains alive
- the current draft stays in the same textarea
- the window moves mostly off-screen
- a narrow visible handle remains on the chosen side

The visible handle is the part of the same window that remains exposed.

## Handle Appearance

The exposed portion should communicate that the hidden window still exists.

Requirements:

- it appears as a rotated `-90deg` rounded trapezoid-like side marker
- its visible length follows the current window height
- it stays attached to the side that the window is docked to

The exact styling remains theme-driven and may later be tuned through `debug1.json`.

## Hover Behavior

### Expand

When docked, the app monitors a configurable hotzone near the visible handle.

If the cursor enters the hotzone and remains there for `hoverOpenDelayMs`, the full window expands back onscreen.

### Re-hide

Once expanded from a docked state, if the cursor leaves the window and remains outside it for `hoverCloseDelayMs`, the window re-hides to the same docked side.

### Draft Preservation

No side-hide transition writes anything to markdown.

The current draft remains untouched across:

- dock
- expand
- re-hide
- repeated hover cycles

## Configuration

Side-hide uses JSON configuration only. No in-app toggle is added for debug behavior.

The initial configuration should live alongside the app's local config and expose these fields:

- `sideHideEnabled`
- `edgeSnapThresholdPx`
- `visibleHandleWidthPx`
- `hoverOpenDelayMs`
- `hoverCloseDelayMs`
- `hotzoneWidthPx`
- `debugShowHotzone`

`debugShowHotzone` is numeric and uses `0` or `1`.

## Default Values

Defaults should feel slightly aggressive rather than conservative.

Recommended defaults:

- `sideHideEnabled = 1`
- `edgeSnapThresholdPx = 36`
- `visibleHandleWidthPx = 22`
- `hoverOpenDelayMs = 180`
- `hoverCloseDelayMs = 320`
- `hotzoneWidthPx = 36`
- `debugShowHotzone = 0`

## Debug Behavior

When `debugShowHotzone = 1`:

- the app should visually show the active hover hotzone
- the debug visualization exists only for tuning and should not affect normal behavior

Because the hotzone may extend beyond the visible part of the main window, the debug visualization may use a lightweight overlay surface if needed.

## Window State Model

The main window should track three behavioral states:

1. `normal`
   Fully visible and not docked.
2. `docked-left` or `docked-right`
   Mostly off-screen with a visible handle.
3. `expanded-from-dock`
   Temporarily restored from a docked state and waiting for hover-close evaluation.

The implementation should preserve enough previous geometry to return the window to the correct expanded position from the same side.

## Interaction Rules

- `Save` in normal mode writes to markdown and hides the window completely.
- `Save` in `expanded-from-dock` mode writes to markdown, clears the saved draft, and collapses the window back to its original docked side, leaving the handle visible. It must not remain expanded or wait for the cursor to leave. A failed save keeps the editor and draft available.
- `Close` still clears the draft and hides the window normally.
- Both dragging to a qualifying edge and `Hide` near an edge can enter side-hide mode.
- Global hotkey behavior should continue to show the main window normally; side-hide should not break summon behavior.
- Tray `Show` should also continue to restore the main window normally.

## Error Handling

- If the app cannot determine a valid monitor or window position, `Hide` should fall back to normal hide behavior.
- If hover monitoring fails, the window should remain visible rather than trap itself off-screen.
- If any side-hide configuration field is missing or invalid, the app should fall back to the default values for that field.

## Testing Scope

Manual verification should cover:

- dragging to either edge and automatically docking after releasing the mouse, without pressing `Hide`
- expanding from either docked side, saving successfully, and immediately collapsing back to the same side with the handle visible
- saving from normal mode and hiding completely
- hiding near the left edge and getting left docking
- hiding near the right edge and getting right docking
- hiding away from edges and getting normal hide instead of docking
- hovering the dock hotzone and seeing expansion after the configured delay
- leaving the expanded window and seeing re-hide after the configured delay
- preserving draft content across repeated dock and expand cycles
- verifying the visible handle height tracks the current window height
- enabling `debugShowHotzone` and confirming the hotzone becomes visible
