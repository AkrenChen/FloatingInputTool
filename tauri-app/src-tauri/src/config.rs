use crate::storage::atomic_write_string;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager, Runtime};

const CONFIG_FILE_NAME: &str = "config.json";
const CONFIG_HELP_FILE_NAME: &str = "config.help.md";
const DEFAULT_TARGET_FILE_NAME: &str = "Fleeting Note.md";
pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+Space";
pub const DEFAULT_HIDE_HOTKEY: &str = "Ctrl+Alt+H";
pub const DEFAULT_NEXT_TARGET_HOTKEY: &str = "Ctrl+Alt+Right";
pub const DEFAULT_SIDE_HIDE_ENABLED: u8 = 1;
pub const DEFAULT_EDGE_SNAP_THRESHOLD_PX: u32 = 36;
pub const DEFAULT_VISIBLE_HANDLE_WIDTH_PX: u32 = 22;
pub const DEFAULT_HOVER_OPEN_DELAY_MS: u64 = 180;
pub const DEFAULT_HOVER_CLOSE_DELAY_MS: u64 = 100;
pub const DEFAULT_HOTZONE_WIDTH_PX: u32 = 36;
pub const DEFAULT_DEBUG_SHOW_HOTZONE: u8 = 0;
pub const DEFAULT_SAVE_SHORTCUT_MODE: SaveShortcutMode = SaveShortcutMode::CtrlEnterSave;
pub const DEFAULT_EMPTY_INPUT_PLACEHOLDER_COLOR: &str = "#8A8A8A";
pub const DEFAULT_SAVE_SHORTCUT_TEXT_COLOR: &str = "#333333";
pub const DEFAULT_SAVE_SHORTCUT_FONT_SIZE_PX: u32 = 9;
pub const DEFAULT_CUSTOM_WINDOW_COLOR: &str = "#F8F8FF";
pub const DEFAULT_CUSTOM_WINDOW_OPACITY: f64 = 0.86;
pub const DEFAULT_CUSTOM_TEXT_COLOR: &str = "#333333";
pub const DEFAULT_CUSTOM_ACCENT_COLOR: &str = "#3EB4BF";
pub const DEFAULT_TARGET_ID: &str = "default";
pub const DEFAULT_TARGET_NICKNAME: &str = "Fleeting";
pub const DEFAULT_NOTE_TEMPLATE: &str = "> [!fleeting]+ {{timestamp}}\n>\n{{text.callout}}";
pub const DEFAULT_LANGUAGE_MODE: LanguageMode = LanguageMode::English;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeMode {
    #[default]
    FollowSystem,
    ThemeWhite,
    ThemeDark,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SaveShortcutMode {
    #[default]
    CtrlEnterSave,
    EnterSave,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum LanguageMode {
    #[default]
    English,
    Chinese,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomTheme {
    #[serde(default = "default_custom_window_color")]
    pub window_color: String,
    #[serde(default = "default_custom_window_opacity")]
    pub window_opacity: f64,
    #[serde(default = "default_custom_text_color")]
    pub text_color: String,
    #[serde(default = "default_custom_accent_color")]
    pub accent_color: String,
}

impl Default for CustomTheme {
    fn default() -> Self {
        Self {
            window_color: DEFAULT_CUSTOM_WINDOW_COLOR.into(),
            window_opacity: DEFAULT_CUSTOM_WINDOW_OPACITY,
            text_color: DEFAULT_CUSTOM_TEXT_COLOR.into(),
            accent_color: DEFAULT_CUSTOM_ACCENT_COLOR.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownTarget {
    #[serde(default = "default_target_id")]
    pub id: String,
    #[serde(default = "default_target_nickname")]
    pub nickname: String,
    #[serde(default)]
    pub path: String,
}

impl Default for MarkdownTarget {
    fn default() -> Self {
        Self {
            id: DEFAULT_TARGET_ID.into(),
            nickname: DEFAULT_TARGET_NICKNAME.into(),
            path: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct EditorTypography {
    pub font_family: String,
    pub font_size_px: u32,
    pub bold: bool,
    pub italic: bool,
    pub custom_fonts: Vec<String>,
}

impl Default for EditorTypography {
    fn default() -> Self {
        Self {
            font_family: String::new(),
            font_size_px: 15,
            bold: false,
            italic: false,
            custom_fonts: Vec::new(),
        }
    }
}

pub fn sanitize_editor_typography(mut value: EditorTypography) -> EditorTypography {
    value.font_family = value.font_family.trim().to_string();
    value.font_size_px = value.font_size_px.clamp(10, 48);
    let mut fonts: Vec<String> = Vec::new();
    for font in value.custom_fonts {
        let font = font.trim();
        if !font.is_empty() && !fonts.iter().any(|item| item.eq_ignore_ascii_case(font)) {
            fonts.push(font.to_string());
        }
    }
    value.custom_fonts = fonts;
    value
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default)]
    pub target_file_path: String,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default = "default_hide_hotkey")]
    pub hide_hotkey: String,
    #[serde(default = "default_next_target_hotkey")]
    pub next_target_hotkey: String,
    #[serde(default = "default_side_hide_enabled")]
    pub side_hide_enabled: u8,
    #[serde(default = "default_edge_snap_threshold_px")]
    pub edge_snap_threshold_px: u32,
    #[serde(default = "default_visible_handle_width_px")]
    pub visible_handle_width_px: u32,
    #[serde(default = "default_hover_open_delay_ms")]
    pub hover_open_delay_ms: u64,
    #[serde(default = "default_hover_close_delay_ms")]
    pub hover_close_delay_ms: u64,
    #[serde(default = "default_hotzone_width_px")]
    pub hotzone_width_px: u32,
    #[serde(default = "default_debug_show_hotzone")]
    pub debug_show_hotzone: u8,
    #[serde(default = "default_save_shortcut_mode")]
    pub save_shortcut_mode: SaveShortcutMode,
    #[serde(default = "default_empty_input_placeholder_color")]
    pub empty_input_placeholder_color: String,
    #[serde(default = "default_save_shortcut_text_color")]
    pub save_shortcut_text_color: String,
    #[serde(default = "default_save_shortcut_font_size_px")]
    pub save_shortcut_font_size_px: u32,
    #[serde(default)]
    pub custom_theme: CustomTheme,
    #[serde(default)]
    pub editor_typography: EditorTypography,
    #[serde(default)]
    pub targets: Vec<MarkdownTarget>,
    #[serde(default = "default_target_id")]
    pub active_target_id: String,
    #[serde(default = "default_note_template")]
    pub note_template: String,
    #[serde(default = "default_language_mode")]
    pub language_mode: LanguageMode,
    pub theme_mode: ThemeMode,
}

pub fn load_app_config<R: Runtime>(app: &AppHandle<R>) -> Result<AppConfig, String> {
    ensure_config_help_file(app)?;

    let config_path = config_file_path(app)?;
    log::info!(
        "loading app config from {} (exists={})",
        config_path.display(),
        config_path.exists()
    );
    if !config_path.exists() {
        return Ok(default_app_config(app));
    }

    let raw = fs::read_to_string(&config_path).map_err(|error| {
        format!(
            "failed to read config from {}: {error}",
            config_path.display()
        )
    })?;

    let mut config: AppConfig = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "failed to parse config from {}: {error}",
            config_path.display()
        )
    })?;

    if config.target_file_path.trim().is_empty() {
        config.target_file_path = default_target_file_path(app)?;
    }
    normalize_targets(app, &mut config)?;
    if config.hotkey.trim().is_empty() {
        config.hotkey = DEFAULT_HOTKEY.into();
    }
    if config.hide_hotkey.trim().is_empty() {
        config.hide_hotkey = DEFAULT_HIDE_HOTKEY.into();
    }
    if config.next_target_hotkey.trim().is_empty() {
        config.next_target_hotkey = DEFAULT_NEXT_TARGET_HOTKEY.into();
    }
    if config.edge_snap_threshold_px == 0 {
        config.edge_snap_threshold_px = DEFAULT_EDGE_SNAP_THRESHOLD_PX;
    }
    if config.visible_handle_width_px == 0 {
        config.visible_handle_width_px = DEFAULT_VISIBLE_HANDLE_WIDTH_PX;
    }
    if config.hover_open_delay_ms == 0 {
        config.hover_open_delay_ms = DEFAULT_HOVER_OPEN_DELAY_MS;
    }
    if config.hover_close_delay_ms == 0 {
        config.hover_close_delay_ms = DEFAULT_HOVER_CLOSE_DELAY_MS;
    }
    if config.hotzone_width_px == 0 {
        config.hotzone_width_px = DEFAULT_HOTZONE_WIDTH_PX;
    }
    if config.empty_input_placeholder_color.trim().is_empty() {
        config.empty_input_placeholder_color = DEFAULT_EMPTY_INPUT_PLACEHOLDER_COLOR.into();
    }
    if config.save_shortcut_text_color.trim().is_empty() {
        config.save_shortcut_text_color = DEFAULT_SAVE_SHORTCUT_TEXT_COLOR.into();
    }
    if config.save_shortcut_font_size_px == 0 {
        config.save_shortcut_font_size_px = DEFAULT_SAVE_SHORTCUT_FONT_SIZE_PX;
    }
    normalize_custom_theme(&mut config.custom_theme);
    config.editor_typography = sanitize_editor_typography(config.editor_typography);
    if config.note_template.trim().is_empty() {
        config.note_template = DEFAULT_NOTE_TEMPLATE.into();
    }

    log::info!(
        "loaded app config with {} markdown targets",
        config.targets.len()
    );
    Ok(config)
}

pub fn save_app_config<R: Runtime>(app: &AppHandle<R>, config: &AppConfig) -> Result<(), String> {
    let config_path = config_file_path(app)?;
    let parent = config_path.parent().ok_or_else(|| {
        format!(
            "failed to resolve config directory for {}",
            config_path.display()
        )
    })?;

    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create config directory {}: {error}",
            parent.display()
        )
    })?;

    let raw = serde_json::to_string_pretty(config)
        .map_err(|error| format!("failed to serialize config: {error}"))?;

    atomic_write_string(&config_path, &raw).map_err(|error| {
        format!(
            "failed to write config to {}: {error}",
            config_path.display()
        )
    })?;
    ensure_config_help_file(app)
}

pub fn ensure_config_help_file<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let help_path = config_help_file_path(app)?;
    let parent = help_path.parent().ok_or_else(|| {
        format!(
            "failed to resolve config help directory for {}",
            help_path.display()
        )
    })?;

    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create config help directory {}: {error}",
            parent.display()
        )
    })?;

    atomic_write_string(&help_path, CONFIG_HELP_CONTENT).map_err(|error| {
        format!(
            "failed to write config help to {}: {error}",
            help_path.display()
        )
    })
}

fn config_file_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|path| path.join(CONFIG_FILE_NAME))
        .map_err(|error| format!("failed to resolve app config directory: {error}"))
}

fn config_help_file_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|path| path.join(CONFIG_HELP_FILE_NAME))
        .map_err(|error| format!("failed to resolve app config directory: {error}"))
}

pub fn default_app_config<R: Runtime>(app: &AppHandle<R>) -> AppConfig {
    AppConfig {
        target_file_path: default_target_file_path(app)
            .unwrap_or_else(|_| DEFAULT_TARGET_FILE_NAME.into()),
        hotkey: DEFAULT_HOTKEY.into(),
        hide_hotkey: DEFAULT_HIDE_HOTKEY.into(),
        next_target_hotkey: DEFAULT_NEXT_TARGET_HOTKEY.into(),
        side_hide_enabled: DEFAULT_SIDE_HIDE_ENABLED,
        edge_snap_threshold_px: DEFAULT_EDGE_SNAP_THRESHOLD_PX,
        visible_handle_width_px: DEFAULT_VISIBLE_HANDLE_WIDTH_PX,
        hover_open_delay_ms: DEFAULT_HOVER_OPEN_DELAY_MS,
        hover_close_delay_ms: DEFAULT_HOVER_CLOSE_DELAY_MS,
        hotzone_width_px: DEFAULT_HOTZONE_WIDTH_PX,
        debug_show_hotzone: DEFAULT_DEBUG_SHOW_HOTZONE,
        save_shortcut_mode: DEFAULT_SAVE_SHORTCUT_MODE,
        empty_input_placeholder_color: DEFAULT_EMPTY_INPUT_PLACEHOLDER_COLOR.into(),
        save_shortcut_text_color: DEFAULT_SAVE_SHORTCUT_TEXT_COLOR.into(),
        save_shortcut_font_size_px: DEFAULT_SAVE_SHORTCUT_FONT_SIZE_PX,
        custom_theme: CustomTheme::default(),
        editor_typography: EditorTypography::default(),
        targets: vec![MarkdownTarget {
            id: DEFAULT_TARGET_ID.into(),
            nickname: DEFAULT_TARGET_NICKNAME.into(),
            path: default_target_file_path(app).unwrap_or_else(|_| DEFAULT_TARGET_FILE_NAME.into()),
        }],
        active_target_id: DEFAULT_TARGET_ID.into(),
        note_template: DEFAULT_NOTE_TEMPLATE.into(),
        language_mode: DEFAULT_LANGUAGE_MODE,
        theme_mode: ThemeMode::default(),
    }
}

fn default_target_file_path<R: Runtime>(app: &AppHandle<R>) -> Result<String, String> {
    app.path()
        .document_dir()
        .map(|path| {
            path.join(DEFAULT_TARGET_FILE_NAME)
                .to_string_lossy()
                .into_owned()
        })
        .map_err(|error| format!("failed to resolve default target file path: {error}"))
}

fn default_hotkey() -> String {
    DEFAULT_HOTKEY.into()
}

fn default_hide_hotkey() -> String {
    DEFAULT_HIDE_HOTKEY.into()
}

fn default_next_target_hotkey() -> String {
    DEFAULT_NEXT_TARGET_HOTKEY.into()
}

fn default_side_hide_enabled() -> u8 {
    DEFAULT_SIDE_HIDE_ENABLED
}

fn default_edge_snap_threshold_px() -> u32 {
    DEFAULT_EDGE_SNAP_THRESHOLD_PX
}

fn default_visible_handle_width_px() -> u32 {
    DEFAULT_VISIBLE_HANDLE_WIDTH_PX
}

fn default_hover_open_delay_ms() -> u64 {
    DEFAULT_HOVER_OPEN_DELAY_MS
}

fn default_hover_close_delay_ms() -> u64 {
    DEFAULT_HOVER_CLOSE_DELAY_MS
}

fn default_hotzone_width_px() -> u32 {
    DEFAULT_HOTZONE_WIDTH_PX
}

fn default_debug_show_hotzone() -> u8 {
    DEFAULT_DEBUG_SHOW_HOTZONE
}

fn default_save_shortcut_mode() -> SaveShortcutMode {
    DEFAULT_SAVE_SHORTCUT_MODE
}

fn default_empty_input_placeholder_color() -> String {
    DEFAULT_EMPTY_INPUT_PLACEHOLDER_COLOR.into()
}

fn default_save_shortcut_text_color() -> String {
    DEFAULT_SAVE_SHORTCUT_TEXT_COLOR.into()
}

fn default_save_shortcut_font_size_px() -> u32 {
    DEFAULT_SAVE_SHORTCUT_FONT_SIZE_PX
}

fn default_target_id() -> String {
    DEFAULT_TARGET_ID.into()
}

fn default_target_nickname() -> String {
    DEFAULT_TARGET_NICKNAME.into()
}

fn default_note_template() -> String {
    DEFAULT_NOTE_TEMPLATE.into()
}

fn default_language_mode() -> LanguageMode {
    DEFAULT_LANGUAGE_MODE
}

fn default_custom_window_color() -> String {
    DEFAULT_CUSTOM_WINDOW_COLOR.into()
}

fn default_custom_window_opacity() -> f64 {
    DEFAULT_CUSTOM_WINDOW_OPACITY
}

fn default_custom_text_color() -> String {
    DEFAULT_CUSTOM_TEXT_COLOR.into()
}

fn default_custom_accent_color() -> String {
    DEFAULT_CUSTOM_ACCENT_COLOR.into()
}

fn normalize_custom_theme(custom_theme: &mut CustomTheme) {
    custom_theme.window_color =
        normalize_hex_color(&custom_theme.window_color, DEFAULT_CUSTOM_WINDOW_COLOR);
    custom_theme.window_opacity = custom_theme.window_opacity.clamp(0.35, 1.0);
    custom_theme.text_color =
        normalize_hex_color(&custom_theme.text_color, DEFAULT_CUSTOM_TEXT_COLOR);
    custom_theme.accent_color =
        normalize_hex_color(&custom_theme.accent_color, DEFAULT_CUSTOM_ACCENT_COLOR);
}

pub fn sanitize_custom_theme(mut custom_theme: CustomTheme) -> CustomTheme {
    normalize_custom_theme(&mut custom_theme);
    custom_theme
}

pub fn normalize_targets<R: Runtime>(
    app: &AppHandle<R>,
    config: &mut AppConfig,
) -> Result<(), String> {
    let fallback_path = if config.target_file_path.trim().is_empty() {
        default_target_file_path(app)?
    } else {
        config.target_file_path.trim().to_string()
    };

    config.targets = sanitize_targets(std::mem::take(&mut config.targets), &fallback_path);
    if config.targets.is_empty() {
        config.targets.push(MarkdownTarget {
            id: DEFAULT_TARGET_ID.into(),
            nickname: DEFAULT_TARGET_NICKNAME.into(),
            path: fallback_path,
        });
    }

    if !config
        .targets
        .iter()
        .any(|target| target.id == config.active_target_id)
    {
        config.active_target_id = config
            .targets
            .first()
            .map(|target| target.id.clone())
            .unwrap_or_else(|| DEFAULT_TARGET_ID.into());
    }

    if let Some(active_target) = config
        .targets
        .iter()
        .find(|target| target.id == config.active_target_id)
    {
        config.target_file_path = active_target.path.clone();
    }

    Ok(())
}

pub fn sanitize_targets(
    mut targets: Vec<MarkdownTarget>,
    fallback_path: &str,
) -> Vec<MarkdownTarget> {
    targets.retain(|target| !target.path.trim().is_empty());
    if targets.is_empty() {
        return vec![MarkdownTarget {
            id: DEFAULT_TARGET_ID.into(),
            nickname: DEFAULT_TARGET_NICKNAME.into(),
            path: fallback_path.into(),
        }];
    }

    for (index, target) in targets.iter_mut().enumerate() {
        if target.id.trim().is_empty() {
            target.id = format!("target-{}", index + 1);
        } else {
            target.id = sanitize_target_id(&target.id, index);
        }
        target.nickname = target.nickname.trim().to_string();
        target.path = target.path.trim().to_string();
    }

    targets
}

fn sanitize_target_id(value: &str, index: usize) -> String {
    let sanitized: String = value
        .trim()
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || *character == '-' || *character == '_'
        })
        .collect();

    if sanitized.is_empty() {
        format!("target-{}", index + 1)
    } else {
        sanitized
    }
}

fn normalize_hex_color(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if is_hex_color(trimmed) {
        trimmed.to_ascii_uppercase()
    } else {
        fallback.into()
    }
}

fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    hex.len() == 6 && hex.chars().all(|character| character.is_ascii_hexdigit())
}

const CONFIG_HELP_CONTENT: &str = r#"# FloatingInputTool Config Reference

This file explains the adjacent `config.json`. The app reads `config.json`; this help file is for manual editing reference only.

## Copyright
FloatingInputTool (c) 2026 chenyb. All rights reserved.

Creator platform: Xiaohongshu @落雨返屋企 (210601284kk).

## themeMode
Controls the active theme.

Allowed values:
- `follow-system`: follow the Windows light/dark setting.
- `theme-white`: use the built-in white theme.
- `theme-dark`: use the built-in dark theme.
- `custom`: use `customTheme`.

## languageMode
Controls the application UI language.

Allowed values:
- `english`: use English UI labels.
- `chinese`: use Chinese UI labels and the SimHei font stack in the web windows.

## customTheme.windowColor
Controls the main capture window surface color when `themeMode` is `custom`.

Format: hex color, for example `#F8F8FF`.

## customTheme.windowOpacity
Controls the main capture window surface opacity when `themeMode` is `custom`.

Range: `0.35` to `1.0`.

## customTheme.textColor
Controls primary text color in the main capture window when `themeMode` is `custom`.

Format: hex color, for example `#333333`.

## customTheme.accentColor
Controls accent color in the main capture window when `themeMode` is `custom`.

This affects the caret, text selection, selected states, and accent borders where supported.

Format: hex color, for example `#3EB4BF`.

## targetFilePath
The markdown file path used by the current single-target writer.

This remains for compatibility and mirrors the currently active target path.

## targets
The markdown target list shown in Settings and the main capture window target switcher.

Each item has:
- `id`: stable internal target id.
- `nickname`: short display name in the main window.
- `path`: markdown file path.

## activeTargetId
The currently selected target id. Clicking a target in the main capture window updates this field.

## noteTemplate
Controls the markdown block written for each saved note.

Supported placeholders:
- `{{timestamp}}`: save time, formatted as `yyyy-mm-dd hh:mm`.
- `{{text}}`: raw multi-line note text.
- `{{text.callout}}`: note text converted into safe Obsidian callout lines.

Default:

```md
> [!fleeting]+ {{timestamp}}
>
{{text.callout}}
```

## hotkey
The global hotkey that shows or toggles the capture window.

Example: `Ctrl+Alt+Space`.

## hideHotkey
The global hotkey that hides or docks the capture window while preserving the current draft.

Example: `Ctrl+Alt+H`.

## nextTargetHotkey
The global hotkey that switches the active markdown target to the next configured target.

The current draft stays unchanged.

Example: `Ctrl+Alt+Right`.

## saveShortcutMode
Controls how Enter behaves in the main input box.

Allowed values:
- `ctrl-enter-save`: `Ctrl+Enter` saves, `Enter` inserts a new line.
- `enter-save`: `Enter` saves, `Ctrl+Enter` inserts a new line.

## emptyInputPlaceholderColor
Controls the placeholder text color in the empty main input box.

Format: hex color, for example `#8A8A8A`.

## saveShortcutTextColor
Controls the small shortcut label color inside the Save button.

Format: hex color, for example `#333333`.

## saveShortcutFontSizePx
Controls the small shortcut label font size inside the Save button.

Unit: pixels.

## sideHideEnabled
Enables side-hide docking.

Values:
- `1`: enabled.
- `0`: disabled.

## edgeSnapThresholdPx
Distance from the left or right screen edge where the window can dock.

Unit: pixels.

## visibleHandleWidthPx
Visible side handle width while docked.

Unit: pixels.

## hoverOpenDelayMs
Delay before a docked window expands when the mouse enters its hotzone.

Unit: milliseconds.

## hoverCloseDelayMs
Delay before an expanded docked window re-docks after the cursor leaves and the editor is not focused.

Unit: milliseconds.

## hotzoneWidthPx
Width of the hover zone used to expand a docked window.

Unit: pixels.

## debugShowHotzone
Shows the side-hide hotzone overlay.

Values:
- `1`: visible.
- `0`: hidden.
"#;

#[cfg(test)]
mod typography_tests {
    use super::*;

    #[test]
    fn old_config_keeps_existing_editor_appearance() {
        let config: AppConfig = serde_json::from_str(r#"{"themeMode":"follow-system"}"#).unwrap();
        assert_eq!(config.editor_typography, EditorTypography::default());
    }

    #[test]
    fn typography_survives_config_round_trip() {
        let mut config: AppConfig =
            serde_json::from_str(r#"{"themeMode":"follow-system"}"#).unwrap();
        config.editor_typography = sanitize_editor_typography(EditorTypography {
            font_family: " Custom Font ".into(),
            font_size_px: 22,
            bold: true,
            italic: true,
            custom_fonts: vec![" Custom Font ".into(), "custom font".into(), "".into()],
        });
        let restored: AppConfig =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(restored.editor_typography, config.editor_typography);
        assert_eq!(restored.editor_typography.custom_fonts, vec!["Custom Font"]);
        assert_eq!(restored.editor_typography.font_family, "Custom Font");
    }
}
