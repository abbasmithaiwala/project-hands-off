import { describe, expect, it } from 'vitest'
import type { DetectedObject } from '@/api/client'
import { affectedRegion, boundingBox, unionBox } from './regions'

const size = { width: 1000, height: 500 }

const object: DetectedObject = {
  id: 0,
  label: 'couch',
  score: 0.9,
  bbox: { x: 0.2, y: 0.4, width: 0.4, height: 0.4 },
  outline: [
    { x: 0.2, y: 0.4 },
    { x: 0.6, y: 0.4 },
    { x: 0.6, y: 0.8 },
    { x: 0.2, y: 0.8 },
  ],
}

describe('boundingBox and unionBox', () => {
  it('finds the box around points', () => {
    expect(
      boundingBox([
        { x: 0.2, y: 0.5 },
        { x: 0.6, y: 0.1 },
      ]),
    ).toEqual({
      x: 0.2,
      y: 0.1,
      width: expect.closeTo(0.4, 6),
      height: expect.closeTo(0.4, 6),
    })
    expect(boundingBox([])).toBeNull()
  })

  it('unions two boxes', () => {
    const union = unionBox(
      { x: 0, y: 0, width: 0.2, height: 0.2 },
      { x: 0.5, y: 0.5, width: 0.1, height: 0.1 },
    )
    expect(union).toEqual({
      x: 0,
      y: 0,
      width: expect.closeTo(0.6, 6),
      height: expect.closeTo(0.6, 6),
    })
  })
})

describe('affectedRegion', () => {
  it('covers the object even when nothing moves, with a small margin', () => {
    const region = affectedRegion(object, (p) => p, size)
    expect(region.x).toBeLessThan(object.bbox.x)
    expect(region.y).toBeLessThan(object.bbox.y)
    expect(region.x + region.width).toBeGreaterThan(0.6)
  })

  it('covers both where the object was and where it moved to', () => {
    const shiftRight = (p: { x: number; y: number }) => ({ x: p.x + 300, y: p.y })
    const region = affectedRegion(object, shiftRight, size)
    expect(region.x).toBeLessThanOrEqual(0.2)
    expect(region.x + region.width).toBeGreaterThanOrEqual(0.9)
  })

  it('stays inside the image even if the object is pushed out of it', () => {
    const shoveOut = (p: { x: number; y: number }) => ({ x: p.x + 5000, y: p.y - 5000 })
    const region = affectedRegion(object, shoveOut, size)
    for (const value of [region.x, region.y, region.x + region.width, region.y + region.height]) {
      expect(value).toBeGreaterThanOrEqual(0)
      expect(value).toBeLessThanOrEqual(1)
    }
    expect(region.width).toBeGreaterThan(0)
    expect(region.height).toBeGreaterThan(0)
  })
})
