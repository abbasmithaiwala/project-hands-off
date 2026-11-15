import { describe, expect, it } from 'vitest'
import type { EditChange, Widget } from '@/api/client'
import {
  constrainPoint,
  describeChange,
  describeMove,
  frameCorners,
  initialPoints,
  isUnchanged,
  rotateDegreesAt,
  rotateHandlePoint,
  sliderHandlePoint,
  sliderValueAt,
  wrapDegrees,
} from './widgetMath'
import { scaleAbout } from './warp'

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

const tilt: Extract<Widget, { kind: 'rotate' }> = {
  kind: 'rotate',
  pivot: { x: 0.5, y: 0.8 },
  handle: { x: 0.5, y: 0.2 },
  min: -45,
  max: 45,
}

describe('rotate handle', () => {
  it('rests straight above the pivot at zero turn', () => {
    const rest = rotateHandlePoint(tilt, 0, size)
    expect(rest.x).toBeCloseTo(0.5)
    expect(rest.y).toBeCloseTo(0.2)
  })

  it('swings clockwise on screen for a positive turn and keeps its radius', () => {
    const turned = rotateHandlePoint(tilt, 90, size)
    // 0.3 of the 500px height is 300px; a quarter turn puts the handle 300px to the right.
    expect(turned.x * size.width).toBeCloseTo(500 + 300)
    expect(turned.y * size.height).toBeCloseTo(0.8 * size.height)
    const back = rotateHandlePoint(tilt, -90, size)
    expect(back.x * size.width).toBeCloseTo(500 - 300)
  })

  it('round-trips between angle and position', () => {
    for (const degrees of [-45, -20, 0, 12.5, 45]) {
      expect(rotateDegreesAt(tilt, rotateHandlePoint(tilt, degrees, size), size)).toBeCloseTo(
        degrees,
      )
    }
  })

  it('reads the angle from the pointer direction, not its distance', () => {
    const far = { x: 0.5 + 0.2, y: 0.8 - 0.45 }
    const near = { x: 0.5 + 0.04, y: 0.8 - 0.09 }
    expect(rotateDegreesAt(tilt, far, size)).toBeCloseTo(rotateDegreesAt(tilt, near, size))
  })

  it('clamps to the allowed turn and is stable on the pivot', () => {
    expect(rotateDegreesAt(tilt, { x: 0.95, y: 0.8 }, size)).toBe(45)
    expect(rotateDegreesAt(tilt, { x: 0.05, y: 0.8 }, size)).toBe(-45)
    expect(rotateDegreesAt(tilt, tilt.pivot, size)).toBe(0)
  })

  it('wraps angles into a half-open half turn either side', () => {
    expect(wrapDegrees(190)).toBeCloseTo(-170)
    expect(wrapDegrees(-190)).toBeCloseTo(170)
    expect(wrapDegrees(180)).toBe(180)
    expect(wrapDegrees(-180)).toBe(180)
    expect(wrapDegrees(720 + 30)).toBeCloseTo(30)
  })
})

describe('frameCorners', () => {
  it('maps the four corners of a box through a deformation, in pixels', () => {
    const corners = frameCorners(
      { x: 0.2, y: 0.2, width: 0.4, height: 0.2 },
      size,
      scaleAbout({ x: 200, y: 100 }, 2),
    )
    const expected = [
      { x: 200, y: 100 },
      { x: 1000, y: 100 },
      { x: 1000, y: 300 },
      { x: 200, y: 300 },
    ]
    expect(corners).toHaveLength(4)
    corners.forEach((corner, i) => {
      expect(corner.x).toBeCloseTo(expected[i]?.x ?? NaN)
      expect(corner.y).toBeCloseTo(expected[i]?.y ?? NaN)
    })
  })
})

describe('describing transforms', () => {
  it('shows turns with a sign, sizes as percentages and moves in words', () => {
    const pivot = { x: 0.5, y: 0.5 }
    expect(describeChange({ kind: 'rotate', degrees: 14.6, pivot })).toBe('+15°')
    expect(describeChange({ kind: 'rotate', degrees: -8, pivot })).toBe('-8°')
    expect(describeChange({ kind: 'scale', value: 1.254, pivot })).toBe('125%')
    expect(describeChange({ kind: 'move', dx: -0.12, dy: 0.05 })).toBe('12% left, 5% down')
    expect(describeMove(0, -0.3)).toBe('30% up')
    expect(describeMove(0.001, 0)).toBe('0%')
  })

  it('knows when a transform changed nothing', () => {
    const pivot = { x: 0.5, y: 0.5 }
    expect(isUnchanged({ kind: 'rotate', degrees: 0.01, pivot }, tilt)).toBe(true)
    expect(isUnchanged({ kind: 'rotate', degrees: 5, pivot }, tilt)).toBe(false)
    expect(isUnchanged({ kind: 'move', dx: 0, dy: 0 }, { kind: 'move', handle: pivot })).toBe(true)
    expect(isUnchanged({ kind: 'move', dx: 0.02, dy: 0 }, { kind: 'move', handle: pivot })).toBe(
      false,
    )
    const grow: Widget = { kind: 'scale', axis: slider.axis, min: 0.5, max: 2, initial: 1 }
    expect(isUnchanged({ kind: 'scale', value: 1, pivot }, grow)).toBe(true)
    expect(isUnchanged({ kind: 'scale', value: 1.4, pivot }, grow)).toBe(false)
    expect(isUnchanged({ kind: 'scale', value: 1, pivot }, slider)).toBe(false)
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
    origin: initialPoints(widget as Extract<Widget, { kind: 'multi_handle' }>),
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
