# FloatingInputTool Reliability Fixes Design

## Goal

Fix the installed application's intermittent capture shortcut behavior, preserve unsaved drafts across hiding and process restarts, and eliminate black transparent-window backgrounds while retaining the current visual character where Windows supports it.

## Scope

This change covers four reliability areas in priority order:

1. Global shortcut registration and capture-window activation.
2. Draft persistence and explicit discard behavior.
3. Native Windows backdrop effects with a stable fallback.
4. Atomic configuration and Markdown writes, diagnostics, and focused tests.

Unrelated UI redesign and new capture features are out of scope.

## Shortcut Reliability

The Capture shortcut will be idempotent: it always restores, shows, foregrounds, and focuses the main editor. It will never hide the window. The existing Hide shortcut remains the only shortcut that hides or docks the window.

The application will enforce a single running instance. A second launch will activate the existing main window instead of creating another tray icon or competing for shortcuts.

Shortcut changes will be transactional:

1. Parse and validate the requested accelerator.
2. Attempt to register it without changing persisted configuration.
3. If registration succeeds, unregister the previous accelerator and persist the new value.
4. If any later step fails, restore the previous registration and configuration.

Startup registration failures will be surfaced through persistent release logs and an application-visible warning. The active shortcut reported by Settings must reflect the actual registered accelerator, not only the configured value.

## Draft Persistence

Draft state will be stored separately from `config.json` in the application config directory as `draft.json`. It will contain:

- Draft text.
- Selected Markdown target id.
- Original creation timestamp.
- Last update timestamp.

The frontend will debounce draft updates and send snapshots to Rust. Rust will write snapshots atomically. Startup will restore the latest valid draft before the editor is focused.

The debounce target is 150 milliseconds. Hide, close, tray quit, and orderly process exit will explicitly flush the current snapshot. Forced process termination can lose only edits made inside the final debounce window; guaranteeing every keystroke against immediate process termination would require synchronous disk I/O on each input and is intentionally out of scope.

Draft lifecycle rules:

- Hide, dock, Capture toggle attempts, and the main close button preserve the draft.
- The main close button becomes a hide action.
- A trash icon beside the existing Hide action provides explicit discard with confirmation when the draft is non-empty.
- Successful Markdown save clears the persisted draft.
- Failed Markdown save retains the persisted draft.
- An explicit Discard action clears both in-memory and persisted draft state.
- Tray Quit and normal application exit flush the latest pending draft snapshot.
- A malformed draft file is quarantined or ignored with a logged warning; it must not prevent application startup.

## Windows Backdrop

The application will use a native Windows backdrop where supported rather than relying solely on CSS `backdrop-filter` over a transparent WebView.

Backdrop selection:

- Windows 10 and Windows 11: prefer Acrylic because it samples the actual content behind the floating window.
- Mica is not used because it is a system/wallpaper material and does not satisfy the requirement to reflect the current underlying application.
- Unsupported, failed, remote-session, or compositor-disabled environments: use an opaque or strongly translucent theme-colored fallback that never exposes a black transparent surface.

The native effect will be applied when the main and Settings windows are created and reapplied after relevant show/restore events if necessary. CSS blur remains decorative only and must not be required for correct background rendering.

## Atomic Storage

Configuration, draft, and prepended Markdown content will use a shared atomic-write helper:

1. Create a temporary file in the destination directory.
2. Write and flush complete content.
3. Replace the destination using Windows-compatible rename/replace behavior.
4. Preserve the original destination if replacement fails.

Markdown prepend remains compatible with the current file format. Because prepend requires reading existing content, the implementation will retain that behavior but remove the partial-overwrite data-loss window. Large-file performance will be measured and logged.

## Diagnostics

Release builds will enable rotating file logs in the application log directory. Logs will include:

- Startup version and process identity.
- Shortcut registration, fallback, and activation failures.
- Window show/focus/backdrop failures.
- Draft load/save/clear failures without logging draft content.
- Atomic file replacement failures.

No note text, target file content, or sensitive path content should be logged beyond what is necessary to diagnose a filesystem error.

## Testing

Rust unit tests will cover shortcut state transitions, draft serialization and recovery, atomic replacement, note formatting, and failed-write preservation.

Frontend tests will cover restored draft initialization, close-as-hide behavior, explicit discard, save-success clearing, and save-failure retention where practical. If introducing a frontend test runner would materially expand scope, the state transitions will be extracted into testable pure functions and verified during the existing TypeScript build.

Manual Windows verification will cover:

- Capture while hidden, visible but unfocused, docked, and after Settings closes.
- Second-instance launch.
- Invalid and conflicting shortcut changes.
- Close, hide, tray quit, forced termination, and restart draft recovery.
- Save success and save failure draft behavior.
- Windows 10/11, lock/unlock, display changes, and remote-session backdrop fallback.

## Acceptance Criteria

- Capture always produces a visible, foreground, focused editor and never hides it.
- Only one application instance and tray icon can exist per user session.
- Unsaved text survives close, hide, tray quit, and restart. Forced termination restores the last snapshot written outside the documented 150 millisecond debounce window.
- Draft data clears only after successful save or explicit discard.
- No supported or fallback environment displays a black transparent background.
- Failed shortcut updates leave both active and persisted shortcuts unchanged.
- Interrupted config, draft, or Markdown writes do not truncate the previous valid file.
- Production failures leave actionable logs without captured note content.

## Runtime Polish Addendum (historical)

Superseded on 2026-10-27: the 22 px HWND region clipping proposal below is retained as history. The current implementation uses DWM native corners and a shared 10 px CSS radius. Custom background opacity is final and does not change with focus; Settings uses an opaque surface.

- Acrylic windows must be clipped at the native HWND level with a DPI-aware 22 px radius so the material cannot remain visible in rectangular CSS corner gaps.
- Native corner clipping must be refreshed after window size changes and applied to both the main and Settings windows.
- The Discard and Hide actions share one footer icon-button layout: identical 42 px hit areas, aligned visual centers, matching muted opacity and stroke weight, and no rectangular hover or focus background.
- Discard remains visually red and keeps its confirmation behavior, while disabled state uses reduced opacity without exposing its hit-area shape.
- Focus state is application-controlled rather than inferred from Windows Acrylic defaults: focused windows use a high-opacity theme surface for readability, while unfocused windows expose more Acrylic blur as the inactive-state cue.
- Losing focus triggers a delayed Acrylic refresh because Windows can otherwise replace inactive Acrylic with a flat material.
- The 22 px native clip targets only the WebView client area. Invisible native resize borders stay outside the rendered region, and internal Tauri resize handles preserve edge/corner resizing without exposing non-client pixels.

## 2026-10-27 用户确认的25px圆角

用户在磨砂外观调整后明确要求25px圆角。此前DWM系统默认圆角方案由本次指定半径替代：CSS使用25逻辑像素，原生client区域采用同半径的DPI换算裁剪，尺寸/缩放改变时刷新，贴边时解除。此前区域裁剪的材质残留问题需要重新验证；验收以实际25px边缘、磨砂材质贴合和可缩放性为准。

## 2026-10-27 最终选择：优先磨砂与稳定
用户明确接受系统控制的圆角半径，撤回精确25px要求。移除自定义HWND/子窗region、窗口消息子类、角点诊断和配套前端缩放覆盖层，恢复Tauri原生无边框缩放。使用DWM ROUND/DONOTROUND控制普通/贴边形态，CSS圆角采用8px配合系统外观，外轮廓以系统裁剪为准；原生Acrylic与减淡后的外阴影保留。此条覆盖上面的25px指定半径实验。
