import assert from 'node:assert/strict'
import test from 'node:test'
import { mergeSettingsDraft } from './settingsDraft.ts'

test('saving another section preserves an unfinished template edit', () => {
  const edited = { noteTemplate: 'unsaved text', themeMode: 'theme-white', targets: ['editing'] }
  const incoming = { noteTemplate: 'old template', themeMode: 'theme-dark', targets: ['saved'] }
  const actual = mergeSettingsDraft(edited, incoming, new Set<keyof typeof edited>(['noteTemplate', 'targets']))
  assert.deepEqual(actual, { noteTemplate: 'unsaved text', themeMode: 'theme-dark', targets: ['editing'] })
})

test('a clean section follows new persisted config', () => {
  assert.deepEqual(mergeSettingsDraft({ text: 'old' }, { text: 'new' }, new Set()), { text: 'new' })
})
