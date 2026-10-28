import type { MainWindowMode, SaveShortcutMode } from './appModel.ts'

type SubmitKey = {
  key: string
  ctrlKey: boolean
  shiftKey: boolean
  altKey: boolean
  metaKey: boolean
  isComposing: boolean
  keyCode: number
}

export function shouldSubmit(event: SubmitKey, mode: SaveShortcutMode) {
  if (event.isComposing || event.keyCode === 229 || event.key !== 'Enter' ||
      event.shiftKey || event.altKey || event.metaKey) return false
  return mode === 'enter-save' ? !event.ctrlKey : event.ctrlKey
}

export function shouldRedock(
  mode: MainWindowMode, pointerInside: boolean, composing: boolean, dragging: boolean, saving: boolean,
) {
  return mode === 'expanded-from-dock' && !pointerInside && !composing && !dragging && !saving
}
