import { describe, expect, it } from 'vitest'
import { deformPositions, dragControlPoints, gridMesh, pinsAround } from './warp'

// Dragging must stay inside a 60 fps frame (16 ms). The limit here is deliberately loose so the
// test is not flaky on slow CI machines; the measured cost is far lower.
const FRAME_BUDGET_MS = 8

describe('warp performance', () => {
  it('deforms a full preview grid with many control points well inside a frame', () => {
    const rect = { x: 0, y: 0, width: 4000, height: 3000 }
    const mesh = gridMesh(rect, 28, 28)
    const origin = Array.from({ length: 16 }, (_, i) => ({
      x: 500 + (i % 4) * 900,
      y: 400 + Math.floor(i / 4) * 700,
    }))
    const target = origin.map((p, i) => (i === 5 ? { x: p.x + 300, y: p.y - 200 } : p))
    const deform = dragControlPoints(origin, target, pinsAround(rect))
    const out = new Float32Array(mesh.positions.length)

    deformPositions(mesh.positions, deform, out) // warm up the JIT
    const runs = 200
    const started = performance.now()
    for (let i = 0; i < runs; i++) deformPositions(mesh.positions, deform, out)
    const perFrame = (performance.now() - started) / runs

    console.info(
      `deformation of ${mesh.positions.length / 2} vertices: ${perFrame.toFixed(3)} ms per frame`,
    )
    expect(perFrame).toBeLessThan(FRAME_BUDGET_MS)
  })
})
