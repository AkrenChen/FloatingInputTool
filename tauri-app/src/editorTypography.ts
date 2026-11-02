import type { CSSProperties } from 'react'
import type { EditorTypography, LanguageMode } from './appModel'

export const defaultEditorTypography: EditorTypography = {
  fontFamily: '', fontSizePx: 15, bold: false, italic: false, customFonts: [],
}

export const suggestedFonts = [
  'Microsoft YaHei', 'Microsoft JhengHei', 'SimSun', 'SimHei', 'KaiTi',
  'DengXian', 'Segoe UI', 'Arial', 'Consolas', 'Times New Roman',
]

export function editorTypographyStyle(
  value: EditorTypography = defaultEditorTypography,
  language: LanguageMode = 'english',
): CSSProperties {
  const fallback = language === 'chinese'
    ? 'SimHei, "Microsoft YaHei", "Noto Sans SC", sans-serif'
    : undefined
  return {
    fontFamily: value.fontFamily
      ? `"${value.fontFamily.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}", ${fallback ?? 'sans-serif'}`
      : fallback,
    fontSize: `${value.fontSizePx}px`,
    fontWeight: value.bold ? 700 : 400,
    fontStyle: value.italic ? 'italic' : 'normal',
    fontSynthesis: 'weight style',
  }
}
