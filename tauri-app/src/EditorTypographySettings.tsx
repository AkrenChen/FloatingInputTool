import { useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { AppConfig, EditorTypography, LanguageMode } from './appModel'
import { defaultEditorTypography, editorTypographyStyle, suggestedFonts } from './editorTypography'

export function EditorTypographySettings({ language }: { language: LanguageMode }) {
  const zh = language === 'chinese'
  const [draft, setDraft] = useState(defaultEditorTypography)
  const [saved, setSaved] = useState(defaultEditorTypography)
  const [fontName, setFontName] = useState('')
  const [sizeInput, setSizeInput] = useState('15')
  const [adding, setAdding] = useState(false)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [feedback, setFeedback] = useState('')
  const [error, setError] = useState(false)
  const dirty = useRef(false)

  useEffect(() => {
    let active = true
    let unlisten: (() => void) | undefined
    const apply = (config: AppConfig) => {
      if (!active) return
      const value = config.editorTypography ?? defaultEditorTypography
      setSaved(value)
      if (!dirty.current) { setDraft(value); setSizeInput(String(value.fontSizePx)) }
    }
    void (async () => {
      try {
        const stop = await listen<AppConfig>('app-config-changed', event => apply(event.payload))
        if (!active) { stop(); return }
        unlisten = stop
        apply(await invoke<AppConfig>('get_app_config'))
        if (active) setLoading(false)
      } catch (reason) {
        if (active) { setFeedback(String(reason)); setError(true) }
      }
    })()
    return () => { active = false; unlisten?.() }
  }, [])

  const update = (patch: Partial<EditorTypography>) => {
    dirty.current = true
    setDraft(current => ({ ...current, ...patch }))
    setFeedback('')
  }
  const fonts = [...new Set([...suggestedFonts, ...draft.customFonts, draft.fontFamily].filter(Boolean))]
  const addFont = () => {
    const name = fontName.trim()
    if (!name) return
    const existing = fonts.find(font => font.toLowerCase() === name.toLowerCase())
    update({
      fontFamily: existing ?? name,
      customFonts: existing ? draft.customFonts : [...draft.customFonts, name],
    })
    setFontName('')
    setAdding(false)
  }
  const save = async () => {
    setSaving(true)
    setFeedback('')
    try {
      const config = await invoke<AppConfig>('set_editor_typography', { editorTypography: draft })
      dirty.current = false
      setDraft(config.editorTypography)
      setSizeInput(String(config.editorTypography.fontSizePx))
      setSaved(config.editorTypography)
      setError(false)
      setFeedback(zh ? '正文样式已保存。' : 'Editor style saved.')
    } catch (reason) {
      setError(true)
      setFeedback(String(reason))
    } finally { setSaving(false) }
  }

  return <section className="settings-section typography-settings">
    <span className="settings-label">{zh ? '正文样式' : 'Editor typography'}</span>
    <span className="settings-inline-value">{zh ? '仅影响输入框显示，不改变保存的 Markdown 内容。' : 'Only changes the editor display, not the saved Markdown.'}</span>
    <fieldset disabled={loading || saving} className="typography-fields">
      <div className="typography-preview" style={editorTypographyStyle(draft, language)} aria-label={zh ? '正文预览' : 'Editor preview'}>
        记录一闪而过的灵感。<br />Capture a fleeting thought. 0123456789
      </div>
      <div className="typography-row">
        <label className="typography-font">
          <span className="settings-label">{zh ? '字体' : 'Font'}</span>
          <select className="settings-input" value={draft.fontFamily} onChange={event => update({ fontFamily: event.target.value })}>
            <option value="">{zh ? '沿用当前字体' : 'Current default'}</option>
            {fonts.map(font => <option key={font} value={font}>{font}</option>)}
          </select>
        </label>
        <label className="typography-size">
          <span className="settings-label">{zh ? '字号（px）' : 'Size (px)'}</span>
          <input className="settings-input" type="number" min={10} max={48} step={1}
            value={sizeInput} onChange={event => {
              setSizeInput(event.target.value)
              const size = event.target.valueAsNumber
              if (Number.isInteger(size) && size >= 10 && size <= 48) update({ fontSizePx: size })
            }} onBlur={() => {
              const size = Math.min(48, Math.max(10, Math.round(Number(sizeInput) || 15)))
              setSizeInput(String(size))
              if (size !== draft.fontSizePx) update({ fontSizePx: size })
            }} />
        </label>
      </div>
      <button type="button" className="settings-text-button" aria-expanded={adding} onClick={() => setAdding(!adding)}>{zh ? '添加字体' : 'Add font'}</button>
      {adding && <div className="typography-add">
        <input className="settings-input" aria-label={zh ? '已安装的字体名称' : 'Installed font name'}
          placeholder={zh ? '输入已安装的字体名称' : 'Enter an installed font name'} value={fontName}
          onChange={event => setFontName(event.target.value)}
          onKeyDown={event => { if (event.key === 'Enter') { event.preventDefault(); addFont() } }} />
        <button type="button" className="settings-save-button" disabled={!fontName.trim()} onClick={addFont}>{zh ? '加入候选' : 'Add to list'}</button>
      </div>}
      <div className="typography-toggles">
        <label><input type="checkbox" checked={draft.bold} onChange={event => update({ bold: event.target.checked })} />{zh ? '固定粗体' : 'Always bold'}</label>
        <label><input type="checkbox" checked={draft.italic} onChange={event => update({ italic: event.target.checked })} />{zh ? '固定斜体' : 'Always italic'}</label>
      </div>
      <div className="settings-actions">
        <button type="button" className="settings-save-button" onClick={() => void save()}>{saving ? (zh ? '保存中…' : 'Saving…') : (zh ? '保存正文样式' : 'Save editor style')}</button>
        <button type="button" className="settings-save-button is-secondary" onClick={() => {
          dirty.current = false; setDraft(saved); setSizeInput(String(saved.fontSizePx)); setAdding(false); setFontName(''); setFeedback('')
        }}>{zh ? '取消修改' : 'Cancel changes'}</button>
      </div>
    </fieldset>
    <span className={`settings-inline-value ${error ? 'settings-feedback is-error' : ''}`} role="status">{feedback}</span>
  </section>
}
