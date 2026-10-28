import type { CSSProperties } from 'react'
import type { ResolvedTheme } from './appModel'
import surfaceDefaults from './surfaceTheme.json'

export const SURFACE_STATE_EVENT = 'main-surface-state-changed'
export type FocusEffects = typeof surfaceDefaults
export type SurfaceState = {
  available: boolean
  focused: boolean
  active: boolean
  interfaceOpacity: number
  inset: number
}

export function defaultFocusEffects(theme: ResolvedTheme, customOpacity: number): FocusEffects {
  return {
    ...surfaceDefaults,
    focusedTint: theme === 'custom' ? customOpacity : theme === 'dark' ? 0.65 : surfaceDefaults.focusedTint,
    idleTint: theme === 'custom' ? customOpacity * 0.4 : theme === 'dark' ? 0.45 : surfaceDefaults.idleTint,
  }
}

export function focusEffectVariables(effects: FocusEffects): CSSProperties {
  return { '--window-radius': `${effects.cornerRadius}px` } as CSSProperties
}

export function surfaceColor(hex: string): [number, number, number] {
  return [1, 3, 5].map(offset => Number.parseInt(hex.slice(offset, offset + 2), 16)) as [number, number, number]
}
