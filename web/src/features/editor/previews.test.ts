import { describe, expect, it } from 'vitest'
import type { EditChange, Widget } from '@/api/client'
import { editKey, type PendingEdit } from '@/state/editorStore'
import { detectedObject } from '@/test/utils'
import {
  affectedRegions,
  buildPreviews,
  deformationFor,
  outlineShapes,
  reachFor,
  referenceKind,
  requestEdits,
} from './previews'

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

const tilt: Widget = {
  kind: 'rotate',
  pivot: { x: 0.5, y: 0.8 },
  handle: { x: 0.5, y: 0.2 },
  min: -45,
  max: 45,
}

const grow: Widget = {
  kind: 'scale',
  axis: { start: { x: 0.4, y: 0.4 }, end: { x: 0.7, y: 0.2 } },
  min: 0.5,
  max: 2,
  initial: 1,
}

const carry: Widget = { kind: 'move', handle: { x: 0.45, y: 0.45 } }

describe('deformationFor', () => {
  it('turns the object clockwise about the pivot in pixel space', () => {
    const pivot = { x: 0.5, y: 0.8 }
    const deform = deformationFor(tilt, { kind: 'rotate', degrees: 90, pivot }, size, bbox)
    // The pivot is at (500, 400). A point 100px straight above it swings 100px to the right.
    const moved = deform?.({ x: 500, y: 300 })
    expect(moved?.x).toBeCloseTo(600)
    expect(moved?.y).toBeCloseTo(400)
    const fixed = deform?.({ x: 500, y: 400 })
    expect(fixed?.x).toBeCloseTo(500)
    expect(fixed?.y).toBeCloseTo(400)
  })

  it('resizes evenly about the anchor in both directions', () => {
    const deform = deformationFor(
      grow,
      { kind: 'scale', value: 1.5, pivot: { x: 0.4, y: 0.4 } },
      size,
      bbox,
    )
    // The anchor is at (400, 200): a point 100 right and 50 below it moves to 150 and 75.
    expect(deform?.({ x: 500, y: 250 })).toEqual({ x: 550, y: 275 })
    expect(deform?.({ x: 400, y: 200 })).toEqual({ x: 400, y: 200 })
  })

  it('carries the object by the same share of the image width and height', () => {
    const deform = deformationFor(carry, { kind: 'move', dx: 0.1, dy: -0.2 }, size, bbox)
    expect(deform?.({ x: 300, y: 300 })).toEqual({ x: 400, y: 200 })
  })

  it('ignores transforms left at their starting value', () => {
    const pivot = { x: 0.5, y: 0.5 }
    expect(deformationFor(tilt, { kind: 'rotate', degrees: 0, pivot }, size, bbox)).toBeNull()
    expect(deformationFor(grow, { kind: 'scale', value: 1, pivot }, size, bbox)).toBeNull()
    expect(deformationFor(carry, { kind: 'move', dx: 0, dy: 0 }, size, bbox)).toBeNull()
  })

  it('refuses a change that does not belong to the widget', () => {
    expect(deformationFor(slider, { kind: 'move', dx: 0.1, dy: 0 }, size, bbox)).toBeNull()
    expect(
      deformationFor(carry, { kind: 'rotate', degrees: 20, pivot: { x: 0.5, y: 0.5 } }, size, bbox),
    ).toBeNull()
  })

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

  it('moves pixels for sliders but only the border for reshaping controls', () => {
    const [stretch] = buildPreviews(
      [pending(0, slider, { kind: 'slider', value: 0.7 })],
      objects,
      size,
      maskUrl,
    )
    expect(stretch?.spec.showPixels).toBe(true)

    const [reshape] = buildPreviews(
      [
        pending(0, handles, {
          kind: 'multi_handle',
          origin: [{ x: 0.3, y: 0.3 }],
          target: [{ x: 0.35, y: 0.3 }],
        }),
      ],
      objects,
      size,
      maskUrl,
    )
    expect(reshape?.spec.showPixels).toBe(false)
    expect(reshape?.spec.deform).not.toBeNull()
  })

  it('moves pixels for every control that carries the object as a whole', () => {
    const pivot = { x: 0.5, y: 0.8 }
    for (const edit of [
      pending(0, tilt, { kind: 'rotate', degrees: 20, pivot }),
      pending(0, grow, { kind: 'scale', value: 1.3, pivot }),
      pending(0, carry, { kind: 'move', dx: 0.1, dy: 0 }),
    ]) {
      const [preview] = buildPreviews([edit], objects, size, maskUrl)
      expect(preview?.spec.showPixels, edit.widget.kind).toBe(true)
    }
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

describe('requestEdits', () => {
  it('adds the slider axis so the edit can be described in words', () => {
    const [edit] = requestEdits([pending(0, slider, { kind: 'slider', value: 0.7 })])
    expect(edit?.change).toEqual({
      kind: 'slider',
      value: 0.7,
      axis: { start: { x: 0.2, y: 0.5 }, end: { x: 0.7, y: 0.5 } },
    })
  })

  it('leaves point edits untouched', () => {
    const change: EditChange = {
      kind: 'multi_handle',
      origin: [{ x: 0.3, y: 0.3 }],
      target: [{ x: 0.4, y: 0.3 }],
    }
    expect(requestEdits([pending(0, handles, change)])[0]?.change).toEqual(change)
  })
})

describe('affectedRegions', () => {
  const objects = [detectedObject(0, 'couch'), detectedObject(1, 'lamp')]

  it('returns one region per edited object, covering the object', () => {
    const regions = affectedRegions(
      [pending(1, slider, { kind: 'slider', value: 0.5 })],
      objects,
      size,
    )
    expect(regions).toHaveLength(1)
    expect(regions[0]?.x).toBeLessThanOrEqual(objects[1]?.bbox.x ?? 1)
  })

  it('is empty when nothing has really changed', () => {
    expect(
      affectedRegions([pending(0, slider, { kind: 'slider', value: 1 })], objects, size),
    ).toEqual([])
    expect(affectedRegions([], objects, size)).toEqual([])
  })
})

describe('referenceKind and outlineShapes', () => {
  const points: Widget = {
    kind: 'control_points',
    points: [
      { x: 0.2, y: 0.2 },
      { x: 0.6, y: 0.2 },
      { x: 0.4, y: 0.6 },
    ],
  }
  const reshape = (): PendingEdit =>
    pending(0, points, {
      kind: 'control_points',
      origin: [
        { x: 0.2, y: 0.2 },
        { x: 0.6, y: 0.2 },
        { x: 0.4, y: 0.6 },
      ],
      target: [
        { x: 0.1, y: 0.2 },
        { x: 0.7, y: 0.2 },
        { x: 0.4, y: 0.6 },
      ],
    })
  const stretch = (): PendingEdit => ({
    ...pending(1, slider, { kind: 'slider', value: 0.7 }),
    key: editKey(1, 'q'),
  })

  it('draws the outline when every edit reshapes with control points', () => {
    expect(referenceKind([reshape()])).toBe('outline')
  })

  it('falls back to the warped preview for sliders, handles and mixed edits', () => {
    expect(referenceKind([stretch()])).toBe('warped')
    expect(referenceKind([reshape(), stretch()])).toBe('warped')
    expect(referenceKind([])).toBe('warped')
  })

  it('collects the target points of each reshaped object', () => {
    const [shape] = outlineShapes([reshape(), stretch()])
    expect(shape).toEqual([
      { x: 0.1, y: 0.2 },
      { x: 0.7, y: 0.2 },
      { x: 0.4, y: 0.6 },
    ])
    expect(outlineShapes([stretch()])).toEqual([])
  })
})
