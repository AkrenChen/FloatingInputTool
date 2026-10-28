import assert from 'node:assert/strict'
import test from 'node:test'
import { shouldSubmit, shouldRedock } from './captureBehavior.ts'

const enter = { key: 'Enter', ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, isComposing: false, keyCode: 13 }

test('Chinese candidate confirmation never submits', () => {
  assert.equal(shouldSubmit({ ...enter, isComposing: true }, 'enter-save'), false)
  assert.equal(shouldSubmit({ ...enter, keyCode: 229 }, 'enter-save'), false)
})

test('submission follows Enter and Ctrl+Enter modes without consuming Shift+Enter', () => {
  assert.equal(shouldSubmit(enter, 'enter-save'), true)
  assert.equal(shouldSubmit(enter, 'ctrl-enter-save'), false)
  assert.equal(shouldSubmit({ ...enter, ctrlKey: true }, 'ctrl-enter-save'), true)
  assert.equal(shouldSubmit({ ...enter, ctrlKey: true }, 'enter-save'), false)
  assert.equal(shouldSubmit({ ...enter, shiftKey: true }, 'enter-save'), false)
})

test('pointer leaving collapses hover expansion unless composing, saving, or dragging', () => {
  assert.equal(shouldRedock('expanded-from-dock', false, false, false, false), true)
  assert.equal(shouldRedock('normal', false, false, false, false), false)
  assert.equal(shouldRedock('expanded-from-dock', true, false, false, false), false)
  assert.equal(shouldRedock('expanded-from-dock', false, true, false, false), false)
  assert.equal(shouldRedock('expanded-from-dock', false, false, true, false), false)
  assert.equal(shouldRedock('expanded-from-dock', false, false, false, true), false)
})
