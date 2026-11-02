import { editorTypographyStyle } from './editorTypography'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { CSSProperties } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import './App.css'
import { defaultFocusEffects, focusEffectVariables, SURFACE_STATE_EVENT, surfaceColor } from './themeEffects'
import type { SurfaceState } from './themeEffects'
import type {
  AppConfig,
  AppConfigChangedPayload,
  CustomTheme,
  CustomThemePreviewChangedPayload,
  DraftSnapshot,
  HotkeyRuntimeStatus,
  LanguageMode,
  MainWindowMode,
  MainWindowModeChangedPayload,
  MarkdownTarget,
  NativeBackdropStatus,
  ResolvedTheme,
  SaveShortcutMode,
  ThemeModeChangedPayload,
} from './appModel'
import { uiText } from './appModel'
import { shouldSubmit, shouldRedock } from './captureBehavior'

const nativeDockObserver = navigator.userAgent.includes('Windows')

function sideHideDebugLog(label: string, details?: Record<string, unknown>) {
  const time = new Date().toISOString()
  if (details) {
    console.log(`[side-hide][${time}] ${label}`, details)
  } else {
    console.log(`[side-hide][${time}] ${label}`)
  }
}

function App() {
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null)
  const [prefersDark, setPrefersDark] = useState(() =>
    window.matchMedia('(prefers-color-scheme: dark)').matches,
  )
  const [themeMode, setThemeMode] = useState<AppConfig['themeMode']>('follow-system')
  const [languageMode, setLanguageMode] = useState<LanguageMode>('english')
  const [customThemePreview, setCustomThemePreview] = useState<CustomTheme | null>(null)
  const [mainWindowMode, setMainWindowMode] = useState<MainWindowMode>('normal')
  const [dockSide, setDockSide] = useState<'left' | 'right' | null>(null)
  const [draft, setDraft] = useState('')
  const [isDraftLoaded, setIsDraftLoaded] = useState(false)
  const [hasNativeBackdrop, setHasNativeBackdrop] = useState(false)
  const [isWindowFocused, setIsWindowFocused] = useState(true)
  const [surfaceOpacity, setSurfaceOpacity] = useState(1)
  const [createdAt, setCreatedAt] = useState(() => formatTimestamp(new Date()))
  const [captureFeedback, setCaptureFeedback] = useState('')
  const [isSavingNote, setIsSavingNote] = useState(false)
  const composingRef = useRef(false)
  const savingRef = useRef(false)
  const modeRef = useRef<MainWindowMode>('normal')
  const openTimerRef = useRef<number | null>(null)
  const closeTimerRef = useRef<number | null>(null)
  const manualDragActiveRef = useRef(false)
  const previousMainWindowModeRef = useRef<MainWindowMode>('normal')
  const isEditorFocusedRef = useRef(false)
  const cursorInsideWindowRef = useRef(false)
  const dockReopenBlockedUntilRef = useRef(0)
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)

  const cursorPaddingPx = 8
  const dockReopenCooldownMs = 260

  const clearCloseTimer = useCallback(() => {
    if (closeTimerRef.current) {
      window.clearTimeout(closeTimerRef.current)
      closeTimerRef.current = null
    }
  }, [])

  const scheduleRedock = useCallback(
    (reason: string, delayOverride?: number) => {
      if (nativeDockObserver) return
      if (mainWindowMode !== 'expanded-from-dock') {
        sideHideDebugLog('scheduleRedock skipped: not expanded', { reason, mainWindowMode })
        return
      }

      if (!shouldRedock(modeRef.current, cursorInsideWindowRef.current, composingRef.current, manualDragActiveRef.current, savingRef.current)) {
        sideHideDebugLog('scheduleRedock skipped: blocked by focus/inside', {
          reason,
          isEditorFocused: isEditorFocusedRef.current,
          cursorInside: cursorInsideWindowRef.current,
        })
        clearCloseTimer()
        return
      }

      clearCloseTimer()
      const delay = delayOverride ?? appConfig?.hoverCloseDelayMs ?? 320
      sideHideDebugLog('scheduleRedock armed', { reason, delay })
      closeTimerRef.current = window.setTimeout(async () => {
        closeTimerRef.current = null

        if (!shouldRedock(modeRef.current, cursorInsideWindowRef.current, composingRef.current, manualDragActiveRef.current, savingRef.current)) {
          sideHideDebugLog('scheduleRedock fire canceled by focus/inside', {
            reason,
            isEditorFocused: isEditorFocusedRef.current,
            cursorInside: cursorInsideWindowRef.current,
          })
          return
        }

        try {
          const isInside = await invoke<boolean>('is_cursor_inside_main_window', {
            paddingPx: cursorPaddingPx,
          })
          sideHideDebugLog('scheduleRedock fire recheck', { reason, isInside })
          if (!shouldRedock(modeRef.current, isInside, composingRef.current, manualDragActiveRef.current, savingRef.current)) {
            return
          }

          sideHideDebugLog('redock_main_window invoke', { reason })
          await invoke('redock_main_window')
        } catch (error) {
          console.error('Failed to redock window', error)
        }
      }, delay)
    },
    [appConfig?.hoverCloseDelayMs, clearCloseTimer, cursorPaddingPx, mainWindowMode],
  )

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
    if (draft.trim()) {
      return
    }

    const timer = window.setInterval(() => {
      setCreatedAt(formatTimestamp(new Date()))
    }, 60_000)

    return () => window.clearInterval(timer)
  }, [draft])

  useEffect(() => {
    let ignore = false
    let unlistenThemeChange: (() => void) | undefined
    let unlistenAppConfigChange: (() => void) | undefined
    let unlistenCustomThemePreviewChange: (() => void) | undefined
    let unlistenMainWindowModeChange: (() => void) | undefined
    let unlistenFocusMainEditor: (() => void) | undefined
    let unlistenNativeBackdrop: (() => void) | undefined

    const loadConfig = async () => {
      try {
        const [config, hotkeyStatus, restoredDraft, backdropStatus] = await Promise.all([
          invoke<AppConfig>('get_app_config'),
          invoke<HotkeyRuntimeStatus>('get_hotkey_runtime_status'),
          invoke<DraftSnapshot>('get_draft'),
          invoke<NativeBackdropStatus>('get_native_backdrop_status'),
        ])
        let resolvedConfig = config
        if (
          restoredDraft.text.trim() &&
          restoredDraft.targetId &&
          restoredDraft.targetId !== config.activeTargetId &&
          config.targets.some((target) => target.id === restoredDraft.targetId)
        ) {
          resolvedConfig = await invoke<AppConfig>('set_active_target_id', {
            activeTargetId: restoredDraft.targetId,
          })
        }
        if (!ignore) {
          setAppConfig(resolvedConfig)
          setThemeMode(resolvedConfig.themeMode)
          setLanguageMode(resolvedConfig.languageMode)
          setDraft(restoredDraft.text)
          setCreatedAt(restoredDraft.createdAt || formatTimestamp(new Date()))
          setIsDraftLoaded(true)
          setHasNativeBackdrop(backdropStatus.available)
          if (hotkeyStatus.warnings.length > 0) {
            setCaptureFeedback(hotkeyStatus.warnings.join(' '))
          }
        }
      } catch (error) {
        console.error('Failed to load app config', error)
      }
    }

    const attachThemeListener = async () => {
      unlistenThemeChange = await listen<ThemeModeChangedPayload>('theme-mode-changed', (event) => {
        if (!ignore) {
          setThemeMode(event.payload.themeMode)
        }
      })
    }

    const attachAppConfigListener = async () => {
      unlistenAppConfigChange = await listen<AppConfigChangedPayload>('app-config-changed', (event) => {
        if (!ignore) {
          setAppConfig(event.payload)
          setThemeMode(event.payload.themeMode)
          setLanguageMode(event.payload.languageMode)
          setCustomThemePreview(null)
        }
      })
    }

    const attachCustomThemePreviewListener = async () => {
      unlistenCustomThemePreviewChange = await listen<CustomThemePreviewChangedPayload>(
        'custom-theme-preview-changed',
        (event) => {
          if (!ignore) {
            setCustomThemePreview(event.payload.customTheme)
          }
        },
      )
    }

    const attachMainWindowModeListener = async () => {
      unlistenMainWindowModeChange = await listen<MainWindowModeChangedPayload>(
        'main-window-mode-changed',
        (event) => {
          if (!ignore) {
            modeRef.current = event.payload.mode
            setMainWindowMode(event.payload.mode)
            setDockSide(event.payload.dockSide ?? null)
          }
        },
      )
    }

    const attachFocusMainEditorListener = async () => {
      unlistenFocusMainEditor = await listen('focus-main-editor', () => {
        if (!ignore) {
          window.setTimeout(() => {
            textareaRef.current?.focus()
          }, 0)
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
      console.error('Failed to attach theme listener', error)
    })
    attachAppConfigListener().catch((error) => {
      console.error('Failed to attach app config listener', error)
    })
    attachCustomThemePreviewListener().catch((error) => {
      console.error('Failed to attach custom theme preview listener', error)
    })
    attachMainWindowModeListener().catch((error) => {
      console.error('Failed to attach main window mode listener', error)
    })
    attachFocusMainEditorListener().catch((error) => {
      console.error('Failed to attach main editor focus listener', error)
    })
    attachNativeBackdropListener().catch((error) => {
      console.error('Failed to attach native backdrop listener', error)
    })

    return () => {
      ignore = true
      unlistenThemeChange?.()
      unlistenAppConfigChange?.()
      unlistenCustomThemePreviewChange?.()
      unlistenMainWindowModeChange?.()
      unlistenFocusMainEditor?.()
      unlistenNativeBackdrop?.()
    }
  }, [])

  useEffect(() => {
    const previousMode = previousMainWindowModeRef.current

    if (mainWindowMode === 'docked-left' || mainWindowMode === 'docked-right') {
      manualDragActiveRef.current = false
    }

    if (mainWindowMode !== 'docked-left' && mainWindowMode !== 'docked-right' && openTimerRef.current) {
      window.clearTimeout(openTimerRef.current)
      openTimerRef.current = null
    }

    if (mainWindowMode !== 'expanded-from-dock' && closeTimerRef.current) {
      window.clearTimeout(closeTimerRef.current)
      closeTimerRef.current = null
    }

    if (mainWindowMode === 'expanded-from-dock') {
      cursorInsideWindowRef.current = true
      clearCloseTimer()

    }

    if (
      (mainWindowMode === 'docked-left' || mainWindowMode === 'docked-right') &&
      previousMode === 'expanded-from-dock'
    ) {
      dockReopenBlockedUntilRef.current = Date.now() + dockReopenCooldownMs
    }

    sideHideDebugLog('mainWindowMode changed', {
      previousMode,
      nextMode: mainWindowMode,
      dockSide,
    })

    previousMainWindowModeRef.current = mainWindowMode
  }, [clearCloseTimer, dockSide, mainWindowMode])

  useEffect(
    () => () => {
      if (openTimerRef.current) {
        window.clearTimeout(openTimerRef.current)
      }

      if (closeTimerRef.current) {
        window.clearTimeout(closeTimerRef.current)
      }
    },
    [],
  )

  useEffect(() => {
    if (nativeDockObserver || mainWindowMode !== 'expanded-from-dock') {
      cursorInsideWindowRef.current = false
      return
    }

    let cancelled = false
    let timeoutId: number | null = null

    const pollCursorInsideWindow = async () => {
      if (cancelled) {
        return
      }

      try {
        const isInside = await invoke<boolean>('is_cursor_inside_main_window', {
          paddingPx: cursorPaddingPx,
        })

        if (cancelled) {
          return
        }

        cursorInsideWindowRef.current = isInside
        sideHideDebugLog('expanded poll', {
          isInside,
          isEditorFocused: isEditorFocusedRef.current,
          hasCloseTimer: Boolean(closeTimerRef.current),
        })

        if (!shouldRedock(modeRef.current, isInside, composingRef.current, manualDragActiveRef.current, savingRef.current)) {
          clearCloseTimer()
        } else if (!closeTimerRef.current) {
          scheduleRedock('poll-outside')
        }
      } catch (error) {
        console.error('Failed to check cursor position against main window', error)
      } finally {
        if (!cancelled) {
          timeoutId = window.setTimeout(() => {
            void pollCursorInsideWindow()
          }, 120)
        }
      }
    }

    void pollCursorInsideWindow()

    return () => {
      cancelled = true
      if (timeoutId) {
        window.clearTimeout(timeoutId)
      }
    }
  }, [clearCloseTimer, mainWindowMode, scheduleRedock])

  useEffect(() => {
    const finalizeManualWindowDrag = () => {
      if (!manualDragActiveRef.current) {
        return
      }

      manualDragActiveRef.current = false
      sideHideDebugLog('manual drag ended from mouseup')
      void invoke('end_manual_window_drag').catch((error) => {
        console.error('Failed to finalize manual window drag state', error)
      })
    }

    window.addEventListener('mouseup', finalizeManualWindowDrag)
    return () => {
      window.removeEventListener('mouseup', finalizeManualWindowDrag)
    }
  }, [])

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

  const canSaveNote = isDraftLoaded && appConfig !== null && draft.trim().length > 0 && !isSavingNote
  const isDocked = mainWindowMode === 'docked-left' || mainWindowMode === 'docked-right'
  const shouldShowDebugHotzone = appConfig?.debugShowHotzone === 1 && isDocked
  const saveShortcutMode: SaveShortcutMode = appConfig?.saveShortcutMode ?? 'ctrl-enter-save'
  const saveShortcutLabel = saveShortcutMode === 'enter-save' ? 'Enter' : 'Ctrl+Enter'
  const customTheme = customThemePreview ?? appConfig?.customTheme ?? defaultCustomTheme
  const targets = appConfig?.targets ?? []
  const activeTargetId = appConfig?.activeTargetId ?? targets[0]?.id ?? ''
  const text = uiText[languageMode]
  const surfaceEffects = useMemo(() => defaultFocusEffects(resolvedTheme, customTheme.windowOpacity),
  [resolvedTheme, customTheme.windowOpacity])
  const tintColor = resolvedTheme === 'custom' ? customTheme.windowColor : resolvedTheme === 'dark' ? '#121215' : '#f6f6fb'

  useEffect(() => {
    let disposed = false
    const attach = listen<SurfaceState>(SURFACE_STATE_EVENT, ({ payload }) => {
      if (disposed) return
      setHasNativeBackdrop(payload.available)
      setIsWindowFocused(payload.focused)
      setSurfaceOpacity(payload.interfaceOpacity)
    })
    void attach.then(() => {
      if (disposed) return
      return invoke('update_main_surface', { parameters: {
        effects: surfaceEffects,
        color: surfaceColor(tintColor),
      } })
    }).catch(error => console.error('Failed to update native surface', error))
    return () => { disposed = true; void attach.then(unlisten => unlisten()) }
  }, [surfaceEffects, tintColor])

  useEffect(() => {
    if (!isDraftLoaded || savingRef.current) {
      return
    }
    const snapshot: DraftSnapshot = {
      text: draft,
      targetId: activeTargetId,
      createdAt,
      updatedAt: new Date().toISOString(),
    }
    void invoke('update_draft', { draft: snapshot }).catch((error) => {
      console.error('Failed to update draft snapshot', error)
    })
  }, [activeTargetId, createdAt, draft, isDraftLoaded])

  const flushCurrentDraft = async () => {
    const snapshot: DraftSnapshot = {
      text: draft,
      targetId: activeTargetId,
      createdAt,
      updatedAt: new Date().toISOString(),
    }
    await invoke('update_draft', { draft: snapshot })
    await invoke('flush_draft')
  }

  const hideWindow = async () => {
    clearOpenTimer()
    clearCloseTimer()
    try {
      await getCurrentWindow().hide()
    } catch (error) {
      console.error('Failed to hide window', error)
    }
  }

  const handleWindowDragStart = async (event: React.MouseEvent<HTMLElement>) => {
    if (event.button !== 0 || savingRef.current) {
      return
    }

    const target = event.target as HTMLElement | null
    if (target?.closest('button, textarea, input, select, [data-no-drag]')) {
      return
    }

    try {
      manualDragActiveRef.current = true
      sideHideDebugLog('manual drag started')
      void invoke('begin_manual_window_drag').catch((error) => {
        console.error('Failed to mark manual window drag as started', error)
        manualDragActiveRef.current = false
      })
      await getCurrentWindow().startDragging()
    } catch (error) {
      console.error('Failed to start window drag', error)
      manualDragActiveRef.current = false
      void invoke('end_manual_window_drag').catch((cleanupError) => {
        console.error('Failed to finalize manual window drag state', cleanupError)
      })
    }
  }

  const handleHide = async () => {
    if (savingRef.current) return
    setCaptureFeedback('')
    try {
      await flushCurrentDraft()
      await invoke('hide_or_dock_main_window')
    } catch (error) {
      console.error('Failed to hide or dock window', error)
      await hideWindow()
    }
  }

  const handleClose = async () => {
    if (savingRef.current) return
    setCaptureFeedback('')
    try {
      await flushCurrentDraft()
    } catch (error) {
      console.error('Failed to flush draft before closing', error)
    }
    await hideWindow()
  }

  const handleOpenSettings = async () => {
    if (savingRef.current) return
    clearOpenTimer()
    clearCloseTimer()
    try {
      await flushCurrentDraft()
      await invoke('open_settings_window')
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setCaptureFeedback(message)
    }
  }

  const handleTargetSelect = async (targetId: string) => {
    if (savingRef.current || !targetId || targetId === activeTargetId) {
      return
    }

    try {
      const config = await invoke<AppConfig>('set_active_target_id', {
        activeTargetId: targetId,
      })
      setAppConfig(config)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setCaptureFeedback(message)
    }
  }

  const handleSaveNote = async () => {
    if (!canSaveNote || savingRef.current || composingRef.current) {
      return
    }

    savingRef.current = true
    clearOpenTimer()
    clearCloseTimer()
    setIsSavingNote(true)
    setCaptureFeedback('')

    try {
      await invoke('set_capture_interaction', { saving: true })
      await invoke('save_note', { noteText: draft, targetId: activeTargetId })
      setDraft('')
      setCreatedAt(formatTimestamp(new Date()))
      setCaptureFeedback(text.savedFeedback)
      await invoke('finish_capture')
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setCaptureFeedback(message)
    } finally {
      void invoke('set_capture_interaction', { saving: false }).catch(console.error)
      savingRef.current = false
      setIsSavingNote(false)
    }
  }

  const insertEditorNewline = () => {
    const editor = textareaRef.current
    if (!editor) {
      setDraft((currentDraft) => `${currentDraft}\n`)
      return
    }

    const selectionStart = editor.selectionStart
    const selectionEnd = editor.selectionEnd
    setDraft((currentDraft) => {
      const nextDraft = `${currentDraft.slice(0, selectionStart)}\n${currentDraft.slice(selectionEnd)}`
      window.setTimeout(() => {
        editor.selectionStart = selectionStart + 1
        editor.selectionEnd = selectionStart + 1
      }, 0)
      return nextDraft
    })
  }

  const handleEditorKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (composingRef.current || event.nativeEvent.isComposing || event.nativeEvent.keyCode === 229 || savingRef.current) return
    if (event.key !== 'Enter' || event.shiftKey || event.altKey || event.metaKey) {
      return
    }

    if (saveShortcutMode === 'enter-save' && event.ctrlKey) {
      event.preventDefault()
      insertEditorNewline()
      return
    }

    if (!shouldSubmit({
      key: event.key, ctrlKey: event.ctrlKey, shiftKey: event.shiftKey,
      altKey: event.altKey, metaKey: event.metaKey,
      isComposing: event.nativeEvent.isComposing, keyCode: event.nativeEvent.keyCode,
    }, saveShortcutMode)) {
      return
    }

    event.preventDefault()
    void handleSaveNote()
  }

  const clearOpenTimer = () => {
    if (openTimerRef.current) {
      window.clearTimeout(openTimerRef.current)
      openTimerRef.current = null
    }
  }

  const handleDockHotzoneEnter = () => {
    if (nativeDockObserver) return
    if (!isDocked) {
      return
    }

    if (Date.now() < dockReopenBlockedUntilRef.current) {
      sideHideDebugLog('dock hotzone enter suppressed by cooldown', {
        blockedUntil: dockReopenBlockedUntilRef.current,
      })
      return
    }

    clearOpenTimer()
    const delay = appConfig?.hoverOpenDelayMs ?? 180
    sideHideDebugLog('dock hotzone enter', { delay })
    openTimerRef.current = window.setTimeout(async () => {
      openTimerRef.current = null
      try {
        sideHideDebugLog('restore_docked_main_window invoke')
        await invoke('restore_docked_main_window')
      } catch (error) {
        console.error('Failed to restore docked window', error)
      }
    }, delay)
  }

  const handleDockHotzoneLeave = () => {
    sideHideDebugLog('dock hotzone leave')
    clearOpenTimer()
  }

  const appShellStyle = {
    ...focusEffectVariables(surfaceEffects),
    '--capture-interface-opacity': surfaceOpacity,
    '--dock-visible-width': `${appConfig?.visibleHandleWidthPx ?? 22}px`,
    '--dock-hotzone-width': `${appConfig?.hotzoneWidthPx ?? 36}px`,
    '--empty-input-placeholder-color': appConfig?.emptyInputPlaceholderColor ?? '#8A8A8A',
    '--save-shortcut-text-color': appConfig?.saveShortcutTextColor ?? 'currentColor',
    '--save-shortcut-font-size': `${appConfig?.saveShortcutFontSizePx ?? 9}px`,
    '--custom-window-background': hexToRgba(customTheme.windowColor, customTheme.windowOpacity),
    '--custom-window-color': customTheme.windowColor,
    '--custom-text-color': customTheme.textColor,
    '--custom-accent-color': customTheme.accentColor,
  } as CSSProperties

  const isSurfaceFocused = isWindowFocused

  return (
    <main
      className={`app-shell capture-surface theme-${resolvedTheme} lang-${languageMode} ${
        hasNativeBackdrop ? 'backdrop-native backdrop-composition' : 'backdrop-fallback'
      } ${isSurfaceFocused ? 'window-focused' : 'window-unfocused'} ${
        isDocked ? `is-docked dock-${dockSide}` : ''
      } ${
        mainWindowMode === 'expanded-from-dock' ? 'is-expanded-from-dock' : ''
      }`}
      style={appShellStyle}
    >
      <section className={`floating-window ${isDocked ? 'is-docked' : ''}`} role="dialog" aria-label={text.dialogLabel}>
        {!isDocked && (['North', 'NorthEast', 'East', 'SouthEast', 'South', 'SouthWest', 'West', 'NorthWest'] as const).map(direction => (
          <div key={direction} className={`capture-resize-edge resize-${direction.toLowerCase()}`} aria-hidden="true"
            onMouseDown={event => {
              if (event.button !== 0) return
              event.preventDefault()
              event.stopPropagation()
              void getCurrentWindow().startResizeDragging(direction).catch(console.error)
            }} />
        ))}
        {isDocked ? (
          <div
            className={`dock-shell dock-shell-${dockSide ?? 'left'}`}
            onMouseEnter={handleDockHotzoneEnter}
            onMouseLeave={handleDockHotzoneLeave}
          >
            {shouldShowDebugHotzone ? <div className="dock-hotzone-debug" aria-hidden="true" /> : null}
            <div className="dock-handle" aria-hidden="true">
              <svg className="dock-handle-surface" viewBox="0 0 22 360" preserveAspectRatio="none">
                <defs>
                  <linearGradient id="dock-surface-fill" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0" stopColor="var(--dock-fill-top)" />
                    <stop offset="1" stopColor="var(--dock-fill-bottom)" />
                  </linearGradient>
                  <clipPath id="dock-clip-left" clipPathUnits="objectBoundingBox">
                    <path d="M0 0 L.55 .018 Q.977 .028 .977 .055 L.977 .945 Q.977 .972 .55 .982 L0 1 Z" />
                  </clipPath>
                  <clipPath id="dock-clip-right" clipPathUnits="objectBoundingBox">
                    <path d="M1 0 L.45 .018 Q.023 .028 .023 .055 L.023 .945 Q.023 .972 .45 .982 L1 1 Z" />
                  </clipPath>
                </defs>
                <path d="M0 0 L12.1 6.48 Q21.5 10.08 21.5 19.8 L21.5 340.2 Q21.5 349.92 12.1 353.52 L0 360 Z"
                  fill="url(#dock-surface-fill)" stroke="var(--dock-outline)" vectorEffect="non-scaling-stroke" />
              </svg>
              <span className="dock-handle-label">{text.dockLabel}</span>
            </div>
          </div>
        ) : (
          <>
            <header className="window-header">
              <div
                className="window-drag-area"
                data-tauri-drag-region
                onMouseDown={handleWindowDragStart}
              >
                <h1 data-tauri-drag-region>{text.appTitle}</h1>
              </div>
              <button
                className="icon-button close-button"
                type="button"
                aria-label={text.close}
                disabled={isSavingNote}
                onClick={handleClose}
              >
                &times;
              </button>
            </header>

            {targets.length > 1 ? (
              <div className="target-switcher" aria-label={text.markdownTargets}>
                {targets.map((target) => (
                  <button
                    key={target.id}
                    className={`target-chip ${target.id === activeTargetId ? 'is-selected' : ''}`}
                    type="button"
                    disabled={isSavingNote}
                    onClick={() => handleTargetSelect(target.id)}
                  >
                    {targetLabel(target)}
                  </button>
                ))}
              </div>
            ) : null}

            <div className="editor-shell">
              <textarea
                style={editorTypographyStyle(appConfig?.editorTypography, appConfig?.languageMode)}
                id="fleeting-note-editor"
                name="fleeting-note-editor"
                ref={textareaRef}
                aria-label={text.editorLabel}
                spellCheck={false}
                value={draft}
                readOnly={isSavingNote || !isDraftLoaded}
                onCompositionStart={() => {
                  composingRef.current = true
                  void invoke('set_capture_interaction', { composing: true }).catch(console.error)
                  clearCloseTimer()
                }}
                onCompositionEnd={() => {
                  composingRef.current = false
                  void invoke('set_capture_interaction', { composing: false }).catch(console.error)
                  scheduleRedock('composition-end')
                }}
                placeholder={text.placeholder}
                onFocus={() => {
                  isEditorFocusedRef.current = true
                  cursorInsideWindowRef.current = true
                  sideHideDebugLog('editor focus')
                  clearCloseTimer()
                }}
                onBlur={() => {
                  isEditorFocusedRef.current = false
                  sideHideDebugLog('editor blur', {
                    mainWindowMode,
                    cursorInside: cursorInsideWindowRef.current,
                  })
                  if (mainWindowMode === 'expanded-from-dock' && !cursorInsideWindowRef.current) {
                    scheduleRedock('editor-blur')
                  }
                }}
                onChange={(event) => {
                  setDraft(event.target.value)
                  if (captureFeedback) {
                    setCaptureFeedback('')
                  }
                }}
                onKeyDown={handleEditorKeyDown}
              />
            </div>

            <footer className="window-footer">
              <div className="window-meta">
                <time>{createdAt}</time>
                {captureFeedback ? <span className="feedback-text">{captureFeedback}</span> : null}
              </div>

              <div className="window-actions">
                <button
                  className="icon-button footer-icon-button settings-button"
                  type="button"
                  aria-label={text.settingsTitle}
                  title={text.settingsTitle}
                  disabled={isSavingNote}
                  onClick={handleOpenSettings}
                >
                  <span className="footer-action-icon settings-icon" aria-hidden="true" />
                </button>
                <button
                  className="icon-button footer-icon-button hide-button"
                  type="button"
                  aria-label={text.hide}
                  disabled={isSavingNote}
                  onClick={handleHide}
                >
                  <svg className="footer-action-icon ghost-icon" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                    <path d="M16 10.5C16 11.3284 15.5523 12 15 12C14.4477 12 14 11.3284 14 10.5C14 9.67157 14.4477 9 15 9C15.5523 9 16 9.67157 16 10.5Z" />
                    <ellipse cx="9" cy="10.5" rx="1" ry="1.5" />
                    <path d="M22 12.3006C22 6.61173 17.5228 2 12 2C6.47715 2 2 6.61173 2 12.3006V19.723C2 21.0453 3.35098 21.9054 4.4992 21.314C5.42726 20.836 6.5328 20.9069 7.39614 21.4998C8.36736 22.1667 9.63264 22.1667 10.6039 21.4998L10.9565 21.2576C11.5884 20.8237 12.4116 20.8237 13.0435 21.2576L13.3961 21.4998C14.3674 22.1667 15.6326 22.1667 16.6039 21.4998C17.4672 20.9069 18.5727 20.836 19.5008 21.314C20.649 21.9054 22 21.0453 22 19.723V16.0118" />
                  </svg>
                </button>
                <button className="save-button" type="button" disabled={!canSaveNote} onClick={handleSaveNote}>
                  <span className="save-button-label">{isSavingNote ? text.saving : text.save}</span>
                  <span className="save-button-shortcut">{saveShortcutLabel}</span>
                </button>
              </div>
            </footer>
          </>
        )}
      </section>
    </main>
  )
}

function formatTimestamp(date: Date) {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  const hours = String(date.getHours()).padStart(2, '0')
  const minutes = String(date.getMinutes()).padStart(2, '0')

  return `${year}-${month}-${day} ${hours}:${minutes}`
}

const defaultCustomTheme: CustomTheme = {
  windowColor: '#F8F8FF',
  windowOpacity: 0.86,
  textColor: '#333333',
  accentColor: '#3EB4BF',
}

function hexToRgba(hexColor: string, opacity: number) {
  const normalized = /^#[\da-f]{6}$/i.test(hexColor) ? hexColor : defaultCustomTheme.windowColor
  const red = Number.parseInt(normalized.slice(1, 3), 16)
  const green = Number.parseInt(normalized.slice(3, 5), 16)
  const blue = Number.parseInt(normalized.slice(5, 7), 16)
  const alpha = Number.isFinite(opacity) ? Math.min(Math.max(opacity, 0.35), 1) : defaultCustomTheme.windowOpacity

  return `rgba(${red}, ${green}, ${blue}, ${alpha})`
}

function targetLabel(target: MarkdownTarget) {
  if (target.nickname.trim()) {
    return target.nickname.trim()
  }

  const normalizedPath = target.path.replaceAll('\\', '/')
  const fileName = normalizedPath.split('/').filter(Boolean).pop()
  return fileName?.replace(/\.md$/i, '') || 'Target'
}

export default App
