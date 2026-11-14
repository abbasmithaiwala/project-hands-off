import { beforeEach, describe, expect, it } from 'vitest'
import { currentHint, markHintDone, readDoneHints, type HintContext } from './hints'

const context = (overrides: Partial<HintContext> = {}): HintContext => ({
  hasObjects: true,
  hasSelection: false,
  hasActiveWidget: false,
  hasPendingEdits: false,
  ...overrides,
})

describe('currentHint', () => {
  it('suggests picking an object once objects are found', () => {
    expect(currentHint(context())).toBe('pick-object')
    expect(currentHint(context({ hasObjects: false }))).toBeNull()
  })

  it('suggests dragging once a control is showing', () => {
    expect(currentHint(context({ hasSelection: true, hasActiveWidget: true }))).toBe('drag-handle')
  })

  it('stays quiet while a selection has no control yet, and once edits exist', () => {
    expect(currentHint(context({ hasSelection: true }))).toBeNull()
    expect(currentHint(context({ hasActiveWidget: true, hasPendingEdits: true }))).toBeNull()
  })
})

describe('remembering finished tips', () => {
  beforeEach(() => window.localStorage.clear())

  it('keeps what was finished and ignores junk in storage', () => {
    expect(readDoneHints().size).toBe(0)
    markHintDone('pick-object')
    markHintDone('drag-handle')
    expect([...readDoneHints()].sort()).toEqual(['drag-handle', 'pick-object'])
    window.localStorage.setItem('hands-off.hints-done', '{"nope":1}')
    expect(readDoneHints().size).toBe(0)
    window.localStorage.setItem('hands-off.hints-done', 'not json')
    expect(readDoneHints().size).toBe(0)
  })
})
