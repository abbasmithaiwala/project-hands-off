import { describe, expect, it } from 'vitest'
import type { EditChange, Widget } from '@/api/client'
import {
  constrainPoint,
  describeChange,
  initialPoints,
  isUnchanged,
  sliderHandlePoint,
  sliderValueAt,
} from './widgetMath'

const size = { width: 1000, height: 500 }

const slider: Extract<Widget, { kind: 'slider' }> = {
  kind: 'slider',
  axis: { start: { x: 0.2, y: 0.5 }, end: { x: 0.6, y: 0.5 } },
  min: 0.5,
  max: 1.5,
  initial: 1,
}

describe('slider handle', () => {
  it('rests at the axis end for the initial value', () => {
    expect(sliderHandlePoint(slider, 1, size)).toEqual({ x: 0.6, y: 0.5 })
  })

  it('moves along the axis in proportion to the value', () => {
    const half = sliderHandlePoint(slider, 0.5, size)
    expect(half.x).toBeCloseTo(0.4)
    const grown = sliderHandlePoint(slider, 1.5, size)
    expect(grown.x).toBeCloseTo(0.8)
  })

  it('round-trips between value and position', () => {
    for (const value of [0.5, 0.8, 1, 1.25, 1.5]) {
      expect(sliderValueAt(slider, sliderHandlePoint(slider, value, size), size)).toBeCloseTo(value)
    }
  })

  it('clamps to the allowed range', () => {
    expect(sliderValueAt(slider, { x: 0.99, y: 0.5 }, size)).toBe(1.5)
    expect(sliderValueAt(slider, { x: 0, y: 0.5 }, size)).toBe(0.5)
  })

  it('projects off-axis drags onto the axis', () => {
    expect(sliderValueAt(slider, { x: 0.6, y: 0.9 }, size)).toBeCloseTo(1)
  })

  it('is stable for a zero-length axis', () => {
    const flat = { ...slider, axis: { start: slider.axis.start, end: slider.axis.start } }
    expect(sliderValueAt(flat, { x: 0.9, y: 0.9 }, size)).toBe(1)
  })

  it('measures distance in pixels, not normalised units', () => {
    const vertical = { ...slider, axis: { start: { x: 0.5, y: 0.2 }, end: { x: 0.5, y: 0.6 } } }
    expect(sliderValueAt(vertical, { x: 0.5, y: 0.4 }, size)).toBeCloseTo(0.5)
  })
})

describe('constrainPoint', () => {
  const origin = { x: 0.5, y: 0.5 }
  const free = { maxDistance: 1e9, snapRadius: 0, axisLock: false }

  it('keeps points inside the image', () => {
    expect(constrainPoint({ x: 1.4, y: -0.2 }, origin, size, free)).toEqual({ x: 1, y: 0 })
  })

  it('snaps back to the origin when released close to it', () => {
    const snapped = constrainPoint({ x: 0.503, y: 0.5 }, origin, size, { ...free, snapRadius: 8 })
    expect(snapped).toEqual(origin)
    const away = constrainPoint({ x: 0.52, y: 0.5 }, origin, size, { ...free, snapRadius: 8 })
    expect(away.x).toBeCloseTo(0.52)
  })

  it('limits how far a point can travel from its origin', () => {
    const limited = constrainPoint({ x: 1, y: 0.5 }, origin, size, { ...free, maxDistance: 100 })
    expect((limited.x - origin.x) * size.width).toBeCloseTo(100)
    expect(limited.y).toBeCloseTo(0.5)
  })

  it('locks movement to the dominant axis', () => {
    const lock = { ...free, axisLock: true }
    expect(constrainPoint({ x: 0.7, y: 0.52 }, origin, size, lock)).toEqual({ x: 0.7, y: 0.5 })
    expect(constrainPoint({ x: 0.51, y: 0.9 }, origin, size, lock)).toEqual({ x: 0.5, y: 0.9 })
  })
})

describe('change helpers', () => {
  const widget: Widget = {
    kind: 'multi_handle',
    handles: [
      { x: 0.1, y: 0.1 },
      { x: 0.9, y: 0.9 },
    ],
  }

  const moved = (shift: number): EditChange => ({
    kind: 'multi_handle',
    origin: initialPoints(widget as Exclude<Widget, { kind: 'slider' }>),
    target: [
      { x: 0.1 + shift, y: 0.1 },
      { x: 0.9, y: 0.9 },
    ],
  })

  it('knows when nothing changed', () => {
    expect(isUnchanged(moved(0), widget)).toBe(true)
    expect(isUnchanged(moved(0.05), widget)).toBe(false)
    expect(isUnchanged({ kind: 'slider', value: 1 }, slider)).toBe(true)
    expect(isUnchanged({ kind: 'slider', value: 0.8 }, slider)).toBe(false)
  })

  it('describes changes for people', () => {
    expect(describeChange({ kind: 'slider', value: 0.804 })).toBe('80%')
    expect(describeChange(moved(0.05))).toBe('1 point moved')
    expect(
      describeChange({
        kind: 'control_points',
        origin: [
          { x: 0, y: 0 },
          { x: 1, y: 1 },
        ],
        target: [
          { x: 0.2, y: 0 },
          { x: 1, y: 0.5 },
        ],
      }),
    ).toBe('2 points moved')
  })

  it('lists initial points for point-based widgets', () => {
    expect(initialPoints({ kind: 'control_points', points: [{ x: 0.3, y: 0.3 }] })).toEqual([
      { x: 0.3, y: 0.3 },
    ])
  })
})
