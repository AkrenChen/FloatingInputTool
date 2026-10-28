mod window_behavior;
use window_behavior::{
    logical_px_to_physical, DockSide, MainWindowMode, MainWindowRuntimeState, StoredWindowBounds,
};
#[cfg(windows)]
mod acrylic;
mod config;
mod draft;
#[cfg(windows)]
mod native_composition;
mod native_dock;
#[cfg(windows)]
mod native_effects;
mod storage;
mod surface;
mod surface_runtime;

use config::{
    default_app_config, load_app_config, normalize_targets, sanitize_custom_theme,
    sanitize_targets, save_app_config, AppConfig, CustomTheme, LanguageMode, MarkdownTarget,
    SaveShortcutMode, ThemeMode, DEFAULT_HIDE_HOTKEY, DEFAULT_HOTKEY, DEFAULT_NEXT_TARGET_HOTKEY,
    DEFAULT_NOTE_TEMPLATE,
};
use draft::{load_draft, save_draft, DraftSnapshot};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex, MutexGuard,
};
use std::{fs, path::Path, str::FromStr};
use std::{
    thread,
    time::{Duration, Instant},
};
use storage::atomic_write_string;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, RunEvent, Runtime, State,
    WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent, Wry,
};
#[cfg(desktop)]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
#[cfg(windows)]
use windows_sys::Win32::Foundation::POINT;
#[cfg(windows)]
use windows_sys::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWMWCP_ROUND,
};
#[cfg(windows)]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetCursorPos, GetSystemMetrics, SetForegroundWindow, ShowWindowAsync,
    SM_REMOTESESSION, SW_RESTORE,
};

const MENU_ID_SHOW: &str = "show";
const MENU_ID_SETTINGS: &str = "settings";
const MENU_ID_THEME_FOLLOW_SYSTEM: &str = "theme-follow-system";
const MENU_ID_THEME_WHITE: &str = "theme-white";
const MENU_ID_THEME_DARK: &str = "theme-dark";
const MENU_ID_THEME_CUSTOM: &str = "theme-custom";
const MENU_ID_LANGUAGE_ENGLISH: &str = "language-english";
const MENU_ID_LANGUAGE_CHINESE: &str = "language-chinese";
const MENU_ID_QUIT: &str = "quit";
const WINDOW_LABEL_MAIN: &str = "main";
const WINDOW_LABEL_SETTINGS: &str = "settings";
const EVENT_THEME_MODE_CHANGED: &str = "theme-mode-changed";
const EVENT_APP_CONFIG_CHANGED: &str = "app-config-changed";
const EVENT_MAIN_WINDOW_MODE_CHANGED: &str = "main-window-mode-changed";
const EVENT_FOCUS_MAIN_EDITOR: &str = "focus-main-editor";
const EVENT_NATIVE_BACKDROP_CHANGED: &str = "native-backdrop-changed";
const AUTO_DOCK_SETTLE_MS: u64 = 180;
const WINDOW_BOUNDS_ANIMATION_MS: u64 = 120;
const WINDOW_BOUNDS_ANIMATION_STEPS: u32 = 10;
const WINDOW_BOUNDS_ANIMATION_PAINT_SETTLE_MS: u64 = 24;
const DRAFT_SAVE_DEBOUNCE_MS: u64 = 150;

struct ThemeMenuItems {
    follow_system: CheckMenuItem<Wry>,
    theme_white: CheckMenuItem<Wry>,
    theme_dark: CheckMenuItem<Wry>,
    theme_custom: CheckMenuItem<Wry>,
}

struct LanguageMenuItems {
    english: CheckMenuItem<Wry>,
    chinese: CheckMenuItem<Wry>,
}

struct TrayMenuItems {
    theme: ThemeMenuItems,
    language: LanguageMenuItems,
}

struct AppState {
    composing: AtomicBool,
    saving: AtomicBool,
    config: Mutex<AppConfig>,
    tray_menu_items: TrayMenuItems,
    active_hotkeys: Mutex<ActiveHotkeys>,
    hotkey_update_lock: Mutex<()>,
    runtime_warnings: Mutex<Vec<String>>,
    draft: Mutex<DraftSnapshot>,
    draft_save_generation: Mutex<u64>,
    native_backdrop: Mutex<NativeBackdropState>,
    main_window_state: Mutex<MainWindowRuntimeState>,
    auto_dock_schedule: Mutex<AutoDockSchedule>,
    transition_lock: Mutex<()>,
    transition_generation: AtomicU64,
    manual_drag_in_progress: Mutex<bool>,
    auto_dock_suppressed_until_ms: Mutex<u128>,
}

#[derive(Debug, Clone, Copy, Default)]
struct NativeBackdropState {
    main: bool,
    settings: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GlobalHotkeyAction {
    Capture,
    Hide,
    NextTarget,
}

#[derive(Debug, Clone, Default)]
struct ActiveHotkeys {
    capture: Option<RegisteredHotkey>,
    hide: Option<RegisteredHotkey>,
    next_target: Option<RegisteredHotkey>,
}

#[derive(Debug, Clone)]
struct RegisteredHotkey {
    accelerator: String,
    id: u32,
}

impl ActiveHotkeys {
    fn get(&self, action: GlobalHotkeyAction) -> Option<&RegisteredHotkey> {
        match action {
            GlobalHotkeyAction::Capture => self.capture.as_ref(),
            GlobalHotkeyAction::Hide => self.hide.as_ref(),
            GlobalHotkeyAction::NextTarget => self.next_target.as_ref(),
        }
    }

    fn set(&mut self, action: GlobalHotkeyAction, hotkey: Option<RegisteredHotkey>) {
        match action {
            GlobalHotkeyAction::Capture => self.capture = hotkey,
            GlobalHotkeyAction::Hide => self.hide = hotkey,
            GlobalHotkeyAction::NextTarget => self.next_target = hotkey,
        }
    }

    fn action_for_hotkey_id(&self, hotkey_id: u32) -> Option<GlobalHotkeyAction> {
        [
            (GlobalHotkeyAction::Capture, self.capture.as_ref()),
            (GlobalHotkeyAction::Hide, self.hide.as_ref()),
            (GlobalHotkeyAction::NextTarget, self.next_target.as_ref()),
        ]
        .into_iter()
        .find_map(|(action, active_hotkey)| {
            active_hotkey
                .filter(|hotkey| hotkey.id == hotkey_id)
                .map(|_| action)
        })
    }
}

impl RegisteredHotkey {
    fn from_accelerator(accelerator: &str) -> Result<Self, String> {
        let shortcut = Shortcut::from_str(accelerator)
            .map_err(|error| format!("failed to parse hotkey {accelerator}: {error}"))?;
        Ok(Self {
            accelerator: accelerator.to_string(),
            id: shortcut.id(),
        })
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ThemeModeChangedPayload {
    theme_mode: ThemeMode,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HotkeyUpdateResponse {
    config: AppConfig,
    warning: Option<String>,
    active_hotkey: Option<String>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HotkeyRuntimeStatus {
    capture: Option<String>,
    hide: Option<String>,
    next_target: Option<String>,
    warnings: Vec<String>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeBackdropStatus {
    available: bool,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MainWindowModeChangedPayload {
    mode: MainWindowMode,
    dock_side: Option<DockSide>,
}

#[tauri::command]
fn get_app_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    state
        .config
        .lock()
        .map(|config| config.clone())
        .map_err(|error| format!("failed to read app state: {error}"))
}

#[tauri::command]
fn get_hotkey_runtime_status(state: State<'_, AppState>) -> Result<HotkeyRuntimeStatus, String> {
    let active_hotkeys = state
        .active_hotkeys
        .lock()
        .map_err(|error| format!("failed to lock active hotkey state: {error}"))?
        .clone();
    let warnings = state
        .runtime_warnings
        .lock()
        .map_err(|error| format!("failed to lock runtime warnings: {error}"))?
        .clone();

    Ok(HotkeyRuntimeStatus {
        capture: active_hotkeys.capture.map(|hotkey| hotkey.accelerator),
        hide: active_hotkeys.hide.map(|hotkey| hotkey.accelerator),
        next_target: active_hotkeys.next_target.map(|hotkey| hotkey.accelerator),
        warnings,
    })
}

#[tauri::command]
fn get_draft(state: State<'_, AppState>) -> Result<DraftSnapshot, String> {
    state
        .draft
        .lock()
        .map_err(|error| format!("failed to lock draft state: {error}"))
        .map(|draft| draft.clone())
}

#[tauri::command]
fn get_native_backdrop_status(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<NativeBackdropStatus, String> {
    let backdrop = state
        .native_backdrop
        .lock()
        .map_err(|error| format!("failed to lock native backdrop state: {error}"))?;
    let available = match window.label() {
        WINDOW_LABEL_SETTINGS => backdrop.settings,
        _ => backdrop.main,
    };
    Ok(NativeBackdropStatus { available })
}

#[tauri::command]
fn update_main_surface(
    app: AppHandle,
    parameters: surface_runtime::SurfaceRequest,
) -> Result<(), String> {
    parameters.effects.validate()?;
    surface_runtime::refresh(&app, Some(parameters));
    Ok(())
}

#[tauri::command]
fn update_draft(
    app: AppHandle,
    state: State<'_, AppState>,
    draft: DraftSnapshot,
) -> Result<(), String> {
    *state
        .draft
        .lock()
        .map_err(|error| format!("failed to lock draft state: {error}"))? = draft;
    let generation = {
        let mut generation = state
            .draft_save_generation
            .lock()
            .map_err(|error| format!("failed to lock draft save generation: {error}"))?;
        *generation += 1;
        *generation
    };

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(DRAFT_SAVE_DEBOUNCE_MS));
        let state = app.state::<AppState>();
        let current_generation = match state.draft_save_generation.lock() {
            Ok(generation) => *generation,
            Err(error) => {
                log::warn!("failed to lock draft save generation: {error}");
                return;
            }
        };
        if generation != current_generation {
            return;
        }
        if let Err(error) = flush_draft_inner(&app, &state) {
            log::warn!("debounced draft save failed: {error}");
        }
    });
    Ok(())
}

#[tauri::command]
fn flush_draft(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    flush_draft_inner(&app, &state)
}

fn flush_draft_inner<R: Runtime>(
    app: &AppHandle<R>,
    state: &State<'_, AppState>,
) -> Result<(), String> {
    let draft = state
        .draft
        .lock()
        .map_err(|error| format!("failed to lock draft state: {error}"))?
        .clone();
    save_draft(app, &draft)
}

fn clear_draft_inner<R: Runtime>(
    app: &AppHandle<R>,
    state: &State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut generation = state
            .draft_save_generation
            .lock()
            .map_err(|error| format!("failed to lock draft save generation: {error}"))?;
        *generation += 1;
    }
    let empty_draft = DraftSnapshot::default();
    *state
        .draft
        .lock()
        .map_err(|error| format!("failed to lock draft state: {error}"))? = empty_draft.clone();
    save_draft(app, &empty_draft)
}

#[tauri::command]
fn set_theme_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    theme_mode: ThemeMode,
) -> Result<AppConfig, String> {
    let mut config = state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app state: {error}"))?;

    config.theme_mode = theme_mode;
    save_app_config(&app, &config)?;
    sync_theme_menu_items(&state.tray_menu_items.theme, theme_mode)?;

    let next_config = config.clone();
    drop(config);

    app.emit(
        EVENT_THEME_MODE_CHANGED,
        ThemeModeChangedPayload { theme_mode },
    )
    .map_err(|error| format!("failed to emit theme mode change: {error}"))?;
    app.emit(EVENT_APP_CONFIG_CHANGED, next_config.clone())
        .map_err(|error| format!("failed to emit app config change: {error}"))?;

    Ok(next_config)
}

#[tauri::command]
fn set_language_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    language_mode: LanguageMode,
) -> Result<AppConfig, String> {
    let mut config = state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app state: {error}"))?;

    config.language_mode = language_mode;
    save_app_config(&app, &config)?;
    sync_language_menu_items(&state.tray_menu_items.language, language_mode)?;

    let next_config = config.clone();
    drop(config);

    app.emit(EVENT_APP_CONFIG_CHANGED, next_config.clone())
        .map_err(|error| format!("failed to emit app config change: {error}"))?;

    Ok(next_config)
}

#[tauri::command]
fn set_custom_theme(
    app: AppHandle,
    state: State<'_, AppState>,
    custom_theme: CustomTheme,
) -> Result<AppConfig, String> {
    let mut config = state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app state: {error}"))?;

    config.custom_theme = sanitize_custom_theme(custom_theme);
    save_app_config(&app, &config)?;

    let next_config = config.clone();
    drop(config);

    app.emit(EVENT_APP_CONFIG_CHANGED, next_config.clone())
        .map_err(|error| format!("failed to emit app config change: {error}"))?;

    Ok(next_config)
}

#[tauri::command]
fn set_target_file_path(
    app: AppHandle,
    state: State<'_, AppState>,
    target_file_path: String,
) -> Result<AppConfig, String> {
    let normalized_target_file_path = normalize_target_file_path(&target_file_path)?;
    let mut config = state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app state: {error}"))?;

    config.target_file_path = normalized_target_file_path;
    let active_target_id = config.active_target_id.clone();
    let active_target_path = config.target_file_path.clone();
    if let Some(active_target) = config
        .targets
        .iter_mut()
        .find(|target| target.id == active_target_id)
    {
        active_target.path = active_target_path;
    }
    normalize_targets(&app, &mut config)?;
    save_app_config(&app, &config)?;

    let next_config = config.clone();
    drop(config);

    app.emit(EVENT_APP_CONFIG_CHANGED, next_config.clone())
        .map_err(|error| format!("failed to emit app config change: {error}"))?;

    Ok(next_config)
}

#[tauri::command]
fn set_active_target_id(
    app: AppHandle,
    state: State<'_, AppState>,
    active_target_id: String,
) -> Result<AppConfig, String> {
    let mut config = state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app state: {error}"))?;

    if !config
        .targets
        .iter()
        .any(|target| target.id == active_target_id)
    {
        return Err(String::from("selected target does not exist"));
    }

    config.active_target_id = active_target_id;
    normalize_targets(&app, &mut config)?;
    save_app_config(&app, &config)?;

    let next_config = config.clone();
    drop(config);

    app.emit(EVENT_APP_CONFIG_CHANGED, next_config.clone())
        .map_err(|error| format!("failed to emit app config change: {error}"))?;

    Ok(next_config)
}

#[tauri::command]
fn set_markdown_targets(
    app: AppHandle,
    state: State<'_, AppState>,
    targets: Vec<MarkdownTarget>,
    active_target_id: Option<String>,
) -> Result<AppConfig, String> {
    let mut config = state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app state: {error}"))?;

    let normalized_targets = targets
        .into_iter()
        .map(|mut target| {
            target.path = normalize_target_file_path(&target.path)?;
            Ok(target)
        })
        .collect::<Result<Vec<_>, String>>()?;

    config.targets = sanitize_targets(normalized_targets, &config.target_file_path);
    if let Some(active_target_id) = active_target_id {
        config.active_target_id = active_target_id;
    }
    normalize_targets(&app, &mut config)?;
    save_app_config(&app, &config)?;

    let next_config = config.clone();
    drop(config);

    app.emit(EVENT_APP_CONFIG_CHANGED, next_config.clone())
        .map_err(|error| format!("failed to emit app config change: {error}"))?;

    Ok(next_config)
}

#[tauri::command]
fn set_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    hotkey: String,
) -> Result<HotkeyUpdateResponse, String> {
    set_action_hotkey(&app, &state, GlobalHotkeyAction::Capture, hotkey)
}

#[tauri::command]
fn set_hide_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    hotkey: String,
) -> Result<HotkeyUpdateResponse, String> {
    set_action_hotkey(&app, &state, GlobalHotkeyAction::Hide, hotkey)
}

#[tauri::command]
fn set_next_target_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    hotkey: String,
) -> Result<HotkeyUpdateResponse, String> {
    set_action_hotkey(&app, &state, GlobalHotkeyAction::NextTarget, hotkey)
}

fn set_action_hotkey<R: Runtime>(
    app: &AppHandle<R>,
    state: &State<'_, AppState>,
    action: GlobalHotkeyAction,
    hotkey: String,
) -> Result<HotkeyUpdateResponse, String> {
    let _update_guard = state
        .hotkey_update_lock
        .lock()
        .map_err(|error| format!("failed to lock hotkey update state: {error}"))?;
    let normalized_hotkey = hotkey.trim().to_string();
    let current_config = state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app state: {error}"))?
        .clone();

    let current_active = state
        .active_hotkeys
        .lock()
        .map_err(|error| format!("failed to lock active hotkey state: {error}"))?
        .get(action)
        .cloned();

    let requested = match RegisteredHotkey::from_accelerator(&normalized_hotkey) {
        Ok(requested) => requested,
        Err(error) => {
            return Ok(HotkeyUpdateResponse {
                config: current_config,
                warning: Some(error),
                active_hotkey: current_active.map(|hotkey| hotkey.accelerator),
            });
        }
    };

    if current_active.as_ref().map(|hotkey| hotkey.id) == Some(requested.id) {
        let mut next_config = current_config.clone();
        set_configured_hotkey(&mut next_config, action, normalized_hotkey.clone());
        save_app_config(app, &next_config)?;
        *state
            .config
            .lock()
            .map_err(|error| format!("failed to lock app state: {error}"))? = next_config.clone();
        return Ok(HotkeyUpdateResponse {
            config: next_config,
            warning: None,
            active_hotkey: Some(normalized_hotkey),
        });
    }

    {
        let active_hotkeys = state
            .active_hotkeys
            .lock()
            .map_err(|error| format!("failed to lock active hotkey state: {error}"))?;
        if let Some(conflicting_action) = active_hotkeys.action_for_hotkey_id(requested.id) {
            return Ok(HotkeyUpdateResponse {
                config: current_config,
                warning: Some(format!(
                    "{} hotkey conflicts with the active {} hotkey.",
                    hotkey_action_label(action),
                    hotkey_action_label(conflicting_action)
                )),
                active_hotkey: current_active.map(|hotkey| hotkey.accelerator),
            });
        }
    }

    if let Err(error) = app.global_shortcut().register(normalized_hotkey.as_str()) {
        return Ok(HotkeyUpdateResponse {
            config: current_config,
            warning: Some(format!(
                "{} hotkey could not be activated. It may be invalid or already in use. {error}",
                hotkey_action_label(action)
            )),
            active_hotkey: current_active.map(|hotkey| hotkey.accelerator),
        });
    }

    let mut next_config = current_config.clone();
    set_configured_hotkey(&mut next_config, action, normalized_hotkey.clone());
    if let Err(error) = save_app_config(app, &next_config) {
        let _ = app.global_shortcut().unregister(normalized_hotkey.as_str());
        return Err(error);
    }

    if let Some(previous_hotkey) = current_active.as_ref() {
        if let Err(error) = app
            .global_shortcut()
            .unregister(previous_hotkey.accelerator.as_str())
        {
            let _ = app.global_shortcut().unregister(normalized_hotkey.as_str());
            let _ = save_app_config(app, &current_config);
            return Err(format!(
                "failed to deactivate previous {} hotkey: {error}",
                hotkey_action_label(action)
            ));
        }
    }

    state
        .active_hotkeys
        .lock()
        .map_err(|error| format!("failed to lock active hotkey state: {error}"))?
        .set(action, Some(requested));
    *state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app state: {error}"))? = next_config.clone();
    app.emit(EVENT_APP_CONFIG_CHANGED, next_config.clone())
        .map_err(|error| format!("failed to emit app config change: {error}"))?;

    Ok(HotkeyUpdateResponse {
        config: next_config,
        warning: None,
        active_hotkey: Some(normalized_hotkey),
    })
}

fn set_configured_hotkey(config: &mut AppConfig, action: GlobalHotkeyAction, hotkey: String) {
    match action {
        GlobalHotkeyAction::Capture => config.hotkey = hotkey,
        GlobalHotkeyAction::Hide => config.hide_hotkey = hotkey,
        GlobalHotkeyAction::NextTarget => config.next_target_hotkey = hotkey,
    }
}

#[tauri::command]
fn set_save_shortcut_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    save_shortcut_mode: SaveShortcutMode,
) -> Result<AppConfig, String> {
    let mut config = state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app state: {error}"))?;

    config.save_shortcut_mode = save_shortcut_mode;
    save_app_config(&app, &config)?;

    let next_config = config.clone();
    drop(config);

    app.emit(EVENT_APP_CONFIG_CHANGED, next_config.clone())
        .map_err(|error| format!("failed to emit app config change: {error}"))?;

    Ok(next_config)
}

#[tauri::command]
fn set_note_template(
    app: AppHandle,
    state: State<'_, AppState>,
    note_template: String,
) -> Result<AppConfig, String> {
    let mut config = state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app state: {error}"))?;

    config.note_template = sanitize_note_template(&note_template);
    save_app_config(&app, &config)?;

    let next_config = config.clone();
    drop(config);

    app.emit(EVENT_APP_CONFIG_CHANGED, next_config.clone())
        .map_err(|error| format!("failed to emit app config change: {error}"))?;

    Ok(next_config)
}

#[tauri::command]
fn save_note(
    app: AppHandle,
    state: State<'_, AppState>,
    note_text: String,
    target_id: String,
) -> Result<(), String> {
    if note_text.trim().is_empty() {
        return Err(String::from("note content cannot be empty"));
    }

    let config = state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app state: {error}"))?
        .clone();

    let target_file_path = config
        .targets
        .iter()
        .find(|target| target.id == target_id)
        .map(|target| target.path.clone())
        .ok_or_else(|| format!("save target {target_id} is missing"))?;
    let note_block = format_note_block(&note_text, &config.note_template);
    prepend_to_file(&target_file_path, &note_block)?;
    if let Err(error) = clear_draft_inner(&app, &state) {
        log::error!("note saved but persisted draft could not be cleared: {error}");
    }
    Ok(())
}

fn activate_next_target<R: Runtime>(app: &AppHandle<R>) -> Result<AppConfig, String> {
    let state = app.state::<AppState>();
    let mut config = state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app state: {error}"))?;

    normalize_targets(app, &mut config)?;
    if config.targets.len() <= 1 {
        return Ok(config.clone());
    }

    let current_index = config
        .targets
        .iter()
        .position(|target| target.id == config.active_target_id)
        .unwrap_or(0);
    let next_index = (current_index + 1) % config.targets.len();
    config.active_target_id = config.targets[next_index].id.clone();
    normalize_targets(app, &mut config)?;
    save_app_config(app, &config)?;

    let next_config = config.clone();
    drop(config);

    app.emit(EVENT_APP_CONFIG_CHANGED, next_config.clone())
        .map_err(|error| format!("failed to emit app config change: {error}"))?;

    Ok(next_config)
}

#[tauri::command]
fn begin_manual_window_drag(state: State<'_, AppState>) -> Result<(), String> {
    let mut is_dragging = state
        .manual_drag_in_progress
        .lock()
        .map_err(|error| format!("failed to lock manual drag state: {error}"))?;
    *is_dragging = true;
    Ok(())
}

#[tauri::command]
async fn end_manual_window_drag(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if !window_behavior::finish_manual_drag(&state.manual_drag_in_progress)? {
        log::debug!("ignored duplicate manual drag completion");
        return Ok(());
    }

    sync_main_window_mode_after_move(&app)
}

#[tauri::command]
fn is_cursor_inside_main_window(app: AppHandle, padding_px: Option<i32>) -> Result<bool, String> {
    let window = app
        .get_webview_window(WINDOW_LABEL_MAIN)
        .ok_or_else(|| String::from("main window is missing"))?;
    let mut bounds = capture_window_bounds(&window)?;
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let docked = app
        .state::<AppState>()
        .main_window_state
        .lock()
        .map(|runtime| runtime.mode.is_docked())
        .unwrap_or(false);
    if !docked {
        bounds = surface::visible_bounds(
            bounds,
            logical_px_to_physical(surface::SHADOW_INSET, scale) as i32,
        );
    }
    let padding = logical_px_to_physical(padding_px.unwrap_or(0).max(0) as u32, scale) as i32;

    let Some((cursor_x, cursor_y)) = current_cursor_position() else {
        return Ok(false);
    };

    let left = bounds.x - padding;
    let top = bounds.y - padding;
    let right = bounds.x + bounds.width as i32 + padding;
    let bottom = bounds.y + bounds.height as i32 + padding;
    let is_inside = cursor_x >= left && cursor_x < right && cursor_y >= top && cursor_y < bottom;
    log::info!(
    "[side-hide] is_cursor_inside_main_window cursor=({}, {}) bounds=({}, {}, {}, {}) padding={} inside={}",
    cursor_x,
    cursor_y,
    left,
    top,
    right,
    bottom,
    padding,
    is_inside
  );
    Ok(is_inside)
}

#[tauri::command]
async fn hide_or_dock_main_window(app: AppHandle) -> Result<(), String> {
    hide_or_dock_main_window_inner(&app)
}

fn hide_or_dock_main_window_inner<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let state = app.state::<AppState>();
    let transition = begin_window_transition(&state)?;
    let runtime_state = state
        .main_window_state
        .lock()
        .map_err(|error| error.to_string())?
        .clone();
    if runtime_state.mode.is_docked() {
        return Ok(());
    }
    let config = state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app config: {error}"))?
        .clone();

    let window = app
        .get_webview_window(WINDOW_LABEL_MAIN)
        .ok_or_else(|| String::from("main window is missing"))?;

    if config.side_hide_enabled == 0 {
        window
            .hide()
            .map_err(|error| format!("failed to hide main window: {error}"))?;
        update_main_window_runtime_state(
            app,
            &state,
            MainWindowRuntimeState {
                mode: MainWindowMode::Normal,
                dock_side: None,
                normal_bounds: Some(capture_window_bounds(&window)?),
                docked_bounds: None,
            },
        )?;
        return Ok(());
    }

    let current_bounds = capture_window_bounds(&window)?;
    if let Some((dock_side, docked_bounds)) =
        compute_docked_bounds(&window, &current_bounds, &config)?
    {
        suppress_auto_dock_for(&state, WINDOW_BOUNDS_ANIMATION_MS + AUTO_DOCK_SETTLE_MS)?;
        apply_window_bounds_animated(&window, current_bounds, docked_bounds, &transition)?;
        set_window_resizable(&window, false)?;
        settle_window_bounds_animation();
        update_main_window_runtime_state(
            app,
            &state,
            runtime_state.dock(dock_side, current_bounds, docked_bounds),
        )?;
    } else {
        window
            .hide()
            .map_err(|error| format!("failed to hide main window: {error}"))?;
        update_main_window_runtime_state(
            app,
            &state,
            MainWindowRuntimeState {
                mode: MainWindowMode::Normal,
                dock_side: None,
                normal_bounds: Some(current_bounds),
                docked_bounds: None,
            },
        )?;
    }

    Ok(())
}

#[tauri::command]
async fn restore_docked_main_window(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    restore_docked_main_window_inner(&app, &state)
}

fn restore_docked_main_window_inner<R: Runtime>(
    app: &AppHandle<R>,
    state: &State<'_, AppState>,
) -> Result<(), String> {
    let transition = begin_window_transition(&state)?;
    let window = app
        .get_webview_window(WINDOW_LABEL_MAIN)
        .ok_or_else(|| String::from("main window is missing"))?;

    let runtime_state = state
        .main_window_state
        .lock()
        .map_err(|error| format!("failed to lock main window state: {error}"))?
        .clone();

    let Some(saved_normal_bounds) = runtime_state.normal_bounds else {
        return Ok(());
    };

    if runtime_state.mode == MainWindowMode::Normal {
        return Ok(());
    }

    let normal_bounds =
        compute_restore_bounds(&window, saved_normal_bounds, runtime_state.dock_side)?;

    suppress_auto_dock_for(
        &state,
        WINDOW_BOUNDS_ANIMATION_MS + (AUTO_DOCK_SETTLE_MS * 3),
    )?;
    update_main_window_runtime_state(
        app,
        state,
        MainWindowRuntimeState {
            mode: MainWindowMode::ExpandedFromDock,
            dock_side: runtime_state.dock_side,
            normal_bounds: Some(normal_bounds),
            docked_bounds: runtime_state.docked_bounds,
        },
    )?;
    apply_window_bounds_animated(
        &window,
        capture_window_bounds(&window)?,
        normal_bounds,
        &transition,
    )?;
    set_window_resizable(&window, true)?;
    log::info!(
        "[side-hide] restore_docked_main_window normal_bounds=({}, {}, {}, {}) mode_before={:?}",
        normal_bounds.x,
        normal_bounds.y,
        normal_bounds.width,
        normal_bounds.height,
        runtime_state.mode
    );

    Ok(())
}

#[tauri::command]
async fn redock_main_window(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let transition = begin_window_transition(&state)?;
    redock_remembered(&app, &state, &transition)
}

fn redock_remembered<R: Runtime>(
    app: &AppHandle<R>,
    state: &State<'_, AppState>,
    transition: &WindowTransition<'_>,
) -> Result<(), String> {
    let runtime = state
        .main_window_state
        .lock()
        .map_err(|error| error.to_string())?
        .clone();
    let Some((side, bounds)) = runtime.capture_finished_dock() else {
        return Ok(());
    };
    let window = app
        .get_webview_window(WINDOW_LABEL_MAIN)
        .ok_or("main window is missing")?;
    suppress_auto_dock_for(state, WINDOW_BOUNDS_ANIMATION_MS + AUTO_DOCK_SETTLE_MS)?;
    apply_window_bounds_animated(&window, capture_window_bounds(&window)?, bounds, transition)?;
    set_window_resizable(&window, false)?;
    update_main_window_runtime_state(
        app,
        state,
        MainWindowRuntimeState {
            mode: side.to_docked_mode(),
            ..runtime
        },
    )
}

#[tauri::command]
async fn finish_capture(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let transition = begin_window_transition(&state)?;
    let mode = state
        .main_window_state
        .lock()
        .map_err(|error| error.to_string())?
        .mode;
    if mode == MainWindowMode::ExpandedFromDock {
        redock_remembered(&app, &state, &transition)
    } else if mode.is_docked() {
        Ok(())
    } else {
        app.get_webview_window(WINDOW_LABEL_MAIN)
            .ok_or("main window is missing")?
            .hide()
            .map_err(|error| error.to_string())
    }
}

fn sync_theme_menu_items(items: &ThemeMenuItems, active_mode: ThemeMode) -> Result<(), String> {
    items
        .follow_system
        .set_checked(active_mode == ThemeMode::FollowSystem)
        .map_err(|error| format!("failed to update tray theme menu: {error}"))?;
    items
        .theme_white
        .set_checked(active_mode == ThemeMode::ThemeWhite)
        .map_err(|error| format!("failed to update tray theme menu: {error}"))?;
    items
        .theme_dark
        .set_checked(active_mode == ThemeMode::ThemeDark)
        .map_err(|error| format!("failed to update tray theme menu: {error}"))?;
    items
        .theme_custom
        .set_checked(active_mode == ThemeMode::Custom)
        .map_err(|error| format!("failed to update tray theme menu: {error}"))?;

    Ok(())
}

fn sync_language_menu_items(
    items: &LanguageMenuItems,
    active_mode: LanguageMode,
) -> Result<(), String> {
    items
        .english
        .set_checked(active_mode == LanguageMode::English)
        .map_err(|error| format!("failed to update tray language menu: {error}"))?;
    items
        .chinese
        .set_checked(active_mode == LanguageMode::Chinese)
        .map_err(|error| format!("failed to update tray language menu: {error}"))?;

    Ok(())
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _transition = begin_window_transition(&state)?;
    show_main_window_inner(app, &state)
}

fn show_main_window_inner<R: Runtime>(
    app: &AppHandle<R>,
    state: &State<'_, AppState>,
) -> Result<(), String> {
    restore_main_window_to_normal(app, state)?;

    if let Some(settings_window) = app.get_webview_window(WINDOW_LABEL_SETTINGS) {
        if settings_window.is_visible().unwrap_or(false) {
            settings_window
                .hide()
                .map_err(|error| format!("failed to hide Settings before capture: {error}"))?;
        }
    }

    let window = app
        .get_webview_window(WINDOW_LABEL_MAIN)
        .ok_or_else(|| String::from("main window is missing"))?;
    window
        .set_enabled(true)
        .map_err(|error| format!("failed to enable main window: {error}"))?;
    window
        .show()
        .map_err(|error| format!("failed to show main window: {error}"))?;
    window
        .unminimize()
        .map_err(|error| format!("failed to restore main window: {error}"))?;
    update_native_backdrop(app, &window);
    focus_main_window(&window)?;
    app.emit(EVENT_FOCUS_MAIN_EDITOR, ())
        .map_err(|error| format!("failed to focus main editor: {error}"))?;

    Ok(())
}

fn focus_main_window<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), String> {
    #[cfg(windows)]
    {
        let hwnd = window
            .hwnd()
            .map_err(|error| format!("failed to resolve main window handle: {error}"))?;
        unsafe {
            let raw_hwnd = hwnd.0 as _;
            let _ = ShowWindowAsync(raw_hwnd, SW_RESTORE);
            let _ = BringWindowToTop(raw_hwnd);
            if SetForegroundWindow(raw_hwnd) == 0 {
                log::warn!("Windows declined SetForegroundWindow for the main window");
            }
        }
    }

    window
        .set_focus()
        .map_err(|error| format!("failed to focus main window: {error}"))?;
    Ok(())
}

fn update_native_backdrop<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) {
    if window.label() == WINDOW_LABEL_MAIN {
        surface_runtime::refresh(app, None);
        return;
    }
    let available = apply_native_backdrop(window);
    apply_native_corner_preference(window, true);
    let state = app.state::<AppState>();
    match state.native_backdrop.lock() {
        Ok(mut backdrop) => match window.label() {
            WINDOW_LABEL_SETTINGS => backdrop.settings = available,
            _ => backdrop.main = available,
        },
        Err(error) => log::warn!("failed to lock native backdrop state: {error}"),
    }
    if let Err(error) = window.emit(
        EVENT_NATIVE_BACKDROP_CHANGED,
        NativeBackdropStatus { available },
    ) {
        log::warn!("failed to emit native backdrop status: {error}");
    }
}

#[cfg(windows)]
fn apply_native_corner_preference<R: Runtime>(window: &WebviewWindow<R>, rounded: bool) {
    if window.label() == WINDOW_LABEL_MAIN {
        return;
    }
    let hwnd = match window.hwnd() {
        Ok(hwnd) => hwnd,
        Err(error) => {
            log::warn!("failed to resolve native corner window: {error}");
            return;
        }
    };
    let preference = if rounded {
        DWMWCP_ROUND
    } else {
        DWMWCP_DONOTROUND
    };
    let result = unsafe {
        DwmSetWindowAttribute(
            hwnd.0 as _,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            std::ptr::from_ref(&preference).cast(),
            std::mem::size_of_val(&preference) as u32,
        )
    };
    if result < 0 {
        log::warn!(
            "native corner preference unavailable for {}: HRESULT {result:#010x}",
            window.label()
        );
    }
}

#[cfg(not(windows))]
fn apply_native_corner_preference<R: Runtime>(_window: &WebviewWindow<R>, _rounded: bool) {}

#[cfg(windows)]
fn apply_native_backdrop<R: Runtime>(window: &WebviewWindow<R>) -> bool {
    let is_remote_session = unsafe { GetSystemMetrics(SM_REMOTESESSION) != 0 };
    if is_remote_session {
        let _ = window_vibrancy::clear_acrylic(window);
        if let Ok(hwnd) = window.hwnd() {
            let _ = acrylic::set_enabled(hwnd.0 as _, false);
        }
        log::info!(
            "native Acrylic disabled for remote session window {}",
            window.label()
        );
        return false;
    }

    let _ = window_vibrancy::clear_acrylic(window);
    if window.label() == WINDOW_LABEL_MAIN {
        let result = window
            .hwnd()
            .map_err(|error| error.to_string())
            .and_then(|hwnd| acrylic::set_enabled(hwnd.0 as _, true));
        return match result {
            Ok(()) => {
                log::info!("accent Acrylic enabled for window {}", window.label());
                true
            }
            Err(error) => {
                log::warn!("accent Acrylic unavailable for {}: {error}", window.label());
                false
            }
        };
    }
    match window_vibrancy::apply_acrylic(window, None) {
        Ok(()) => {
            log::info!("native Acrylic enabled for window {}", window.label());
            true
        }
        Err(error) => {
            log::warn!(
                "native Acrylic unavailable for window {}: {error}",
                window.label()
            );
            false
        }
    }
}

#[cfg(not(windows))]
fn apply_native_backdrop<R: Runtime>(_window: &WebviewWindow<R>) -> bool {
    false
}

fn dispatch_window_action<R: Runtime>(
    app: &AppHandle<R>,
    action: fn(&AppHandle<R>) -> Result<(), String>,
) {
    let app = app.clone();
    thread::spawn(move || {
        if let Err(error) = action(&app) {
            log::warn!("{error}");
        }
    });
}

fn hide_main_window_to_background<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _transition = begin_window_transition(&state)?;
    if let Err(error) = flush_draft_inner(app, &state) {
        log::warn!("{error}");
    }
    app.get_webview_window(WINDOW_LABEL_MAIN)
        .ok_or("main window is missing")?
        .hide()
        .map_err(|error| error.to_string())
}

fn open_settings<R: Runtime>(app: &AppHandle<R>) {
    dispatch_window_action(app, open_or_focus_settings_window);
}

fn handle_global_hotkey<R: Runtime>(app: &AppHandle<R>, shortcut: &Shortcut) {
    let state = app.state::<AppState>();
    let action = match state.active_hotkeys.lock() {
        Ok(active_hotkeys) => active_hotkeys.action_for_hotkey_id(shortcut.id()),
        Err(error) => {
            log::warn!("failed to lock active hotkey state: {error}");
            None
        }
    };

    match action {
        Some(GlobalHotkeyAction::Capture) => {
            dispatch_window_action(app, show_main_window);
        }
        Some(GlobalHotkeyAction::Hide) => {
            dispatch_window_action(app, hide_or_dock_main_window_inner);
        }
        Some(GlobalHotkeyAction::NextTarget) => {
            if let Err(error) = activate_next_target(app) {
                log::warn!("{error}");
            }
        }
        None => {
            log::warn!("received unknown global shortcut event: {shortcut}");
        }
    }
}

fn enable_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(main_window) = app.get_webview_window(WINDOW_LABEL_MAIN) {
        let _ = main_window.set_enabled(true);
        let _ = focus_main_window(&main_window);
        let _ = app.emit(EVENT_FOCUS_MAIN_EDITOR, ());
    }
}

#[tauri::command]
async fn open_settings_window(app: AppHandle) -> Result<(), String> {
    open_or_focus_settings_window(&app)
}

#[tauri::command]
async fn close_settings_window(app: AppHandle) -> Result<(), String> {
    close_settings_window_inner(&app)
}

fn close_settings_window_inner<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _transition = begin_window_transition(&state)?;
    if let Some(window) = app.get_webview_window(WINDOW_LABEL_SETTINGS) {
        window.hide().map_err(|error| error.to_string())?;
    }
    show_main_window_inner(app, &state)
}

fn attach_settings_window_events<R: Runtime>(
    app: &AppHandle<R>,
    settings_window: &WebviewWindow<R>,
) {
    let app_handle = app.clone();
    settings_window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            dispatch_window_action(&app_handle, close_settings_window_inner);
        }
        WindowEvent::Destroyed => enable_main_window(&app_handle),
        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
            if let Some(window) = app_handle.get_webview_window(WINDOW_LABEL_SETTINGS) {
                apply_native_corner_preference(&window, true);
            }
        }
        _ => {}
    });
}

fn attach_main_window_events<R: Runtime>(app: &AppHandle<R>, main_window: &WebviewWindow<R>) {
    let app_handle = app.clone();
    main_window.on_window_event(move |event| match event {
        WindowEvent::Moved(_) => schedule_main_window_auto_dock(&app_handle),
        WindowEvent::Focused(_) => surface_runtime::refresh(&app_handle, None),
        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
            let state = app_handle.state::<AppState>();
            let rounded = state
                .main_window_state
                .lock()
                .map(|runtime| !runtime.mode.is_docked())
                .unwrap_or(true);
            if let Some(window) = app_handle.get_webview_window(WINDOW_LABEL_MAIN) {
                apply_native_corner_preference(&window, rounded);
            }
            surface_runtime::refresh(&app_handle, None);
        }
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            dispatch_window_action(&app_handle, hide_main_window_to_background);
        }
        WindowEvent::Destroyed => surface_runtime::clear(),
        _ => {}
    });
}

fn build_settings_window<R: Runtime>(
    app: &AppHandle<R>,
    _main_window: &WebviewWindow<R>,
) -> Result<WebviewWindow<R>, String> {
    let window = WebviewWindowBuilder::new(
        app,
        WINDOW_LABEL_SETTINGS,
        WebviewUrl::App("index.html".into()),
    )
    .title("FloatingInputTool Settings")
    .inner_size(560.0, 620.0)
    .min_inner_size(500.0, 560.0)
    .resizable(true)
    .fullscreen(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(true)
    .center()
    .build()
    .map_err(|error| format!("failed to create Settings window: {error}"))?;
    update_native_backdrop(app, &window);
    Ok(window)
}

fn open_or_focus_settings_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _transition = begin_window_transition(&state)?;
    show_main_window_inner(app, &state)?;

    let main_window = app
        .get_webview_window(WINDOW_LABEL_MAIN)
        .ok_or_else(|| String::from("main window is missing"))?;

    if let Some(settings_window) = app.get_webview_window(WINDOW_LABEL_SETTINGS) {
        let _ = settings_window.show();
        let _ = settings_window.unminimize();
        let _ = settings_window.set_focus();
        main_window.set_enabled(false).map_err(|error| {
            format!("failed to disable main window while focusing Settings: {error}")
        })?;
        return Ok(());
    }

    match build_settings_window(app, &main_window) {
        Ok(settings_window) => {
            attach_settings_window_events(app, &settings_window);
            let _ = settings_window.set_focus();
            main_window.set_enabled(false).map_err(|error| {
                format!("failed to disable main window after opening Settings: {error}")
            })?;
            Ok(())
        }
        Err(error) => {
            let _ = main_window.set_enabled(true);
            Err(error)
        }
    }
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        MENU_ID_SHOW => {
            dispatch_window_action(app, show_main_window);
        }
        MENU_ID_SETTINGS => open_settings(app),
        MENU_ID_THEME_FOLLOW_SYSTEM => {
            if let Err(error) = set_theme_mode(
                app.clone(),
                app.state::<AppState>(),
                ThemeMode::FollowSystem,
            ) {
                log::error!("{error}");
            }
        }
        MENU_ID_THEME_WHITE => {
            if let Err(error) =
                set_theme_mode(app.clone(), app.state::<AppState>(), ThemeMode::ThemeWhite)
            {
                log::error!("{error}");
            }
        }
        MENU_ID_THEME_DARK => {
            if let Err(error) =
                set_theme_mode(app.clone(), app.state::<AppState>(), ThemeMode::ThemeDark)
            {
                log::error!("{error}");
            }
        }
        MENU_ID_THEME_CUSTOM => {
            if let Err(error) =
                set_theme_mode(app.clone(), app.state::<AppState>(), ThemeMode::Custom)
            {
                log::error!("{error}");
            }
        }
        MENU_ID_LANGUAGE_ENGLISH => {
            if let Err(error) =
                set_language_mode(app.clone(), app.state::<AppState>(), LanguageMode::English)
            {
                log::error!("{error}");
            }
        }
        MENU_ID_LANGUAGE_CHINESE => {
            if let Err(error) =
                set_language_mode(app.clone(), app.state::<AppState>(), LanguageMode::Chinese)
            {
                log::error!("{error}");
            }
        }
        MENU_ID_QUIT => {
            let state = app.state::<AppState>();
            if let Err(error) = flush_draft_inner(app, &state) {
                log::error!("failed to flush draft before exit: {error}");
            }
            app.exit(0);
        }
        _ => {}
    }
}

fn create_tray(app: &AppHandle, config: &AppConfig) -> Result<TrayMenuItems, String> {
    let show_item = MenuItem::with_id(app, MENU_ID_SHOW, "Show", true, None::<&str>)
        .map_err(|error| format!("failed to create Show menu item: {error}"))?;
    let settings_item = MenuItem::with_id(app, MENU_ID_SETTINGS, "Settings", true, None::<&str>)
        .map_err(|error| format!("failed to create Settings menu item: {error}"))?;
    let follow_system_item = CheckMenuItem::with_id(
        app,
        MENU_ID_THEME_FOLLOW_SYSTEM,
        "Follow System",
        true,
        config.theme_mode == ThemeMode::FollowSystem,
        None::<&str>,
    )
    .map_err(|error| format!("failed to create Follow System menu item: {error}"))?;
    let theme_white_item = CheckMenuItem::with_id(
        app,
        MENU_ID_THEME_WHITE,
        "Theme White",
        true,
        config.theme_mode == ThemeMode::ThemeWhite,
        None::<&str>,
    )
    .map_err(|error| format!("failed to create Theme White menu item: {error}"))?;
    let theme_dark_item = CheckMenuItem::with_id(
        app,
        MENU_ID_THEME_DARK,
        "Theme Dark",
        true,
        config.theme_mode == ThemeMode::ThemeDark,
        None::<&str>,
    )
    .map_err(|error| format!("failed to create Theme Dark menu item: {error}"))?;
    let theme_custom_item = CheckMenuItem::with_id(
        app,
        MENU_ID_THEME_CUSTOM,
        "Custom",
        true,
        config.theme_mode == ThemeMode::Custom,
        None::<&str>,
    )
    .map_err(|error| format!("failed to create Custom theme menu item: {error}"))?;
    let language_english_item = CheckMenuItem::with_id(
        app,
        MENU_ID_LANGUAGE_ENGLISH,
        "English",
        true,
        config.language_mode == LanguageMode::English,
        None::<&str>,
    )
    .map_err(|error| format!("failed to create English language menu item: {error}"))?;
    let language_chinese_item = CheckMenuItem::with_id(
        app,
        MENU_ID_LANGUAGE_CHINESE,
        "中文",
        true,
        config.language_mode == LanguageMode::Chinese,
        None::<&str>,
    )
    .map_err(|error| format!("failed to create Chinese language menu item: {error}"))?;
    let separator = PredefinedMenuItem::separator(app)
        .map_err(|error| format!("failed to create separator menu item: {error}"))?;
    let quit_item = MenuItem::with_id(app, MENU_ID_QUIT, "Quit", true, None::<&str>)
        .map_err(|error| format!("failed to create Quit menu item: {error}"))?;

    let theme_submenu = Submenu::with_items(
        app,
        "Theme",
        true,
        &[
            &follow_system_item,
            &theme_white_item,
            &theme_dark_item,
            &theme_custom_item,
        ],
    )
    .map_err(|error| format!("failed to create tray theme submenu: {error}"))?;

    let language_submenu = Submenu::with_items(
        app,
        "Language / 语言",
        true,
        &[&language_english_item, &language_chinese_item],
    )
    .map_err(|error| format!("failed to create tray language submenu: {error}"))?;

    let tray_menu = Menu::with_items(
        app,
        &[
            &show_item,
            &settings_item,
            &theme_submenu,
            &language_submenu,
            &separator,
            &quit_item,
        ],
    )
    .map_err(|error| format!("failed to create tray menu: {error}"))?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| String::from("default window icon is missing"))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("Floating Input Tool")
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                dispatch_window_action(tray.app_handle(), show_main_window);
            }
        })
        .build(app)
        .map_err(|error| format!("failed to create tray icon: {error}"))?;

    Ok(TrayMenuItems {
        theme: ThemeMenuItems {
            follow_system: follow_system_item,
            theme_white: theme_white_item,
            theme_dark: theme_dark_item,
            theme_custom: theme_custom_item,
        },
        language: LanguageMenuItems {
            english: language_english_item,
            chinese: language_chinese_item,
        },
    })
}

fn normalize_target_file_path(target_file_path: &str) -> Result<String, String> {
    let normalized = target_file_path.trim();
    if normalized.is_empty() {
        return Err(String::from("target file path is required"));
    }

    if normalized.to_ascii_lowercase().ends_with(".md") {
        return Ok(normalized.to_string());
    }

    Ok(format!("{normalized}.md"))
}

fn register_startup_hotkeys<R: Runtime>(app: &AppHandle<R>, config: &AppConfig) -> ActiveHotkeys {
    ActiveHotkeys {
        capture: register_startup_hotkey(
            app,
            GlobalHotkeyAction::Capture,
            &config.hotkey,
            Some(DEFAULT_HOTKEY),
        ),
        hide: register_startup_hotkey(
            app,
            GlobalHotkeyAction::Hide,
            &config.hide_hotkey,
            Some(DEFAULT_HIDE_HOTKEY),
        ),
        next_target: register_startup_hotkey(
            app,
            GlobalHotkeyAction::NextTarget,
            &config.next_target_hotkey,
            Some(DEFAULT_NEXT_TARGET_HOTKEY),
        ),
    }
}

fn repair_startup_hotkey_config(
    config: &mut AppConfig,
    active_hotkeys: &ActiveHotkeys,
) -> Vec<String> {
    let mut warnings = Vec::new();
    for action in [
        GlobalHotkeyAction::Capture,
        GlobalHotkeyAction::Hide,
        GlobalHotkeyAction::NextTarget,
    ] {
        let configured = configured_hotkey(config, action).to_string();
        match active_hotkeys.get(action) {
            Some(active) if active.accelerator != configured => {
                warnings.push(format!(
                    "{} shortcut '{}' was unavailable. '{}' is active instead.",
                    hotkey_action_label(action),
                    configured,
                    active.accelerator
                ));
                set_configured_hotkey(config, action, active.accelerator.clone());
            }
            None => warnings.push(format!(
                "No {} shortcut could be registered at startup.",
                hotkey_action_label(action)
            )),
            _ => {}
        }
    }
    warnings
}

fn configured_hotkey(config: &AppConfig, action: GlobalHotkeyAction) -> &str {
    match action {
        GlobalHotkeyAction::Capture => &config.hotkey,
        GlobalHotkeyAction::Hide => &config.hide_hotkey,
        GlobalHotkeyAction::NextTarget => &config.next_target_hotkey,
    }
}

fn register_startup_hotkey<R: Runtime>(
    app: &AppHandle<R>,
    action: GlobalHotkeyAction,
    configured_hotkey: &str,
    fallback_hotkey: Option<&str>,
) -> Option<RegisteredHotkey> {
    let normalized_hotkey = configured_hotkey.trim();
    if !normalized_hotkey.is_empty() {
        match app.global_shortcut().register(normalized_hotkey) {
            Ok(()) => return RegisteredHotkey::from_accelerator(normalized_hotkey).ok(),
            Err(error) => {
                log::warn!(
                    "failed to register configured {} hotkey {normalized_hotkey}: {error}",
                    hotkey_action_label(action)
                );
            }
        }
    }

    let Some(fallback_hotkey) = fallback_hotkey else {
        return None;
    };

    if normalized_hotkey.eq_ignore_ascii_case(fallback_hotkey) {
        return None;
    }

    match app.global_shortcut().register(fallback_hotkey) {
        Ok(()) => RegisteredHotkey::from_accelerator(fallback_hotkey).ok(),
        Err(error) => {
            log::error!(
                "failed to register fallback {} hotkey {fallback_hotkey}: {error}",
                hotkey_action_label(action)
            );
            None
        }
    }
}

fn hotkey_action_label(action: GlobalHotkeyAction) -> &'static str {
    match action {
        GlobalHotkeyAction::Capture => "Capture",
        GlobalHotkeyAction::Hide => "Hide",
        GlobalHotkeyAction::NextTarget => "Next target",
    }
}

fn sync_main_window_mode_after_move<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let state = app.state::<AppState>();
    let transition = begin_window_transition(&state)?;
    let suppressed_until = *state
        .auto_dock_suppressed_until_ms
        .lock()
        .map_err(|error| format!("failed to lock auto dock suppression state: {error}"))?;
    let now_ms = current_time_millis();
    if suppressed_until > now_ms {
        log::info!(
      "[side-hide] sync_main_window_mode_after_move skipped due to suppression now_ms={} suppressed_until={}",
      now_ms,
      suppressed_until
    );
        return Ok(());
    }

    let is_dragging = *state
        .manual_drag_in_progress
        .lock()
        .map_err(|error| format!("failed to lock manual drag state: {error}"))?;
    if is_dragging {
        return Ok(());
    }

    let config = state
        .config
        .lock()
        .map_err(|error| format!("failed to lock app config: {error}"))?
        .clone();

    if config.side_hide_enabled == 0 {
        return Ok(());
    }

    let window = app
        .get_webview_window(WINDOW_LABEL_MAIN)
        .ok_or_else(|| String::from("main window is missing"))?;
    let current_bounds = capture_window_bounds(&window)?;
    let current_mode = state
        .main_window_state
        .lock()
        .map_err(|error| format!("failed to lock main window state: {error}"))?
        .mode;

    match current_mode {
        MainWindowMode::DockedLeft | MainWindowMode::DockedRight => Ok(()),
        MainWindowMode::Normal => {
            if let Some((dock_side, docked_bounds)) =
                compute_docked_bounds(&window, &current_bounds, &config)?
            {
                suppress_auto_dock_for(&state, WINDOW_BOUNDS_ANIMATION_MS + AUTO_DOCK_SETTLE_MS)?;
                apply_window_bounds_animated(&window, current_bounds, docked_bounds, &transition)?;
                set_window_resizable(&window, false)?;
                settle_window_bounds_animation();
                update_main_window_runtime_state(
                    app,
                    &state,
                    MainWindowRuntimeState {
                        mode: dock_side.to_docked_mode(),
                        dock_side: Some(dock_side),
                        normal_bounds: Some(current_bounds),
                        docked_bounds: Some(docked_bounds),
                    },
                )?;
            }

            Ok(())
        }
        MainWindowMode::ExpandedFromDock => {
            if let Some((dock_side, docked_bounds)) =
                compute_docked_bounds(&window, &current_bounds, &config)?
            {
                suppress_auto_dock_for(&state, WINDOW_BOUNDS_ANIMATION_MS + AUTO_DOCK_SETTLE_MS)?;
                apply_window_bounds_animated(&window, current_bounds, docked_bounds, &transition)?;
                set_window_resizable(&window, false)?;
                settle_window_bounds_animation();
                update_main_window_runtime_state(
                    app,
                    &state,
                    MainWindowRuntimeState {
                        mode: dock_side.to_docked_mode(),
                        dock_side: Some(dock_side),
                        normal_bounds: Some(current_bounds),
                        docked_bounds: Some(docked_bounds),
                    },
                )?;
            } else {
                set_window_resizable(&window, true)?;
                update_main_window_runtime_state(
                    app,
                    &state,
                    MainWindowRuntimeState {
                        mode: MainWindowMode::Normal,
                        dock_side: None,
                        normal_bounds: Some(current_bounds),
                        docked_bounds: None,
                    },
                )?;
            }

            Ok(())
        }
    }
}

#[derive(Default)]
struct AutoDockSchedule {
    deadline: Option<Instant>,
    running: bool,
}

fn schedule_main_window_auto_dock<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    let Ok(mut schedule) = state.auto_dock_schedule.lock() else {
        return;
    };
    schedule.deadline = Some(Instant::now() + Duration::from_millis(AUTO_DOCK_SETTLE_MS));
    if schedule.running {
        return;
    }
    schedule.running = true;
    drop(schedule);
    let app = app.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(16));
        let state = app.state::<AppState>();
        let Ok(mut schedule) = state.auto_dock_schedule.lock() else {
            return;
        };
        if schedule
            .deadline
            .is_some_and(|deadline| Instant::now() < deadline)
            || is_left_mouse_button_down()
        {
            continue;
        }
        schedule.deadline = None;
        drop(schedule);
        match window_behavior::finish_manual_drag(&state.manual_drag_in_progress) {
            Ok(true) => {
                if let Err(error) = sync_main_window_mode_after_move(&app) {
                    log::warn!("{error}");
                }
            }
            Ok(false) => {}
            Err(error) => log::warn!("{error}"),
        }
        let Ok(mut schedule) = state.auto_dock_schedule.lock() else {
            return;
        };
        if schedule.deadline.is_none() {
            schedule.running = false;
            return;
        }
    });
}

struct WindowTransition<'a> {
    state: &'a AppState,
    generation: u64,
    _lock: MutexGuard<'a, ()>,
}

fn begin_window_transition(state: &AppState) -> Result<WindowTransition<'_>, String> {
    let generation = state.transition_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let lock = state
        .transition_lock
        .lock()
        .map_err(|error| error.to_string())?;
    if state.transition_generation.load(Ordering::SeqCst) != generation {
        return Err("window transition superseded".into());
    }
    Ok(WindowTransition {
        state,
        generation,
        _lock: lock,
    })
}

impl WindowTransition<'_> {
    fn check(&self) -> Result<(), String> {
        if self.state.transition_generation.load(Ordering::SeqCst) == self.generation {
            Ok(())
        } else {
            Err("window transition superseded".into())
        }
    }
}

#[cfg(windows)]
fn is_left_mouse_button_down() -> bool {
    unsafe { (GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000) != 0 }
}

#[cfg(not(windows))]
fn is_left_mouse_button_down() -> bool {
    false
}

#[cfg(windows)]
fn current_cursor_position() -> Option<(i32, i32)> {
    let mut point = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut point) };
    if ok == 0 {
        None
    } else {
        Some((point.x, point.y))
    }
}

#[cfg(not(windows))]
fn current_cursor_position() -> Option<(i32, i32)> {
    None
}

fn capture_window_bounds<R: Runtime>(
    window: &WebviewWindow<R>,
) -> Result<StoredWindowBounds, String> {
    let position = window
        .outer_position()
        .map_err(|error| format!("failed to read window position: {error}"))?;
    let size = window
        .outer_size()
        .map_err(|error| format!("failed to read window size: {error}"))?;

    Ok(StoredWindowBounds {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    })
}

fn apply_window_bounds<R: Runtime>(
    window: &WebviewWindow<R>,
    bounds: StoredWindowBounds,
) -> Result<(), String> {
    window
        .set_size(PhysicalSize::new(bounds.width, bounds.height))
        .map_err(|error| format!("failed to set window size: {error}"))?;
    window
        .set_position(PhysicalPosition::new(bounds.x, bounds.y))
        .map_err(|error| format!("failed to set window position: {error}"))
}

fn apply_window_bounds_animated<R: Runtime>(
    window: &WebviewWindow<R>,
    from: StoredWindowBounds,
    to: StoredWindowBounds,
    transition: &WindowTransition<'_>,
) -> Result<(), String> {
    if transition.check().is_err() {
        return apply_window_bounds(window, to);
    }
    if from.x == to.x && from.y == to.y && from.width == to.width && from.height == to.height {
        return apply_window_bounds(window, to);
    }

    let step_duration = Duration::from_millis(
        (WINDOW_BOUNDS_ANIMATION_MS / WINDOW_BOUNDS_ANIMATION_STEPS as u64).max(1),
    );

    for step in 1..=WINDOW_BOUNDS_ANIMATION_STEPS {
        if transition.check().is_err() {
            return apply_window_bounds(window, to);
        }
        let progress = step as f64 / WINDOW_BOUNDS_ANIMATION_STEPS as f64;
        let eased = 1.0 - (1.0 - progress).powi(3);
        let next_bounds = StoredWindowBounds {
            x: lerp_i32(from.x, to.x, eased),
            y: lerp_i32(from.y, to.y, eased),
            width: lerp_u32(from.width, to.width, eased),
            height: lerp_u32(from.height, to.height, eased),
        };

        apply_window_bounds(window, next_bounds)?;

        if step < WINDOW_BOUNDS_ANIMATION_STEPS {
            thread::sleep(step_duration);
        }
    }

    Ok(())
}

fn lerp_i32(from: i32, to: i32, progress: f64) -> i32 {
    (from as f64 + (to - from) as f64 * progress).round() as i32
}

fn lerp_u32(from: u32, to: u32, progress: f64) -> u32 {
    (from as f64 + (to as f64 - from as f64) * progress)
        .round()
        .max(1.0) as u32
}

fn suppress_auto_dock_for(state: &State<'_, AppState>, duration_ms: u64) -> Result<(), String> {
    let mut suppressed_until = state
        .auto_dock_suppressed_until_ms
        .lock()
        .map_err(|error| format!("failed to lock auto dock suppression state: {error}"))?;
    *suppressed_until = current_time_millis() + duration_ms as u128;
    Ok(())
}

fn settle_window_bounds_animation() {
    thread::sleep(Duration::from_millis(
        WINDOW_BOUNDS_ANIMATION_PAINT_SETTLE_MS,
    ));
}

fn set_window_resizable<R: Runtime>(
    window: &WebviewWindow<R>,
    is_resizable: bool,
) -> Result<(), String> {
    window
        .set_resizable(is_resizable)
        .map_err(|error| format!("failed to set window resizable state: {error}"))
}

fn compute_docked_bounds<R: Runtime>(
    window: &WebviewWindow<R>,
    current_bounds: &StoredWindowBounds,
    config: &AppConfig,
) -> Result<Option<(DockSide, StoredWindowBounds)>, String> {
    let Some(monitor) = window
        .current_monitor()
        .map_err(|error| format!("failed to resolve current monitor: {error}"))?
    else {
        return Ok(None);
    };

    let work_area = monitor.work_area();
    let work_left = work_area.position.x;
    let work_top = work_area.position.y;
    let work_right = work_left + work_area.size.width as i32;
    let work_bottom = work_top + work_area.size.height as i32;
    let threshold =
        logical_px_to_physical(config.edge_snap_threshold_px, monitor.scale_factor()) as i32;

    let inset = logical_px_to_physical(surface::SHADOW_INSET, monitor.scale_factor()) as i32;
    let content = surface::visible_bounds(*current_bounds, inset);
    let left_gap = content.x - work_left;
    let right_gap = work_right - (content.x + content.width as i32);
    let near_left = left_gap <= threshold;
    let near_right = right_gap <= threshold;

    if !near_left && !near_right {
        return Ok(None);
    }

    let dock_side = match (near_left, near_right) {
        (true, true) => {
            if left_gap.abs() <= right_gap.abs() {
                DockSide::Left
            } else {
                DockSide::Right
            }
        }
        (true, false) => DockSide::Left,
        (false, true) => DockSide::Right,
        (false, false) => return Ok(None),
    };

    let exposed_width =
        logical_px_to_physical(effective_exposed_width(config), monitor.scale_factor())
            .min(current_bounds.width)
            .max(1) as i32;
    let min_y = work_top - inset;
    let max_y = (work_bottom - current_bounds.height as i32 + inset).max(min_y);
    let clamped_y = current_bounds.y.clamp(min_y, max_y);

    let dock_x = match dock_side {
        DockSide::Left => work_left - (current_bounds.width as i32 - exposed_width),
        DockSide::Right => work_right - exposed_width,
    };

    Ok(Some((
        dock_side,
        StoredWindowBounds {
            x: dock_x,
            y: clamped_y,
            width: current_bounds.width,
            height: current_bounds.height,
        },
    )))
}

fn compute_restore_bounds<R: Runtime>(
    window: &WebviewWindow<R>,
    saved_bounds: StoredWindowBounds,
    dock_side: Option<DockSide>,
) -> Result<StoredWindowBounds, String> {
    let Some(monitor) = window
        .monitor_from_point(
            saved_bounds.x as f64 + saved_bounds.width as f64 / 2.0,
            saved_bounds.y as f64 + saved_bounds.height as f64 / 2.0,
        )
        .map_err(|error| format!("failed to resolve current monitor: {error}"))?
    else {
        return Ok(saved_bounds);
    };

    let work_area = monitor.work_area();
    let work_left = work_area.position.x;
    let work_top = work_area.position.y;
    let work_right = work_left + work_area.size.width as i32;
    let work_bottom = work_top + work_area.size.height as i32;
    let inset = logical_px_to_physical(surface::SHADOW_INSET, monitor.scale_factor()) as i32;
    let min_y = work_top - inset;
    let max_y = (work_bottom - saved_bounds.height as i32 + inset).max(min_y);
    let y = saved_bounds.y.clamp(min_y, max_y);
    let x = surface::restore_x(dock_side, saved_bounds, work_left, work_right, inset);

    Ok(StoredWindowBounds {
        x,
        y,
        width: saved_bounds.width,
        height: saved_bounds.height,
    })
}

fn effective_exposed_width(config: &AppConfig) -> u32 {
    config.visible_handle_width_px.max(1)
}

fn restore_main_window_to_normal<R: Runtime>(
    app: &AppHandle<R>,
    state: &State<'_, AppState>,
) -> Result<(), String> {
    let runtime_state = state
        .main_window_state
        .lock()
        .map_err(|error| format!("failed to lock main window state: {error}"))?
        .clone();

    let window = app
        .get_webview_window(WINDOW_LABEL_MAIN)
        .ok_or_else(|| String::from("main window is missing"))?;

    set_window_resizable(&window, MainWindowMode::Normal.is_resizable())?;
    if runtime_state.mode == MainWindowMode::Normal {
        return Ok(());
    }
    if let Some(normal_bounds) = runtime_state.normal_bounds {
        let normal_bounds =
            compute_restore_bounds(&window, normal_bounds, runtime_state.dock_side)?;
        apply_window_bounds(&window, normal_bounds)?;
        update_main_window_runtime_state(
            app,
            state,
            MainWindowRuntimeState {
                mode: MainWindowMode::Normal,
                dock_side: None,
                normal_bounds: Some(normal_bounds),
                docked_bounds: None,
            },
        )?;
    }

    Ok(())
}

fn update_main_window_runtime_state<R: Runtime>(
    app: &AppHandle<R>,
    state: &State<'_, AppState>,
    next_state: MainWindowRuntimeState,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL_MAIN) {
        apply_native_corner_preference(
            &window,
            !matches!(
                next_state.mode,
                MainWindowMode::DockedLeft | MainWindowMode::DockedRight
            ),
        );
    }
    {
        let mut runtime_state = state
            .main_window_state
            .lock()
            .map_err(|error| format!("failed to lock main window state: {error}"))?;
        *runtime_state = next_state.clone();
    }

    surface_runtime::refresh(app, None);
    app.emit(
        EVENT_MAIN_WINDOW_MODE_CHANGED,
        MainWindowModeChangedPayload {
            mode: next_state.mode,
            dock_side: next_state.dock_side,
        },
    )
    .map_err(|error| format!("failed to emit main window mode change: {error}"))
}

fn sanitize_note_template(note_template: &str) -> String {
    let normalized = note_template.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        DEFAULT_NOTE_TEMPLATE.into()
    } else {
        trimmed.into()
    }
}

fn format_note_block(note_text: &str, note_template: &str) -> String {
    let normalized_text = note_text.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed_text = normalized_text.trim_end_matches('\n');
    let timestamp = current_timestamp_string();
    let template = sanitize_note_template(note_template);
    let callout_text = format_callout_text(trimmed_text);

    let replacements = [
        ("{{timestamp}}", timestamp.as_str()),
        ("{{text.callout}}", callout_text.as_str()),
        ("{{text}}", trimmed_text),
    ];
    let mut rendered = String::new();
    let mut remaining = template.as_str();
    while !remaining.is_empty() {
        if let Some((placeholder, value)) = replacements
            .iter()
            .find(|(placeholder, _)| remaining.starts_with(placeholder))
        {
            rendered.push_str(value);
            remaining = &remaining[placeholder.len()..];
        } else {
            let next = remaining.chars().next().unwrap();
            rendered.push(next);
            remaining = &remaining[next.len_utf8()..];
        }
    }

    normalize_note_block_line_endings(&rendered)
}

fn format_callout_text(note_text: &str) -> String {
    let mut callout_text = String::new();
    for line in note_text.split('\n') {
        if line.is_empty() {
            callout_text.push('>');
        } else {
            callout_text.push_str("> ");
            callout_text.push_str(line);
        }
        callout_text.push('\n');
    }
    callout_text.trim_end_matches('\n').into()
}

fn normalize_note_block_line_endings(note_block: &str) -> String {
    let normalized = note_block.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized.trim_matches('\n');
    format!("{}\r\n\r\n", trimmed.replace('\n', "\r\n"))
}

fn prepend_to_file(target_file_path: &str, note_block: &str) -> Result<(), String> {
    let started_at = Instant::now();
    ensure_destination_exists(target_file_path)?;

    let existing_content = if Path::new(target_file_path).exists() {
        fs::read_to_string(target_file_path)
            .map_err(|error| format!("failed to read target file {target_file_path}: {error}"))?
    } else {
        String::new()
    };

    let mut output = String::from(note_block);
    if !existing_content.is_empty() {
        output.push_str(&existing_content);
    }

    atomic_write_string(Path::new(target_file_path), &output)
        .map_err(|error| format!("failed to write target file {target_file_path}: {error}"))?;
    log::info!(
        "prepended note block to {} existing bytes in {} ms",
        existing_content.len(),
        started_at.elapsed().as_millis()
    );
    Ok(())
}

fn ensure_destination_exists(target_file_path: &str) -> Result<(), String> {
    let target_path = Path::new(target_file_path);
    if let Some(parent) = target_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("failed to create directory {}: {error}", parent.display())
            })?;
        }
    }

    if !target_path.exists() {
        atomic_write_string(target_path, "").map_err(|error| {
            format!(
                "failed to create target file {}: {error}",
                target_path.display()
            )
        })?;
    }

    Ok(())
}

fn current_timestamp_string() -> String {
    #[allow(deprecated)]
    let now = std::time::SystemTime::now();
    let datetime: chrono::DateTime<chrono::Local> = now.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

fn current_time_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            dispatch_window_action(app, show_main_window);
        }))
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
                .max_file_size(2_000_000)
                .build(),
        )
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        handle_global_hotkey(app, shortcut);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let mut config = match load_app_config(app.handle()) {
                Ok(config) => config,
                Err(error) => {
                    log::warn!("{error}");
                    default_app_config(app.handle())
                }
            };

            let active_hotkeys = register_startup_hotkeys(app.handle(), &config);
            let runtime_warnings = repair_startup_hotkey_config(&mut config, &active_hotkeys);
            if !runtime_warnings.is_empty() {
                save_app_config(app.handle(), &config)?;
                for warning in &runtime_warnings {
                    log::warn!("{warning}");
                }
            }
            let tray_menu_items = create_tray(app.handle(), &config)?;
            let draft = match load_draft(app.handle()) {
                Ok(draft) => draft,
                Err(error) => {
                    log::warn!("{error}");
                    DraftSnapshot::default()
                }
            };

            app.manage(AppState {
                composing: AtomicBool::new(false),
                saving: AtomicBool::new(false),
                config: Mutex::new(config),
                tray_menu_items,
                active_hotkeys: Mutex::new(active_hotkeys),
                hotkey_update_lock: Mutex::new(()),
                runtime_warnings: Mutex::new(runtime_warnings),
                draft: Mutex::new(draft),
                draft_save_generation: Mutex::new(0),
                native_backdrop: Mutex::new(NativeBackdropState::default()),
                main_window_state: Mutex::new(MainWindowRuntimeState::default()),
                auto_dock_schedule: Mutex::new(AutoDockSchedule::default()),
                transition_lock: Mutex::new(()),
                transition_generation: AtomicU64::new(0),
                manual_drag_in_progress: Mutex::new(false),
                auto_dock_suppressed_until_ms: Mutex::new(0),
            });

            log::info!(
                "FloatingInputTool {} started with process id {}",
                app.package_info().version,
                std::process::id()
            );

            if let Some(main_window) = app.get_webview_window(WINDOW_LABEL_MAIN) {
                attach_main_window_events(app.handle(), &main_window);
                update_native_backdrop(app.handle(), &main_window);
            }

            native_dock::start(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            native_dock::set_capture_interaction,
            update_main_surface,
            get_app_config,
            get_hotkey_runtime_status,
            get_draft,
            get_native_backdrop_status,
            update_draft,
            flush_draft,
            set_theme_mode,
            set_language_mode,
            set_target_file_path,
            set_active_target_id,
            set_markdown_targets,
            set_hotkey,
            set_hide_hotkey,
            set_next_target_hotkey,
            set_save_shortcut_mode,
            set_note_template,
            set_custom_theme,
            save_note,
            open_settings_window,
            close_settings_window,
            finish_capture,
            begin_manual_window_drag,
            end_manual_window_drag,
            is_cursor_inside_main_window,
            hide_or_dock_main_window,
            restore_docked_main_window,
            redock_main_window
        ])
        .on_menu_event(handle_menu_event)
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|app_handle, event| {
        if matches!(event, RunEvent::Exit) {
            surface_runtime::clear();
        }
        if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
            let state = app_handle.state::<AppState>();
            if let Err(error) = flush_draft_inner(app_handle, &state) {
                log::error!("failed to flush draft during application exit: {error}");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{format_note_block, prepend_to_file};
    use std::fs;

    #[test]
    fn inserted_text_is_not_interpolated_again() {
        assert_eq!(
            format_note_block("literal {{text}}", "{{text.callout}}"),
            "> literal {{text}}\r\n\r\n"
        );
        assert_eq!(
            format_note_block(
                "{{timestamp}} {{text.callout}} {{text}}",
                "{{text}} / {{unknown}}"
            ),
            "{{timestamp}} {{text.callout}} {{text}} / {{unknown}}\r\n\r\n"
        );
    }

    #[test]
    fn formats_multiline_callout_content() {
        let template = "> [!fleeting]+ {{timestamp}}\n>\n{{text.callout}}";
        let block = format_note_block("first\n\nthird", template);

        assert!(block.contains("> first\r\n>\r\n> third\r\n"));
        assert!(block.ends_with("\r\n\r\n"));
    }

    #[test]
    fn prepend_keeps_existing_markdown_content() {
        let directory = std::env::temp_dir().join(format!(
            "floating-input-tool-note-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("notes.md");
        fs::write(&path, "existing\r\n").unwrap();

        prepend_to_file(path.to_str().unwrap(), "new\r\n\r\n").unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "new\r\n\r\nexisting\r\n"
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
