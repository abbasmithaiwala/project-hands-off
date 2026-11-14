import { beforeEach, describe, expect, it } from 'vitest'
import type { DetectedObject, ImageRecord } from '@/api/client'
import { editKey, useEditorStore, type PendingEdit } from './editorStore'

const image = { id: 'img-1', width: 100, height: 80 } as ImageRecord

function object(id: number): DetectedObject {
  return {
    id,
    label: `thing ${id}`,
    score: 0.9,
    bbox: { x: 0, y: 0, width: 1, height: 1 },
    outline: [],
  }
}

function pending(objectId: number, property: string, value = 0.8): PendingEdit {
  return {
    key: editKey(objectId, property),
    objectId,
    objectLabel: `thing ${objectId}`,
    propertyLabel: property,
    widget: {
      kind: 'slider',
      axis: { start: { x: 0, y: 0 }, end: { x: 1, y: 0 } },
      min: 0.5,
      max: 1.5,
      initial: 1,
    },
    edit: { object_id: objectId, property, change: { kind: 'slider', value } },
  }
}

const state = () => useEditorStore.getState()

beforeEach(() => state().reset())

describe('editor store', () => {
  it('starts empty', () => {
    expect(state()).toMatchObject({ image: null, objects: [], analysis: 'idle', selectedId: null })
  })

  it('opening an image clears the previous session but keeps the output panel preference', () => {
    state().toggleOutput()
    state().openImage(image)
    state().addObject(object(0))
    state().select(0)
    state().openImage({ ...image, id: 'img-2' })
    expect(state()).toMatchObject({ outputOpen: true, objects: [], selectedId: null })
    expect(state().image?.id).toBe('img-2')
  })

  it('ignores duplicate objects', () => {
    state().addObject(object(1))
    state().addObject(object(1))
    expect(state().objects).toHaveLength(1)
  })

  it('starting an analysis clears earlier objects and the selection', () => {
    state().addObject(object(1))
    state().select(1)
    state().setAnalysis('running')
    expect(state()).toMatchObject({ objects: [], selectedId: null, analysis: 'running' })
  })

  it('records analysis failures with their message', () => {
    state().setAnalysis('failed', 'model missing')
    expect(state()).toMatchObject({ analysis: 'failed', analysisError: 'model missing' })
    state().setAnalysis('done')
    expect(state().analysisError).toBeNull()
  })

  it('cycles the selection through objects', () => {
    ;[0, 1, 2].forEach((id) => state().addObject(object(id)))
    state().selectNext(1)
    expect(state().selectedId).toBe(0)
    state().selectNext(1)
    state().selectNext(1)
    state().selectNext(1)
    expect(state().selectedId).toBe(0)
    state().selectNext(-1)
    expect(state().selectedId).toBe(2)
  })

  it('forgets the active property whenever the selection changes', () => {
    ;[0, 1].forEach((id) => state().addObject(object(id)))
    state().select(0)
    state().setActiveProperty('length')
    expect(state().activeProperty).toBe('length')
    state().select(1)
    expect(state().activeProperty).toBeNull()
    state().setActiveProperty('shape')
    state().selectNext(1)
    expect(state().activeProperty).toBeNull()
  })

  it('keeps only the most recent log lines', () => {
    for (let i = 0; i < 520; i++) state().appendLog(`line ${i}`)
    expect(state().log).toHaveLength(500)
    expect(state().log.at(-1)).toBe('line 519')
  })

  it('tracks hover independently of selection', () => {
    state().hover(3)
    state().select(4)
    expect(state()).toMatchObject({ hoveredId: 3, selectedId: 4 })
  })

  it('keeps one pending edit per object and property, replacing it as the user drags', () => {
    state().setPendingEdit(pending(0, 'length', 0.9))
    state().setPendingEdit(pending(0, 'length', 0.7))
    state().setPendingEdit(pending(0, 'height'))
    state().setPendingEdit(pending(1, 'length'))
    expect(state().pendingEdits.map((e) => e.key)).toEqual(['0:length', '0:height', '1:length'])
    expect(state().pendingEdits[0]?.edit.change).toEqual({ kind: 'slider', value: 0.7 })
  })

  it('resets one edit or all of them', () => {
    state().setPendingEdit(pending(0, 'length'))
    state().setPendingEdit(pending(0, 'height'))
    state().resetEdit('0:length')
    expect(state().pendingEdits.map((e) => e.key)).toEqual(['0:height'])
    state().clearEdits()
    expect(state().pendingEdits).toEqual([])
  })

  it('drops pending edits when a new image or analysis starts', () => {
    state().setPendingEdit(pending(0, 'length'))
    state().setAnalysis('running')
    expect(state().pendingEdits).toEqual([])
    state().setPendingEdit(pending(0, 'length'))
    state().openImage(image)
    expect(state().pendingEdits).toEqual([])
  })

  it('builds stable keys', () => {
    expect(editKey(3, 'arm_position')).toBe('3:arm_position')
  })

  it('moves through the apply states', () => {
    expect(state().apply).toEqual({ status: 'idle' })
    state().beginApply()
    expect(state().apply).toMatchObject({ status: 'running', jobId: null, stage: 'analysing' })
    state().setApplyJob('job-1')
    state().setApplyStage('generating')
    expect(state().apply).toMatchObject({ status: 'running', jobId: 'job-1', stage: 'generating' })

    const result = {
      summary: 's',
      changes: [],
      instruction: 'i',
      model: 'm',
      cost_usd: 0.1,
      generation_ms: 10,
    }
    state().finishApply('job-1', result)
    expect(state().apply).toEqual({ status: 'review', jobId: 'job-1', result })

    state().closeApply()
    expect(state().apply).toEqual({ status: 'idle' })
  })

  it('ignores job and stage updates when nothing is running', () => {
    state().setApplyJob('late')
    state().setApplyStage('generating')
    expect(state().apply).toEqual({ status: 'idle' })
  })

  it('forgets an apply in progress when a new image opens', () => {
    state().beginApply()
    state().openImage(image)
    expect(state().apply).toEqual({ status: 'idle' })
  })
})
