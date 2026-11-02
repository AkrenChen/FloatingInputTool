import { EditorTypographySettings } from './EditorTypographySettings'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { CSSProperties, PointerEvent } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { emit, listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import './App.css'
import { mergeSettingsDraft } from './settingsDraft'
import type {
  AppConfig,
  CustomTheme,
  FeedbackTone,
  HotkeyRuntimeStatus,
  HotkeyUpdateResponse,
  LanguageMode,
  MarkdownTarget,
  NativeBackdropStatus,
  ResolvedTheme,
  SaveShortcutMode,
  ThemeMode,
  ThemeModeChangedPayload,
} from './appModel'
import { getSaveShortcutOptions, getThemeOptions, uiText } from './appModel'

const MODIFIER_KEYS = new Set(['Control', 'Shift', 'Alt', 'Meta'])
type HotkeyField = 'capture' | 'hide' | 'next-target'

const defaultCustomTheme: CustomTheme = {
  windowColor: '#F8F8FF',
  windowOpacity: 0.86,
  textColor: '#333333',
  accentColor: '#3EB4BF',
}

const defaultNoteTemplate = `> [!fleeting]+ {{timestamp}}
>
{{text.callout}}`

function SettingsApp() {
  const dirtyKeys = useRef(new Set<string>())
  const [activeTab, setActiveTab] = useState('record')
  const mergeField = useCallback(<K extends keyof AppConfig>(key: K, current: AppConfig[K], incoming: AppConfig[K]) =>
    mergeSettingsDraft({ [key]: current }, { [key]: incoming }, dirtyKeys.current)[key], [])

  const [prefersDark, setPrefersDark] = useState(() =>
    window.matchMedia('(prefers-color-scheme: dark)').matches,
  )
  const [themeMode, setThemeMode] = useState<ThemeMode>('follow-system')
  const [languageMode, setLanguageMode] = useState<LanguageMode>('english')
  const [targets, setTargets] = useState<MarkdownTarget[]>([])
  const [activeTargetId, setActiveTargetId] = useState('')
  const [isTargetListExpanded, setIsTargetListExpanded] = useState(true)
  const [isEditingTargets, setIsEditingTargets] = useState(false)
  const [hotkey, setHotkey] = useState('')
  const [hotkeyInput, setHotkeyInput] = useState('')
  const [hideHotkey, setHideHotkey] = useState('')
  const [hideHotkeyInput, setHideHotkeyInput] = useState('')
  const [nextTargetHotkey, setNextTargetHotkey] = useState('')
  const [nextTargetHotkeyInput, setNextTargetHotkeyInput] = useState('')
  const [saveShortcutMode, setSaveShortcutMode] = useState<SaveShortcutMode>('ctrl-enter-save')
  const [noteTemplate, setNoteTemplate] = useState(defaultNoteTemplate)
  const [noteTemplateInput, setNoteTemplateInput] = useState(defaultNoteTemplate)
  const [customThemeDraft, setCustomThemeDraft] = useState<CustomTheme>(defaultCustomTheme)
  const [savedCustomTheme, setSavedCustomTheme] = useState<CustomTheme>(defaultCustomTheme)
  const [draggedTargetId, setDraggedTargetId] = useState('')
  const [isSavingTheme, setIsSavingTheme] = useState(false)
  const [isSavingTargets, setIsSavingTargets] = useState(false)
  const [isSavingHotkey, setIsSavingHotkey] = useState(false)
  const [isSavingHideHotkey, setIsSavingHideHotkey] = useState(false)
  const [isSavingNextTargetHotkey, setIsSavingNextTargetHotkey] = useState(false)
  const [isSavingSaveShortcutMode, setIsSavingSaveShortcutMode] = useState(false)
  const [isSavingNoteTemplate, setIsSavingNoteTemplate] = useState(false)
  const [isSavingCustomTheme, setIsSavingCustomTheme] = useState(false)
  const [settingsFeedback, setSettingsFeedback] = useState('')
  const [settingsFeedbackTone, setSettingsFeedbackTone] = useState<FeedbackTone>('normal')
  const [recordingHotkeyField, setRecordingHotkeyField] = useState<HotkeyField | null>(null)
  const [hasNativeBackdrop, setHasNativeBackdrop] = useState(false)
  const [isWindowFocused, setIsWindowFocused] = useState(true)

  const applyHotkeyConfig = useCallback((config: AppConfig) => {
    setHotkeyInput(current => mergeField('hotkey', current, config.hotkey))
    setHideHotkeyInput(current => mergeField('hideHotkey', current, config.hideHotkey))
    setNextTargetHotkeyInput(current => mergeField('nextTargetHotkey', current, config.nextTargetHotkey))
  }, [mergeField])

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const onChange = (event: MediaQueryListEvent) => {
      setPrefersDark(event.matches)
    }

    media.addEventListener('change', onChange)
    return () => media.removeEventListener('change', onChange)
  }, [])

  useEffect(() => {
    let ignore = false
    let unlistenFocusChanged: (() => void) | undefined
    const currentWindow = getCurrentWindow()

    currentWindow.isFocused().then((focused) => {
      if (!ignore) {
        setIsWindowFocused(focused)
      }
    })

    currentWindow.onFocusChanged(({ payload: focused }) => {
      if (!ignore) {
        setIsWindowFocused(focused)
      }
    }).then((unlisten) => {
      unlistenFocusChanged = unlisten
    })

    return () => {
      ignore = true
      unlistenFocusChanged?.()
    }
  }, [])

  useEffect(() => {
    let ignore = false
    let unlistenThemeChange: (() => void) | undefined
    let unlistenAppConfigChange: (() => void) | undefined
    let unlistenNativeBackdrop: (() => void) | undefined

    const loadConfig = async () => {
      try {
        const [config, hotkeyStatus, backdropStatus] = await Promise.all([
          invoke<AppConfig>('get_app_config'),
          invoke<HotkeyRuntimeStatus>('get_hotkey_runtime_status'),
          invoke<NativeBackdropStatus>('get_native_backdrop_status'),
        ])
        if (!ignore) {
          setThemeMode(config.themeMode)
          setLanguageMode(config.languageMode)
          setTargets(config.targets)
          setActiveTargetId(config.activeTargetId)
          setHotkey(hotkeyStatus.capture ?? '')
          setHotkeyInput(config.hotkey)
          setHideHotkey(hotkeyStatus.hide ?? '')
          setHideHotkeyInput(config.hideHotkey)
          setNextTargetHotkey(hotkeyStatus.nextTarget ?? '')
          setNextTargetHotkeyInput(config.nextTargetHotkey)
          setSaveShortcutMode(config.saveShortcutMode)
          setNoteTemplate(config.noteTemplate)
          setNoteTemplateInput(config.noteTemplate)
          setCustomThemeDraft(config.customTheme)
          setSavedCustomTheme(config.customTheme)
          setHasNativeBackdrop(backdropStatus.available)
          if (hotkeyStatus.warnings.length > 0) {
            setSettingsFeedback(hotkeyStatus.warnings.join(' '))
            setSettingsFeedbackTone('error')
          }
        }
      } catch (error) {
        console.error('Failed to load settings config', error)
      }
    }

    const attachThemeListener = async () => {
      unlistenThemeChange = await listen<ThemeModeChangedPayload>('theme-mode-changed', (event) => {
        if (!ignore) {
          setThemeMode(event.payload.themeMode)
          setIsSavingTheme(false)
        }
      })
    }

    const applyConfig = (config: AppConfig) => {
      setThemeMode(config.themeMode)
      setLanguageMode(config.languageMode)
      setTargets(current => mergeField('targets', current, config.targets))
      setActiveTargetId(current => mergeField('activeTargetId', current, config.activeTargetId))
      applyHotkeyConfig(config)
      setSaveShortcutMode(config.saveShortcutMode)
      setNoteTemplate(config.noteTemplate)
      setNoteTemplateInput(current => mergeField('noteTemplate', current, config.noteTemplate))
      setCustomThemeDraft(current => mergeField('customTheme', current, config.customTheme))
      setSavedCustomTheme(config.customTheme)
      void invoke<HotkeyRuntimeStatus>('get_hotkey_runtime_status').then(status => {
        if (!ignore) {
          setHotkey(status.capture ?? '')
          setHideHotkey(status.hide ?? '')
          setNextTargetHotkey(status.nextTarget ?? '')
        }
      }).catch(console.error)
    }

    const attachAppConfigListener = async () => {
      unlistenAppConfigChange = await listen<AppConfig>('app-config-changed', (event) => {
        if (!ignore) {
          applyConfig(event.payload)
        }
      })
    }

    const attachNativeBackdropListener = async () => {
      unlistenNativeBackdrop = await listen<NativeBackdropStatus>('native-backdrop-changed', (event) => {
        if (!ignore) {
          setHasNativeBackdrop(event.payload.available)
        }
      })
    }

    loadConfig()
    attachThemeListener().catch((error) => {
      console.error('Failed to attach settings listeners', error)
    })
    attachAppConfigListener().catch((error) => {
      console.error('Failed to attach settings app config listener', error)
    })
    attachNativeBackdropListener().catch((error) => {
      console.error('Failed to attach settings native backdrop listener', error)
    })

    return () => {
      ignore = true
      unlistenThemeChange?.()
      unlistenAppConfigChange?.()
      unlistenNativeBackdrop?.()
    }
  }, [applyHotkeyConfig, mergeField])

  useEffect(() => {
    void emit('custom-theme-preview-changed', {
      customTheme: isWindowFocused && themeMode === 'custom' ? customThemeDraft : null,
    })
  }, [isWindowFocused, themeMode, customThemeDraft])

  const resolvedTheme = useMemo<ResolvedTheme>(() => {
    if (themeMode === 'theme-dark') {
      return 'dark'
    }

    if (themeMode === 'theme-white') {
      return 'white'
    }

    if (themeMode === 'custom') {
      return 'custom'
    }

    return prefersDark ? 'dark' : 'white'
  }, [prefersDark, themeMode])

  const themeModeLabel = useMemo(() => {
    return getThemeOptions(languageMode).find((option) => option.value === themeMode)?.label ?? 'Follow System'
  }, [languageMode, themeMode])

  const themeOptions = useMemo(() => getThemeOptions(languageMode), [languageMode])
  const saveShortcutOptions = useMemo(() => getSaveShortcutOptions(languageMode), [languageMode])
  const text = uiText[languageMode]

  const settingsShellStyle = {
    '--custom-window-color': customThemeDraft.windowColor,
    '--custom-text-color': customThemeDraft.textColor,
    '--custom-accent-color': customThemeDraft.accentColor,
  } as CSSProperties

  const getHotkeyRecorderNote = (field: HotkeyField, activeHotkey: string) => {
    if (recordingHotkeyField === field) {
      return text.recordingHotkey
    }

    if (activeHotkey) {
      return `${text.currentActive}: ${activeHotkey}`
    }

    return text.noActiveShortcut
  }

  const handleWindowDragStart = async (event: React.MouseEvent<HTMLElement>) => {
    if (event.button !== 0) {
      return
    }

    const target = event.target as HTMLElement | null
    if (target?.closest('button, textarea, input, select, [data-no-drag]')) {
      return
    }

    try {
      await getCurrentWindow().startDragging()
    } catch (error) {
      console.error('Failed to start settings window drag', error)
    }
  }

  const handleThemeChange = async (nextThemeMode: ThemeMode) => {
    if (themeMode === nextThemeMode || isSavingTheme) {
      return
    }

    setIsSavingTheme(true)
    setSettingsFeedback('')
    setSettingsFeedbackTone('normal')

    try {
      const config = await invoke<AppConfig>('set_theme_mode', { themeMode: nextThemeMode })
      setThemeMode(config.themeMode)
      if (config.themeMode === 'custom') {
        void emit('custom-theme-preview-changed', { customTheme: customThemeDraft })
      }
      setSettingsFeedback(text.themeUpdated)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setSettingsFeedback(message)
      setSettingsFeedbackTone('error')
    } finally {
      setIsSavingTheme(false)
    }
  }

  const handleAddTarget = () => {
    dirtyKeys.current.add('targets')
    dirtyKeys.current.add('activeTargetId')
    const nextTarget: MarkdownTarget = {
      id: createTargetId(),
      nickname: 'New Target',
      path: '',
    }
    setTargets((currentTargets) => [...currentTargets, nextTarget])
    setActiveTargetId(nextTarget.id)
  }

  const handleUpdateTarget = (targetId: string, patch: Partial<MarkdownTarget>) => {
    dirtyKeys.current.add('targets')
    dirtyKeys.current.add('activeTargetId')
    setTargets((currentTargets) =>
      currentTargets.map((target) => (target.id === targetId ? { ...target, ...patch } : target)),
    )
  }

  const handleRemoveTarget = (targetId: string) => {
    dirtyKeys.current.add('targets')
    dirtyKeys.current.add('activeTargetId')
    setTargets((currentTargets) => {
      if (currentTargets.length <= 1) {
        return currentTargets
      }

      const nextTargets = currentTargets.filter((target) => target.id !== targetId)
      if (activeTargetId === targetId) {
        setActiveTargetId(nextTargets[0]?.id ?? '')
      }
      return nextTargets
    })
  }

  const moveTarget = (draggedId: string, targetId: string, placement: 'before' | 'after') => {
    dirtyKeys.current.add('targets')
    dirtyKeys.current.add('activeTargetId')
    if (!draggedId || draggedId === targetId) {
      return
    }

    setTargets((currentTargets) => {
      const draggedTarget = currentTargets.find((target) => target.id === draggedId)
      if (!draggedTarget) {
        return currentTargets
      }

      const nextTargets = currentTargets.filter((target) => target.id !== draggedId)
      const targetIndex = nextTargets.findIndex((target) => target.id === targetId)
      if (targetIndex === -1) {
        return currentTargets
      }

      nextTargets.splice(targetIndex + (placement === 'after' ? 1 : 0), 0, draggedTarget)
      return nextTargets
    })
  }

  const handleTargetDragStart = (targetId: string, event: PointerEvent<HTMLButtonElement>) => {
    if (!isEditingTargets) {
      return
    }

    if (event.button !== 0) {
      return
    }

    event.preventDefault()
    event.currentTarget.setPointerCapture(event.pointerId)
    setDraggedTargetId(targetId)
  }

  useEffect(() => {
    if (!draggedTargetId) {
      return
    }

    const handlePointerMove = (event: globalThis.PointerEvent) => {
      const targetRow = document
        .elementsFromPoint(event.clientX, event.clientY)
        .find(
          (element): element is HTMLElement =>
            element instanceof HTMLElement && typeof element.dataset.targetId === 'string',
        )

      const targetId = targetRow?.dataset.targetId
      if (!targetId || targetId === draggedTargetId) {
        return
      }

      const rect = targetRow.getBoundingClientRect()
      const placement = event.clientY > rect.top + rect.height / 2 ? 'after' : 'before'
      moveTarget(draggedTargetId, targetId, placement)
    }

    const handlePointerEnd = () => {
      setDraggedTargetId('')
    }

    window.addEventListener('pointermove', handlePointerMove)
    window.addEventListener('pointerup', handlePointerEnd)
    window.addEventListener('blur', handlePointerEnd)

    return () => {
      window.removeEventListener('pointermove', handlePointerMove)
      window.removeEventListener('pointerup', handlePointerEnd)
      window.removeEventListener('blur', handlePointerEnd)
    }
  }, [draggedTargetId])

  const handleSaveTargets = async () => {
    setIsSavingTargets(true)
    setSettingsFeedback('')
    setSettingsFeedbackTone('normal')

    try {
      const config = await invoke<AppConfig>('set_markdown_targets', {
        targets,
        activeTargetId,
      })
      setTargets(config.targets)
      setActiveTargetId(config.activeTargetId)
      dirtyKeys.current.delete('targets')
      dirtyKeys.current.delete('activeTargetId')
      setIsEditingTargets(false)
      setSettingsFeedback(text.targetsUpdated)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setSettingsFeedback(message)
      setSettingsFeedbackTone('error')
    } finally {
      setIsSavingTargets(false)
    }
  }

  const handleCancelTargets = async () => {
    try {
      const config = await invoke<AppConfig>('get_app_config')
      setTargets(config.targets)
      setActiveTargetId(config.activeTargetId)
      dirtyKeys.current.delete('targets')
      dirtyKeys.current.delete('activeTargetId')
      setIsEditingTargets(false)
      setSettingsFeedback(text.targetChangesDiscarded)
      setSettingsFeedbackTone('normal')
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setSettingsFeedback(message)
      setSettingsFeedbackTone('error')
    }
  }

  const handleSaveHotkey = async (field: HotkeyField) => {
    setHotkeySavingState(field, true)
    setSettingsFeedback('')
    setSettingsFeedbackTone('normal')

    try {
      const response = await invoke<HotkeyUpdateResponse>(hotkeyCommandForField(field), {
        hotkey: hotkeyInputForField(field),
      })
      dirtyKeys.current.delete(field === 'capture' ? 'hotkey' : field === 'hide' ? 'hideHotkey' : 'nextTargetHotkey')
      applyHotkeyConfig(response.config)
      setActiveHotkeyForField(field, response.activeHotkey ?? '')

      if (response.warning) {
        setSettingsFeedback(response.warning)
        setSettingsFeedbackTone('error')
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setSettingsFeedback(message)
      setSettingsFeedbackTone('error')
    } finally {
      setHotkeySavingState(field, false)
    }
  }

  const hotkeyInputForField = (field: HotkeyField) => {
    if (field === 'hide') {
      return hideHotkeyInput
    }

    if (field === 'next-target') {
      return nextTargetHotkeyInput
    }

    return hotkeyInput
  }

  const hotkeyCommandForField = (field: HotkeyField) => {
    if (field === 'hide') {
      return 'set_hide_hotkey'
    }

    if (field === 'next-target') {
      return 'set_next_target_hotkey'
    }

    return 'set_hotkey'
  }

  const setHotkeySavingState = (field: HotkeyField, isSaving: boolean) => {
    if (field === 'hide') {
      setIsSavingHideHotkey(isSaving)
      return
    }

    if (field === 'next-target') {
      setIsSavingNextTargetHotkey(isSaving)
      return
    }

    setIsSavingHotkey(isSaving)
  }

  const setActiveHotkeyForField = (field: HotkeyField, activeHotkey: string) => {
    if (field === 'hide') {
      setHideHotkey(activeHotkey)
      return
    }

    if (field === 'next-target') {
      setNextTargetHotkey(activeHotkey)
      return
    }

    setHotkey(activeHotkey)
  }

  const handleSaveShortcutModeChange = async (nextMode: SaveShortcutMode) => {
    if (saveShortcutMode === nextMode || isSavingSaveShortcutMode) {
      return
    }

    setIsSavingSaveShortcutMode(true)
    setSettingsFeedback('')
    setSettingsFeedbackTone('normal')

    try {
      const config = await invoke<AppConfig>('set_save_shortcut_mode', {
        saveShortcutMode: nextMode,
      })
      setSaveShortcutMode(config.saveShortcutMode)
      setSettingsFeedback(text.saveShortcutUpdated)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setSettingsFeedback(message)
      setSettingsFeedbackTone('error')
    } finally {
      setIsSavingSaveShortcutMode(false)
    }
  }

  const handleSaveNoteTemplate = async () => {
    setIsSavingNoteTemplate(true)
    setSettingsFeedback('')
    setSettingsFeedbackTone('normal')

    try {
      const config = await invoke<AppConfig>('set_note_template', {
        noteTemplate: noteTemplateInput,
      })
      dirtyKeys.current.delete('noteTemplate')
      setNoteTemplate(config.noteTemplate)
      setNoteTemplateInput(config.noteTemplate)
      setSettingsFeedback(text.noteTemplateUpdated)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setSettingsFeedback(message)
      setSettingsFeedbackTone('error')
    } finally {
      setIsSavingNoteTemplate(false)
    }
  }

  const handleCancelNoteTemplate = () => {
    dirtyKeys.current.delete('noteTemplate')
    setNoteTemplateInput(noteTemplate)
    setSettingsFeedback('')
    setSettingsFeedbackTone('normal')
  }

  const handleResetNoteTemplate = () => {
    dirtyKeys.current.add('noteTemplate')
    setNoteTemplateInput(defaultNoteTemplate)
    setSettingsFeedback('')
    setSettingsFeedbackTone('normal')
  }

  const handleCustomThemeDraftChange = (nextCustomTheme: CustomTheme) => {
    dirtyKeys.current.add('customTheme')
    setCustomThemeDraft(nextCustomTheme)
    if (themeMode === 'custom') {
      void emit('custom-theme-preview-changed', { customTheme: nextCustomTheme })
    }
  }

  const handleSaveCustomTheme = async () => {
    setIsSavingCustomTheme(true)
    setSettingsFeedback('')
    setSettingsFeedbackTone('normal')

    try {
      const config = await invoke<AppConfig>('set_custom_theme', {
        customTheme: customThemeDraft,
      })
      dirtyKeys.current.delete('customTheme')
      setCustomThemeDraft(config.customTheme)
      setSavedCustomTheme(config.customTheme)
      setSettingsFeedback(text.customThemeSaved)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setSettingsFeedback(message)
      setSettingsFeedbackTone('error')
    } finally {
      setIsSavingCustomTheme(false)
    }
  }

  const handleCancelCustomTheme = () => {
    dirtyKeys.current.delete('customTheme')
    setCustomThemeDraft(savedCustomTheme)
    if (themeMode === 'custom') {
      void emit('custom-theme-preview-changed', { customTheme: savedCustomTheme })
    }
    setSettingsFeedback('')
    setSettingsFeedbackTone('normal')
  }

  const handleHotkeyKeyDown = (event: React.KeyboardEvent<HTMLInputElement>, field: HotkeyField) => {
    event.preventDefault()
    event.stopPropagation()

    const nextHotkey = buildAcceleratorFromKeyboardEvent(event)
    if (!nextHotkey) {
      setSettingsFeedback('')
      setSettingsFeedbackTone('normal')
      return
    }

    dirtyKeys.current.add(field === 'capture' ? 'hotkey' : field === 'hide' ? 'hideHotkey' : 'nextTargetHotkey')
    if (field === 'hide') {
      setHideHotkeyInput(nextHotkey)
    } else if (field === 'next-target') {
      setNextTargetHotkeyInput(nextHotkey)
    } else {
      setHotkeyInput(nextHotkey)
    }
    setSettingsFeedback('')
    setSettingsFeedbackTone('normal')
  }

  const handleClose = async () => {
    try {
      await emit('custom-theme-preview-changed', { customTheme: null })
      await invoke('close_settings_window')
    } catch (error) {
      console.error('Failed to close settings window', error)
    }
  }

  return (
    <main
      className={`settings-app-shell theme-${resolvedTheme} lang-${languageMode} ${
        hasNativeBackdrop ? 'backdrop-native' : 'backdrop-fallback'
      } ${isWindowFocused ? 'window-focused' : 'window-unfocused'}`}
      style={settingsShellStyle}
    >
      <section className="settings-panel settings-window" role="dialog" aria-label={text.settingsTitle}>
        <div className="settings-header">
          <div
            className="settings-window-drag-area settings-drag-area"
            data-tauri-drag-region
            onMouseDown={handleWindowDragStart}
          >
            <h2 data-tauri-drag-region>{text.settingsTitle}</h2>
            <p data-tauri-drag-region>{text.settingsDescription}</p>
          </div>
          <button
            className="icon-button settings-close-button"
            type="button"
            aria-label={text.closeSettings}
            data-no-drag
            onClick={handleClose}
          >
            &times;
          </button>
        </div>

        <nav className="settings-tabs" aria-label={text.settingsTitle}>
          {(['record', 'shortcuts', 'appearance', 'advanced'] as const).map((tab, index) => (
            <button key={tab} type="button" aria-current={activeTab === tab ? 'page' : undefined}
              onClick={() => setActiveTab(tab)}>
              {(languageMode === 'chinese' ? ['记录', '快捷键', '外观', '高级'] : ['Capture', 'Shortcuts', 'Appearance', 'Advanced'])[index]}
            </button>
          ))}
        </nav>
        <div className="settings-content">
          <div className="settings-section" hidden={activeTab !== 'record'}>
            <div className="settings-label-row">
              <span className="settings-label">{text.markdownTargets}</span>
              <button
                className="settings-text-button"
                type="button"
                onClick={() => {
                  setIsTargetListExpanded((isExpanded) => !isExpanded)
                  setDraggedTargetId('')
                }}
              >
                {isTargetListExpanded ? text.collapse : text.expand}
              </button>
            </div>
            {isTargetListExpanded ? (
              <div className={`target-list-editor ${isEditingTargets ? 'is-editing' : ''}`}>
                <div className="target-editor-header" aria-hidden="true" hidden>
                  <span />
                  <span>{text.nickname}</span>
                  <span>{text.path}</span>
                  <span>{text.options}</span>
                </div>
                {targets.map((target) => (
                  <div
                    className={`target-editor-row ${target.id === draggedTargetId ? 'is-dragging' : ''}`}
                    key={target.id}
                    data-target-id={target.id}
                  >
                    <button
                      className="target-drag-handle"
                      type="button"
                      aria-label={`Drag ${target.nickname || target.path || 'target'}`}
                      onPointerDown={(event) => handleTargetDragStart(target.id, event)}
                    >
                      ⋮⋮
                    </button>
                    <span className="target-drag-placeholder" aria-hidden="true">
                      ||
                    </span>
                    <label className="target-editor-field">
                      <span className="target-field-label">{text.nickname}</span>
                      <input
                        className="settings-input"
                        type="text"
                        value={target.nickname}
                        readOnly={!isEditingTargets}
                        onChange={(event) => handleUpdateTarget(target.id, { nickname: event.target.value })}
                      />
                    </label>
                    <label className="target-editor-field target-editor-path">
                      <span className="target-field-label">{text.path}</span>
                      <input
                        className="settings-input"
                        type="text"
                        value={target.path}
                        readOnly={!isEditingTargets}
                        onChange={(event) => handleUpdateTarget(target.id, { path: event.target.value })}
                        placeholder="D:\\OneDrive\\Obsidian\\Fleeting Note.md"
                      />
                    </label>
                    <div className="target-editor-actions">
                      <label className="target-active-option">
                        <input
                          type="radio"
                          checked={target.id === activeTargetId}
                          disabled={!isEditingTargets}
                          onChange={() => { dirtyKeys.current.add('activeTargetId'); setActiveTargetId(target.id) }}
                        />
                        {text.active}
                      </label>
                      <button
                        className="settings-text-button target-remove-button"
                        type="button"
                        onClick={() => handleRemoveTarget(target.id)}
                        disabled={!isEditingTargets || targets.length <= 1}
                      >
                        {text.remove}
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            ) : null}
            {isTargetListExpanded ? (
              <div className="settings-actions">
                {isEditingTargets ? (
                  <>
                    <button className="settings-save-button" type="button" onClick={handleAddTarget}>
                      {text.addTarget}
                    </button>
                    <button
                      className="settings-save-button"
                      type="button"
                      onClick={handleSaveTargets}
                      disabled={isSavingTargets}
                    >
                      {isSavingTargets ? text.savingTargets : text.saveTargets}
                    </button>
                    <button className="settings-save-button is-secondary" type="button" onClick={handleCancelTargets}>
                      {text.cancel}
                    </button>
                  </>
                ) : (
                  <button className="settings-save-button" type="button" onClick={() => setIsEditingTargets(true)}>
                    {text.edit}
                  </button>
                )}
              </div>
            ) : null}
          </div>

          <div className="settings-section" hidden={activeTab !== 'advanced'}>
            <div className="settings-label-row">
              <span className="settings-label">{text.noteTemplate}</span>
              <span className="settings-value">{text.markdown}</span>
            </div>
            <textarea
              className="settings-input note-template-input"
              value={noteTemplateInput}
              onChange={(event) => { dirtyKeys.current.add('noteTemplate'); setNoteTemplateInput(event.target.value) }}
              spellCheck={false}
            />
            <p className="settings-inline-value">
              {text.placeholders}: {'{{timestamp}}'}, {'{{text}}'}, {'{{text.callout}}'}.
            </p>
            <div className="settings-actions">
              <button
                className="settings-save-button"
                type="button"
                onClick={handleSaveNoteTemplate}
                disabled={isSavingNoteTemplate}
              >
                {isSavingNoteTemplate ? text.savingTemplate : text.saveTemplate}
              </button>
              <button className="settings-save-button is-secondary" type="button" onClick={handleResetNoteTemplate}>
                {text.default}
              </button>
              <button className="settings-save-button is-secondary" type="button" onClick={handleCancelNoteTemplate}>
                {text.cancel}
              </button>
            </div>
          </div>

          <div className="settings-section" hidden={activeTab !== 'shortcuts'}>
            <div className="settings-label-row">
              <span className="settings-label">{text.hotkeys}</span>
              <span className="settings-value">{text.global}</span>
            </div>
            <div className="hotkey-list">
              <div className="hotkey-row">
                <label className="settings-label" htmlFor="capture-hotkey">
                  {text.capture}
                </label>
                <input
                  id="capture-hotkey"
                  className={`settings-input hotkey-recorder ${recordingHotkeyField === 'capture' ? 'is-recording' : ''}`}
                  type="text"
                  value={hotkeyInput}
                  readOnly
                  onFocus={() => setRecordingHotkeyField('capture')}
                  onBlur={() => setRecordingHotkeyField(null)}
                  onKeyDown={(event) => handleHotkeyKeyDown(event, 'capture')}
                  placeholder={text.hotkeyPlaceholder}
                />
                <button
                  className="settings-save-button"
                  type="button"
                  onClick={() => handleSaveHotkey('capture')}
                  disabled={isSavingHotkey}
                >
                  {isSavingHotkey ? text.saving : text.save}
                </button>
                <span className="settings-inline-value">{getHotkeyRecorderNote('capture', hotkey)}</span>
              </div>

              <div className="hotkey-row">
                <label className="settings-label" htmlFor="hide-hotkey">
                  {text.hide}
                </label>
                <input
                  id="hide-hotkey"
                  className={`settings-input hotkey-recorder ${recordingHotkeyField === 'hide' ? 'is-recording' : ''}`}
                  type="text"
                  value={hideHotkeyInput}
                  readOnly
                  onFocus={() => setRecordingHotkeyField('hide')}
                  onBlur={() => setRecordingHotkeyField(null)}
                  onKeyDown={(event) => handleHotkeyKeyDown(event, 'hide')}
                  placeholder={text.hotkeyPlaceholder}
                />
                <button
                  className="settings-save-button"
                  type="button"
                  onClick={() => handleSaveHotkey('hide')}
                  disabled={isSavingHideHotkey}
                >
                  {isSavingHideHotkey ? text.saving : text.save}
                </button>
                <span className="settings-inline-value">{getHotkeyRecorderNote('hide', hideHotkey)}</span>
              </div>

              <div className="hotkey-row">
                <label className="settings-label" htmlFor="next-target-hotkey">
                  {text.nextTarget}
                </label>
                <input
                  id="next-target-hotkey"
                  className={`settings-input hotkey-recorder ${
                    recordingHotkeyField === 'next-target' ? 'is-recording' : ''
                  }`}
                  type="text"
                  value={nextTargetHotkeyInput}
                  readOnly
                  onFocus={() => setRecordingHotkeyField('next-target')}
                  onBlur={() => setRecordingHotkeyField(null)}
                  onKeyDown={(event) => handleHotkeyKeyDown(event, 'next-target')}
                  placeholder={text.hotkeyPlaceholder}
                />
                <button
                  className="settings-save-button"
                  type="button"
                  onClick={() => handleSaveHotkey('next-target')}
                  disabled={isSavingNextTargetHotkey}
                >
                  {isSavingNextTargetHotkey ? text.saving : text.save}
                </button>
                <span className="settings-inline-value">{getHotkeyRecorderNote('next-target', nextTargetHotkey)}</span>
              </div>
            </div>
            <span className="settings-inline-value">
              {text.saveHotkeyHint}
            </span>
          </div>

          <div hidden={activeTab !== 'appearance'}><EditorTypographySettings language={languageMode} /></div>

          <div className="settings-section" hidden={activeTab !== 'appearance'}>
            <div className="settings-label-row">
              <span className="settings-label">{text.theme}</span>
              <span className="settings-value">{themeModeLabel}</span>
            </div>

            <div className="theme-option-list">
              {themeOptions.map((option) => {
                const selected = option.value === themeMode
                return (
                  <button
                    key={option.value}
                    className={`theme-option ${selected ? 'is-selected' : ''}`}
                    type="button"
                    onClick={() => handleThemeChange(option.value)}
                    disabled={isSavingTheme}
                  >
                    <span className="theme-option-label">{option.label}</span>
                    <span className="theme-option-note">{option.note}</span>
                  </button>
                )
              })}
            </div>

            {themeMode === 'custom' ? (
              <div className="custom-theme-editor">
                <label className="custom-theme-field">
                  <span className="settings-label">{text.windowColor}</span>
                  <input
                    type="color"
                    value={customThemeDraft.windowColor}
                    onChange={(event) =>
                      handleCustomThemeDraftChange({
                        ...customThemeDraft,
                        windowColor: event.target.value,
                      })
                    }
                  />
                </label>

                <label className="custom-theme-field">
                  <span className="settings-label">{text.windowOpacity}</span>
                  <input
                    className="settings-input"
                    type="number"
                    min="0.35"
                    max="1"
                    step="0.01"
                    value={customThemeDraft.windowOpacity}
                    onChange={(event) =>
                      handleCustomThemeDraftChange({
                        ...customThemeDraft,
                        windowOpacity: parseOpacityInput(event.target.value),
                      })
                    }
                  />
                </label>

                <label className="custom-theme-field">
                  <span className="settings-label">{text.textColor}</span>
                  <input
                    type="color"
                    value={customThemeDraft.textColor}
                    onChange={(event) =>
                      handleCustomThemeDraftChange({
                        ...customThemeDraft,
                        textColor: event.target.value,
                      })
                    }
                  />
                </label>

                <label className="custom-theme-field">
                  <span className="settings-label">{text.accentColor}</span>
                  <input
                    type="color"
                    value={customThemeDraft.accentColor}
                    onChange={(event) =>
                      handleCustomThemeDraftChange({
                        ...customThemeDraft,
                        accentColor: event.target.value,
                      })
                    }
                  />
                </label>

                <div className="settings-actions">
                  <button
                    className="settings-save-button"
                    type="button"
                    onClick={handleSaveCustomTheme}
                    disabled={isSavingCustomTheme}
                  >
                    {isSavingCustomTheme ? text.savingCustom : text.saveCustom}
                  </button>
                  <button className="settings-save-button is-secondary" type="button" onClick={handleCancelCustomTheme}>
                    {text.cancel}
                  </button>
                </div>
              </div>
            ) : null}
          </div>

          <div className="settings-section" hidden={activeTab !== 'shortcuts'}>
            <div className="settings-label-row">
              <span className="settings-label">{text.saveShortcut}</span>
              <span className="settings-value">
                {saveShortcutOptions.find((option) => option.value === saveShortcutMode)?.label}
              </span>
            </div>

            <div className="theme-option-list">
              {saveShortcutOptions.map((option) => {
                const selected = option.value === saveShortcutMode
                return (
                  <button
                    key={option.value}
                    className={`theme-option ${selected ? 'is-selected' : ''}`}
                    type="button"
                    onClick={() => handleSaveShortcutModeChange(option.value)}
                    disabled={isSavingSaveShortcutMode}
                  >
                    <span className="theme-option-label">{option.label}</span>
                    <span className="theme-option-note">{option.note}</span>
                  </button>
                )
              })}
            </div>
          </div>

          <footer className="settings-footer" hidden={activeTab !== 'advanced'} aria-label="Copyright">
            <span>{text.copyrightLine}</span>
            <span>{text.creatorLine}</span>
          </footer>
        </div>
        <div className={`settings-feedback ${settingsFeedbackTone === 'error' ? 'is-error' : ''}`} role="status">
          {settingsFeedback || '\u00a0'}
        </div>
      </section>
    </main>
  )
}

function buildAcceleratorFromKeyboardEvent(event: React.KeyboardEvent<HTMLInputElement>) {
  const mainKey = normalizeAcceleratorKey(event.key)
  if (!mainKey) {
    return null
  }

  const parts: string[] = []
  if (event.ctrlKey) {
    parts.push('Ctrl')
  }
  if (event.altKey) {
    parts.push('Alt')
  }
  if (event.shiftKey) {
    parts.push('Shift')
  }
  if (event.metaKey) {
    parts.push('Meta')
  }

  parts.push(mainKey)
  return parts.join('+')
}

function normalizeAcceleratorKey(key: string) {
  if (MODIFIER_KEYS.has(key)) {
    return null
  }

  if (key === ' ') {
    return 'Space'
  }

  if (/^[a-z]$/i.test(key)) {
    return key.toUpperCase()
  }

  if (/^[0-9]$/.test(key)) {
    return key
  }

  if (/^F([1-9]|1[0-9]|2[0-4])$/i.test(key)) {
    return key.toUpperCase()
  }

  const specialKeyMap: Record<string, string> = {
    ArrowUp: 'Up',
    ArrowDown: 'Down',
    ArrowLeft: 'Left',
    ArrowRight: 'Right',
    Escape: 'Esc',
    Enter: 'Enter',
    Tab: 'Tab',
    Backspace: 'Backspace',
    Delete: 'Delete',
    Insert: 'Insert',
    Home: 'Home',
    End: 'End',
    PageUp: 'PageUp',
    PageDown: 'PageDown',
  }

  return specialKeyMap[key] ?? null
}

function parseOpacityInput(value: string) {
  const nextOpacity = Number(value)
  if (!Number.isFinite(nextOpacity)) {
    return defaultCustomTheme.windowOpacity
  }

  return Math.min(Math.max(nextOpacity, 0.35), 1)
}

function createTargetId() {
  return `target-${Date.now().toString(36)}`
}

export default SettingsApp
