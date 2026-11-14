import { beforeEach, describe, expect, it } from 'vitest'
import type { DetectedObject, ImageRecord } from '@/api/client'
import { useEditorStore } from './editorStore'

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
})
