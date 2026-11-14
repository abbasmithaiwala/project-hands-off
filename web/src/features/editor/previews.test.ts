import { describe, expect, it } from 'vitest'
import type { EditChange, Widget } from '@/api/client'
import { editKey, type PendingEdit } from '@/state/editorStore'
import { detectedObject } from '@/test/utils'
import { buildPreviews, deformationFor, reachFor } from './previews'

const size = { width: 1000, height: 500 }
const bbox = { x: 0.2, y: 0.2, width: 0.5, height: 0.5 }

const slider: Widget = {
  kind: 'slider',
  axis: { start: { x: 0.2, y: 0.5 }, end: { x: 0.7, y: 0.5 } },
  min: 0.5,
  max: 1.5,
  initial: 1,
}

const handles: Widget = {
  kind: 'multi_handle',
  handles: [
    { x: 0.3, y: 0.3 },
    { x: 0.6, y: 0.6 },
  ],
}

function pending(objectId: number, widget: Widget, change: EditChange): PendingEdit {
  return {
    key: editKey(objectId, 'p'),
    objectId,
    objectLabel: 'thing',
    propertyLabel: 'P',
    widget,
    edit: { object_id: objectId, property: 'p', change },
  }
}

describe('deformationFor', () => {
  it('stretches along the slider axis in pixel space', () => {
    const deform = deformationFor(slider, { kind: 'slider', value: 0.5 }, size, bbox)
    // Axis runs from x=200 to x=700 at y=250; halving it moves the far end to x=450.
    expect(deform?.({ x: 700, y: 250 })).toEqual({ x: 450, y: 250 })
    expect(deform?.({ x: 200, y: 250 })).toEqual({ x: 200, y: 250 })
  })

  it('moves dragged handles exactly to their targets', () => {
    const change: EditChange = {
      kind: 'multi_handle',
      origin: [
        { x: 0.3, y: 0.3 },
        { x: 0.6, y: 0.6 },
      ],
      target: [
        { x: 0.4, y: 0.3 },
        { x: 0.6, y: 0.6 },
      ],
    }
    const deform = deformationFor(handles, change, size, bbox)
    const moved = deform?.({ x: 300, y: 150 })
    expect(moved?.x).toBeCloseTo(400)
    expect(moved?.y).toBeCloseTo(150)
  })

  it('does nothing for unchanged or mismatched edits', () => {
    expect(deformationFor(slider, { kind: 'slider', value: 1 }, size, bbox)).toBeNull()
    expect(deformationFor(handles, { kind: 'slider', value: 0.7 }, size, bbox)).toBeNull()
  })
})

describe('buildPreviews', () => {
  const objects = [detectedObject(0, 'couch'), detectedObject(1, 'lamp')]
  const maskUrl = (id: number) => `/mask/${id}`

  it('creates one preview per edited object and none for untouched ones', () => {
    const previews = buildPreviews(
      [pending(1, slider, { kind: 'slider', value: 0.7 })],
      objects,
      size,
      maskUrl,
    )
    expect(previews.map((p) => p.objectId)).toEqual([1])
    expect(previews[0]?.spec.maskUrl).toBe('/mask/1')
    expect(previews[0]?.spec.bbox).toEqual(objects[1]?.bbox)
  })

  it('combines several edits on one object in order', () => {
    const edits = [
      pending(0, slider, { kind: 'slider', value: 0.5 }),
      { ...pending(0, slider, { kind: 'slider', value: 0.5 }), key: editKey(0, 'q') },
    ]
    const [preview] = buildPreviews(edits, objects, size, maskUrl)
    // Two halvings: the far end of the axis ends up a quarter of the way along.
    expect(preview?.spec.deform?.({ x: 700, y: 250 }).x).toBeCloseTo(325)
  })

  it('ignores edits that were dragged back to where they started', () => {
    expect(
      buildPreviews([pending(0, slider, { kind: 'slider', value: 1 })], objects, size, maskUrl),
    ).toEqual([])
  })

  it('returns nothing without edits', () => {
    expect(buildPreviews([], objects, size, maskUrl)).toEqual([])
  })
})

describe('reachFor', () => {
  it('is a fraction of the object diagonal in pixels', () => {
    expect(reachFor(detectedObject(0, 'x'), { width: 300, height: 400 })).toBeCloseTo(
      0.6 * Math.hypot(150, 200),
    )
  })
})
