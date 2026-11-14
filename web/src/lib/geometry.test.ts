import { describe, expect, it } from 'vitest'
import { cycle, labelAnchor, pickObject, pointInPolygon, type Pickable } from './geometry'

const square = [
  { x: 0, y: 0 },
  { x: 1, y: 0 },
  { x: 1, y: 1 },
  { x: 0, y: 1 },
]

function object(id: number, scale: number, offset = 0): Pickable {
  return {
    id,
    bbox: { width: scale, height: scale },
    outline: square.map((p) => ({ x: p.x * scale + offset, y: p.y * scale + offset })),
  }
}

describe('pointInPolygon', () => {
  it('detects inside and outside points', () => {
    expect(pointInPolygon({ x: 0.5, y: 0.5 }, square)).toBe(true)
    expect(pointInPolygon({ x: 1.5, y: 0.5 }, square)).toBe(false)
    expect(pointInPolygon({ x: -0.1, y: -0.1 }, square)).toBe(false)
  })

  it('handles concave shapes', () => {
    const lShape = [
      { x: 0, y: 0 },
      { x: 2, y: 0 },
      { x: 2, y: 1 },
      { x: 1, y: 1 },
      { x: 1, y: 2 },
      { x: 0, y: 2 },
    ]
    expect(pointInPolygon({ x: 0.5, y: 1.5 }, lShape)).toBe(true)
    expect(pointInPolygon({ x: 1.5, y: 1.5 }, lShape)).toBe(false)
  })

  it('is false for degenerate polygons', () => {
    expect(pointInPolygon({ x: 0, y: 0 }, [])).toBe(false)
    expect(pointInPolygon({ x: 0, y: 0 }, [{ x: 0, y: 0 }])).toBe(false)
  })
})

describe('pickObject', () => {
  it('prefers the smallest containing object so nested objects can be chosen', () => {
    const person = object(0, 10)
    const cup = object(1, 2, 3)
    expect(pickObject([person, cup], { x: 4, y: 4 })).toBe(1)
    expect(pickObject([person, cup], { x: 8, y: 8 })).toBe(0)
  })

  it('returns null when nothing contains the point', () => {
    expect(pickObject([object(0, 1)], { x: 5, y: 5 })).toBeNull()
    expect(pickObject([], { x: 0, y: 0 })).toBeNull()
  })
})

describe('cycle', () => {
  it('moves forward and backward with wrap-around', () => {
    expect(cycle([4, 5, 6], 4, 1)).toBe(5)
    expect(cycle([4, 5, 6], 6, 1)).toBe(4)
    expect(cycle([4, 5, 6], 4, -1)).toBe(6)
  })

  it('starts at an end when nothing is current or the current item is unknown', () => {
    expect(cycle([4, 5, 6], null, 1)).toBe(4)
    expect(cycle([4, 5, 6], null, -1)).toBe(6)
    expect(cycle([4, 5, 6], 99, 1)).toBe(4)
  })

  it('returns null for an empty list', () => {
    expect(cycle([], null, 1)).toBeNull()
  })
})

describe('labelAnchor', () => {
  it('picks the topmost point, leftmost on ties', () => {
    expect(
      labelAnchor([
        { x: 0.5, y: 0.4 },
        { x: 0.2, y: 0.3 },
        { x: 0.6, y: 0.3 },
        { x: 0.1, y: 0.9 },
      ]),
    ).toEqual({ x: 0.2, y: 0.3 })
  })

  it('has no anchor for an empty outline', () => {
    expect(labelAnchor([])).toBeNull()
  })
})
