export type ThemeMode = 'follow-system' | 'theme-white' | 'theme-dark' | 'custom'
export type ResolvedTheme = 'white' | 'dark' | 'custom'
export type FeedbackTone = 'normal' | 'error'
export type SaveShortcutMode = 'ctrl-enter-save' | 'enter-save'
export type LanguageMode = 'english' | 'chinese'

export type CustomTheme = {
  windowColor: string
  windowOpacity: number
  textColor: string
  accentColor: string
}

export type MarkdownTarget = {
  id: string
  nickname: string
  path: string
}

export type AppConfig = {
  themeMode: ThemeMode
  targetFilePath: string
  targets: MarkdownTarget[]
  activeTargetId: string
  hotkey: string
  hideHotkey: string
  nextTargetHotkey: string
  sideHideEnabled: number
  edgeSnapThresholdPx: number
  visibleHandleWidthPx: number
  hoverOpenDelayMs: number
  hoverCloseDelayMs: number
  hotzoneWidthPx: number
  debugShowHotzone: number
  saveShortcutMode: SaveShortcutMode
  emptyInputPlaceholderColor: string
  saveShortcutTextColor: string
  saveShortcutFontSizePx: number
  customTheme: CustomTheme
  noteTemplate: string
  languageMode: LanguageMode
}

export type HotkeyUpdateResponse = {
  config: AppConfig
  warning?: string | null
  activeHotkey?: string | null
}

export type HotkeyRuntimeStatus = {
  capture?: string | null
  hide?: string | null
  nextTarget?: string | null
  warnings: string[]
}

export type DraftSnapshot = {
  text: string
  targetId: string
  createdAt: string
  updatedAt: string
}

export type NativeBackdropStatus = {
  available: boolean
}

export type ThemeModeChangedPayload = {
  themeMode: ThemeMode
}

export type AppConfigChangedPayload = AppConfig

export type CustomThemePreviewChangedPayload = {
  customTheme: CustomTheme
}

export type MainWindowMode = 'normal' | 'docked-left' | 'docked-right' | 'expanded-from-dock'

export type MainWindowModeChangedPayload = {
  mode: MainWindowMode
  dockSide?: 'left' | 'right' | null
}

export const themeOptions: Array<{ value: ThemeMode; label: string; note: string }> = [
  {
    value: 'follow-system',
    label: 'Follow System',
    note: 'Match the current Windows appearance.',
  },
  {
    value: 'theme-white',
    label: 'Theme White',
    note: 'Soft jade glass with a brighter surface.',
  },
  {
    value: 'theme-dark',
    label: 'Theme Dark',
    note: 'Dark frost glass for a more focused feel.',
  },
  {
    value: 'custom',
    label: 'Custom',
    note: 'Use your own color and opacity values.',
  },
]

export const saveShortcutOptions: Array<{ value: SaveShortcutMode; label: string; note: string }> = [
  {
    value: 'ctrl-enter-save',
    label: 'Ctrl+Enter saves',
    note: 'Enter inserts a new line.',
  },
  {
    value: 'enter-save',
    label: 'Enter saves',
    note: 'Ctrl+Enter inserts a new line.',
  },
]

export function getThemeOptions(languageMode: LanguageMode) {
  if (languageMode === 'chinese') {
    return [
      {
        value: 'follow-system' as ThemeMode,
        label: '跟随系统',
        note: '匹配当前 Windows 外观。',
      },
      {
        value: 'theme-white' as ThemeMode,
        label: '白色主题',
        note: '更明亮的柔和玉石质感。',
      },
      {
        value: 'theme-dark' as ThemeMode,
        label: '黑色主题',
        note: '更聚焦的深色磨砂质感。',
      },
      {
        value: 'custom' as ThemeMode,
        label: '自定义',
        note: '使用自己的颜色和透明度。',
      },
    ]
  }

  return themeOptions
}

export function getSaveShortcutOptions(languageMode: LanguageMode) {
  if (languageMode === 'chinese') {
    return [
      {
        value: 'ctrl-enter-save' as SaveShortcutMode,
        label: 'Ctrl+Enter 保存',
        note: 'Enter 插入新行。',
      },
      {
        value: 'enter-save' as SaveShortcutMode,
        label: 'Enter 保存',
        note: 'Ctrl+Enter 插入新行。',
      },
    ]
  }

  return saveShortcutOptions
}

export const uiText = {
  english: {
    appTitle: 'Fleeting Note',
    dialogLabel: 'Fleeting note',
    dockLabel: 'Fleeting Note',
    close: 'Close',
    hide: 'Hide',
    discardDraft: 'Discard draft',
    discardDraftConfirm: 'Discard the current unsaved draft?',
    markdownTargets: 'Markdown targets',
    editorLabel: 'Fleeting note content',
    placeholder: 'Capture your fleeting thoughts',
    savedFeedback: 'Saved to markdown.',
    saving: 'Saving',
    save: 'Save',
    settingsTitle: 'Settings',
    settingsDescription: 'Theme, destination markdown path, and shortcut recording live here.',
    closeSettings: 'Close settings',
    collapse: 'Collapse',
    expand: 'Expand',
    nickname: 'Nickname',
    path: 'Path',
    options: 'Options',
    active: 'Active',
    remove: 'Remove',
    addTarget: 'Add Target',
    saveTargets: 'Save Targets',
    savingTargets: 'Saving Targets',
    cancel: 'Cancel',
    edit: 'Edit',
    noteTemplate: 'Note Template',
    markdown: 'Markdown',
    placeholders: 'Placeholders',
    saveTemplate: 'Save Template',
    savingTemplate: 'Saving Template',
    default: 'Default',
    hotkeys: 'Hotkeys',
    global: 'Global',
    capture: 'Capture',
    nextTarget: 'Next Target',
    hotkeyPlaceholder: 'Click here, then press your shortcut',
    saveHotkeyHint:
      'Save keeps the entered hotkey in config. If it conflicts, the previous working hotkey stays active.',
    recordingHotkey: 'Press your shortcut now. Modifier-only keys are ignored.',
    currentActive: 'Current active',
    noActiveShortcut: 'No active shortcut yet.',
    hotkeyUpdated: 'hotkey updated.',
    theme: 'Theme',
    windowColor: 'Window Color',
    windowOpacity: 'Window Opacity',
    textColor: 'Text Color',
    accentColor: 'Accent Color',
    saveCustom: 'Save Custom',
    savingCustom: 'Saving Custom',
    saveShortcut: 'Save Shortcut',
    targetChangesDiscarded: 'Markdown target changes discarded.',
    targetsUpdated: 'Markdown targets updated.',
    themeUpdated: 'Theme updated.',
    saveShortcutUpdated: 'Save shortcut updated.',
    noteTemplateUpdated: 'Note template updated.',
    customThemeSaved: 'Custom theme saved.',
    copyrightLine: 'FloatingInputTool © 2026 chenyb. All rights reserved.',
    creatorLine: 'Creator: Xiaohongshu @落雨返屋企 (210601284kk)',
  },
  chinese: {
    appTitle: '闪念笔记',
    dialogLabel: '闪念笔记',
    dockLabel: '闪念笔记',
    close: '关闭',
    hide: '隐藏',
    discardDraft: '丢弃草稿',
    discardDraftConfirm: '确定丢弃当前未保存的草稿吗？',
    markdownTargets: 'Markdown 目标',
    editorLabel: '闪念笔记内容',
    placeholder: '记录你的闪念',
    savedFeedback: '已保存到 Markdown。',
    saving: '保存中',
    save: '保存',
    settingsTitle: '设置',
    settingsDescription: '主题、Markdown 输出位置和快捷键录入在这里配置。',
    closeSettings: '关闭设置',
    collapse: '折叠',
    expand: '展开',
    nickname: '昵称',
    path: '路径',
    options: '选项',
    active: '当前',
    remove: '移除',
    addTarget: '添加目标',
    saveTargets: '保存目标',
    savingTargets: '保存中',
    cancel: '取消',
    edit: '编辑',
    noteTemplate: '记录模板',
    markdown: 'Markdown',
    placeholders: '占位符',
    saveTemplate: '保存模板',
    savingTemplate: '保存中',
    default: '默认',
    hotkeys: '快捷键',
    global: '全局',
    capture: '唤起',
    nextTarget: '下个标签',
    hotkeyPlaceholder: '点击这里，然后按下快捷键',
    saveHotkeyHint: '保存会写入配置。若发生冲突，本次会提示，当前会话继续使用上一个可用快捷键。',
    recordingHotkey: '请按下快捷键。单独的修饰键会被忽略。',
    currentActive: '当前启用',
    noActiveShortcut: '当前没有启用快捷键。',
    hotkeyUpdated: '快捷键已更新。',
    theme: '主题',
    windowColor: '窗口颜色',
    windowOpacity: '窗口不透明度',
    textColor: '文字颜色',
    accentColor: '强调色',
    saveCustom: '保存自定义',
    savingCustom: '保存中',
    saveShortcut: '保存快捷键',
    targetChangesDiscarded: '已放弃 Markdown 目标修改。',
    targetsUpdated: 'Markdown 目标已更新。',
    themeUpdated: '主题已更新。',
    saveShortcutUpdated: '保存快捷键已更新。',
    noteTemplateUpdated: '记录模板已更新。',
    customThemeSaved: '自定义主题已保存。',
    copyrightLine: 'FloatingInputTool © 2026 chenyb. 保留所有权利。',
    creatorLine: '创作者：小红书 @落雨返屋企（210601284kk）',
  },
} as const satisfies Record<LanguageMode, Record<string, string>>
