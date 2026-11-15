import { beforeEach, describe, expect, it } from 'vitest'
import { currentHint, hintText, markHintDone, readDoneHints, type HintContext } from './hints'

const context = (overrides: Partial<HintContext> = {}): HintContext => ({
  hasObjects: true,
  hasSelection: false,
  hasActiveWidget: false,
  hasPendingEdits: false,
  widgetKind: null,
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

describe('hintText', () => {
  it('describes what dragging does for the control that is showing', () => {
    expect(hintText('drag-handle', 'rotate')).toMatch(/around the ring to turn/)
    expect(hintText('drag-handle', 'scale')).toMatch(/bigger or smaller/)
    expect(hintText('drag-handle', 'move')).toMatch(/to move it/)
    expect(hintText('drag-handle', 'slider')).toMatch(/stretch/)
    expect(hintText('drag-handle', 'control_points')).toMatch(/reshape/)
    expect(hintText('drag-handle', 'multi_handle')).toMatch(/pose/)
  })

  it('always says nothing is generated until Apply, and has a fallback without a control', () => {
    expect(hintText('drag-handle', 'rotate')).toMatch(/until you press Apply/)
    expect(hintText('drag-handle', null)).toMatch(
      /^Drag the handle to change it\. Nothing is generated/,
    )
    expect(hintText('pick-object', null)).toMatch(/click it/)
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
