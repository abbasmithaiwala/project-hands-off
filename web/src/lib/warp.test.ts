import { describe, expect, it } from 'vitest'
import {
  compose,
  deformPositions,
  dragControlPoints,
  gridMesh,
  identity,
  objectRegion,
  movingLeastSquares,
  pinsAround,
  stretchAlongAxis,
} from './warp'

const close = (a: { x: number; y: number }, b: { x: number; y: number }, digits = 6) => {
  expect(a.x).toBeCloseTo(b.x, digits)
  expect(a.y).toBeCloseTo(b.y, digits)
}

describe('stretchAlongAxis', () => {
  const start = { x: 100, y: 100 }
  const end = { x: 300, y: 100 }

  it('keeps the start fixed and scales distances along the axis', () => {
    const stretch = stretchAlongAxis(start, end, 0.5)
    close(stretch(start), start)
    close(stretch({ x: 300, y: 100 }), { x: 200, y: 100 })
  })

  it('leaves the perpendicular direction alone', () => {
    close(stretchAlongAxis(start, end, 2)({ x: 200, y: 150 }), { x: 300, y: 150 })
  })

  it('works for diagonal axes without skewing', () => {
    const stretch = stretchAlongAxis({ x: 0, y: 0 }, { x: 10, y: 10 }, 2)
    close(stretch({ x: 5, y: 5 }), { x: 10, y: 10 })
    close(stretch({ x: 5, y: -5 }), { x: 5, y: -5 })
  })

  it('is the identity for scale 1 or a zero-length axis', () => {
    expect(stretchAlongAxis(start, end, 1)).toBe(identity)
    expect(stretchAlongAxis(start, start, 3)).toBe(identity)
  })

  it('can be undone by the inverse scale', () => {
    const point = { x: 237, y: 181 }
    close(stretchAlongAxis(start, end, 1 / 1.7)(stretchAlongAxis(start, end, 1.7)(point)), point)
  })
})

describe('compose', () => {
  it('applies deformations in order', () => {
    const addOne = (p: { x: number; y: number }) => ({ x: p.x + 1, y: p.y })
    const double = (p: { x: number; y: number }) => ({ x: p.x * 2, y: p.y })
    expect(compose([addOne, double])({ x: 1, y: 0 })).toEqual({ x: 4, y: 0 })
    expect(compose([double, addOne])({ x: 1, y: 0 })).toEqual({ x: 3, y: 0 })
  })

  it('is the identity for an empty list', () => {
    expect(compose([])).toBe(identity)
  })
})

describe('movingLeastSquares', () => {
  const origin = [
    { x: 0, y: 0 },
    { x: 100, y: 0 },
    { x: 100, y: 100 },
    { x: 0, y: 100 },
  ]

  it('passes exactly through every control point', () => {
    const target = [
      { x: 10, y: 5 },
      { x: 120, y: -10 },
      { x: 90, y: 110 },
      { x: -5, y: 95 },
    ]
    const warp = movingLeastSquares(origin, target)
    origin.forEach((p, i) => close(warp(p), target[i] as { x: number; y: number }))
  })

  it('is the identity when control points do not move', () => {
    const warp = movingLeastSquares(origin, origin)
    for (const p of [
      { x: 30, y: 60 },
      { x: -20, y: 140 },
      { x: 55, y: 55 },
    ])
      close(warp(p), p, 4)
  })

  it('reproduces a pure translation everywhere', () => {
    const warp = movingLeastSquares(
      origin,
      origin.map((p) => ({ x: p.x + 20, y: p.y - 7 })),
    )
    close(warp({ x: 40, y: 70 }), { x: 60, y: 63 }, 4)
    close(warp({ x: 500, y: -300 }), { x: 520, y: -307 }, 4)
  })

  it('reproduces a global scale exactly (affine maps are in its span)', () => {
    const warp = movingLeastSquares(
      origin,
      origin.map((p) => ({ x: p.x * 2, y: p.y * 2 })),
    )
    close(warp({ x: 50, y: 25 }), { x: 100, y: 50 }, 4)
  })

  it('moves nearby points more than distant ones', () => {
    const warp = movingLeastSquares(
      [
        { x: 0, y: 0 },
        { x: 1000, y: 0 },
      ],
      [
        { x: 50, y: 0 },
        { x: 1000, y: 0 },
      ],
    )
    const near = warp({ x: 20, y: 0 }).x - 20
    const far = warp({ x: 800, y: 0 }).x - 800
    expect(near).toBeGreaterThan(far)
  })

  it('handles degenerate (collinear or single) control sets without NaN', () => {
    const single = movingLeastSquares([{ x: 10, y: 10 }], [{ x: 20, y: 30 }])
    close(single({ x: 50, y: 50 }), { x: 60, y: 70 })
    const line = movingLeastSquares(
      [
        { x: 0, y: 0 },
        { x: 10, y: 0 },
        { x: 20, y: 0 },
      ],
      [
        { x: 0, y: 5 },
        { x: 10, y: 5 },
        { x: 20, y: 5 },
      ],
    )
    const moved = line({ x: 7, y: 3 })
    expect(Number.isFinite(moved.x) && Number.isFinite(moved.y)).toBe(true)
  })

  it('is the identity with no control points', () => {
    expect(movingLeastSquares([], [])).toBe(identity)
  })
})

describe('dragControlPoints', () => {
  const rect = { x: 0, y: 0, width: 200, height: 200 }

  it('is the identity when nothing moved', () => {
    const points = [{ x: 100, y: 100 }]
    expect(dragControlPoints(points, points, pinsAround(rect))).toBe(identity)
  })

  it('moves the dragged point exactly and keeps the pins fixed', () => {
    const warp = dragControlPoints([{ x: 100, y: 100 }], [{ x: 130, y: 90 }], pinsAround(rect))
    close(warp({ x: 100, y: 100 }), { x: 130, y: 90 })
    for (const pin of pinsAround(rect)) close(warp(pin), pin)
  })

  it('falls off smoothly towards the pins', () => {
    const warp = dragControlPoints([{ x: 100, y: 100 }], [{ x: 160, y: 100 }], pinsAround(rect))
    const centre = warp({ x: 100, y: 100 }).x - 100
    const nearEdge = warp({ x: 190, y: 100 }).x - 190
    expect(centre).toBeCloseTo(60)
    expect(Math.abs(nearEdge)).toBeLessThan(centre / 2)
  })
})

describe('pinsAround', () => {
  it('returns the corners and edge midpoints', () => {
    const pins = pinsAround({ x: 10, y: 20, width: 100, height: 40 })
    expect(pins).toHaveLength(8)
    expect(pins).toContainEqual({ x: 60, y: 20 })
    expect(pins).toContainEqual({ x: 110, y: 60 })
  })
})

describe('gridMesh and deformPositions', () => {
  const rect = { x: 10, y: 20, width: 100, height: 50 }

  it('covers the rectangle with the right vertex and triangle counts', () => {
    const mesh = gridMesh(rect, 4, 2)
    expect(mesh.positions).toHaveLength((4 + 1) * (2 + 1) * 2)
    expect(mesh.indices).toHaveLength(4 * 2 * 6)
    expect([...mesh.positions.slice(0, 2)]).toEqual([10, 20])
    expect([...mesh.positions.slice(-2)]).toEqual([110, 70])
  })

  it('maps texture coordinates across the full 0..1 range', () => {
    const mesh = gridMesh(rect, 3, 3)
    expect([...mesh.uvs.slice(0, 2)]).toEqual([0, 0])
    expect([...mesh.uvs.slice(-2)]).toEqual([1, 1])
  })

  it('only references vertices that exist', () => {
    const mesh = gridMesh(rect, 5, 4)
    const vertexCount = mesh.positions.length / 2
    expect(Math.max(...mesh.indices)).toBeLessThan(vertexCount)
  })

  it('never needs fewer than one cell', () => {
    expect(gridMesh(rect, 0, 0).indices).toHaveLength(6)
  })

  it('deforms every vertex and can reuse an output buffer', () => {
    const mesh = gridMesh(rect, 2, 2)
    const out = new Float32Array(mesh.positions.length)
    const result = deformPositions(mesh.positions, (p) => ({ x: p.x + 1, y: p.y * 2 }), out)
    expect(result).toBe(out)
    expect([...out.slice(0, 2)]).toEqual([11, 40])
  })
})

describe('objectRegion', () => {
  const size = { width: 1000, height: 500 }

  it('grows the box by the margin and returns whole pixels', () => {
    const region = objectRegion({ x: 0.2, y: 0.2, width: 0.5, height: 0.5 }, size, 0.1)
    expect(region).toEqual({ x: 150, y: 75, width: 600, height: 300 })
  })

  it('stays inside the image', () => {
    const region = objectRegion({ x: 0, y: 0, width: 1, height: 1 }, size, 0.2)
    expect(region).toEqual({ x: 0, y: 0, width: 1000, height: 500 })
  })

  it('never collapses to nothing', () => {
    const region = objectRegion({ x: 1, y: 1, width: 0, height: 0 }, size, 0)
    expect(region.width).toBeGreaterThanOrEqual(1)
    expect(region.height).toBeGreaterThanOrEqual(1)
  })
})
