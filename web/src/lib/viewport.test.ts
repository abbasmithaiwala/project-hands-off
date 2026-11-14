import { describe, expect, it } from 'vitest'
import { fitViewport, imageToScreen, MAX_SCALE, panBy, screenToImage, zoomAt } from './viewport'

describe('fitViewport', () => {
  it('fits a wide image to the container width and centres it vertically', () => {
    const view = fitViewport({ width: 1000, height: 500 }, { width: 500, height: 500 })
    expect(view.scale).toBe(0.5)
    expect(view).toMatchObject({ x: 0, y: 125 })
  })

  it('respects padding', () => {
    const view = fitViewport({ width: 100, height: 100 }, { width: 300, height: 300 }, 50)
    expect(view.scale).toBe(2)
    expect(view).toMatchObject({ x: 50, y: 50 })
  })

  it('never produces a zero or negative scale for tiny containers', () => {
    const view = fitViewport({ width: 100, height: 100 }, { width: 10, height: 10 }, 50)
    expect(view.scale).toBeGreaterThan(0)
  })
})

describe('zoomAt', () => {
  it('keeps the anchored image point fixed on screen', () => {
    const start = { scale: 1, x: 20, y: 30 }
    const anchor = { x: 200, y: 150 }
    const before = screenToImage(start, anchor)
    const zoomed = zoomAt(start, 2, anchor, 1)
    const after = screenToImage(zoomed, anchor)
    expect(after.x).toBeCloseTo(before.x)
    expect(after.y).toBeCloseTo(before.y)
    expect(zoomed.scale).toBe(2)
  })

  it('clamps to the minimum and maximum scale', () => {
    const view = { scale: 1, x: 0, y: 0 }
    expect(zoomAt(view, 1000, { x: 0, y: 0 }, 1).scale).toBe(MAX_SCALE)
    expect(zoomAt(view, 0.0001, { x: 0, y: 0 }, 1).scale).toBe(0.25)
  })
})

describe('panBy and coordinate conversion', () => {
  it('translates without changing scale', () => {
    expect(panBy({ scale: 2, x: 10, y: 10 }, 5, -5)).toEqual({ scale: 2, x: 15, y: 5 })
  })

  it('round-trips between image and screen space', () => {
    const view = { scale: 1.5, x: 40, y: -20 }
    const point = { x: 123, y: 456 }
    const back = screenToImage(view, imageToScreen(view, point))
    expect(back.x).toBeCloseTo(point.x)
    expect(back.y).toBeCloseTo(point.y)
  })
})
