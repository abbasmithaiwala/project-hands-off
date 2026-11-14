import { describe, expect, it } from 'vitest'
import { dilate, fillMasked } from './fill'

function image(
  width: number,
  height: number,
  colour: (x: number, y: number) => [number, number, number],
) {
  const rgb = new Uint8ClampedArray(width * height * 3)
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) rgb.set(colour(x, y), (y * width + x) * 3)
  }
  return rgb
}

const pixel = (rgb: Uint8ClampedArray, width: number, x: number, y: number) => [
  ...rgb.slice((y * width + x) * 3, (y * width + x) * 3 + 3),
]

describe('fillMasked', () => {
  it('fills a hole in a flat background with that colour', () => {
    const rgb = image(10, 10, () => [200, 100, 50])
    const masked = new Uint8Array(100)
    for (let y = 3; y < 7; y++) for (let x = 3; x < 7; x++) masked[y * 10 + x] = 1
    for (let i = 0; i < 100; i++) if (masked[i]) rgb.set([0, 0, 255], i * 3)

    const filled = fillMasked(rgb, masked, 10, 10)
    for (const [x, y] of [
      [3, 3],
      [5, 5],
      [6, 6],
    ] as const) {
      const [r, g, b] = pixel(filled, 10, x, y)
      expect(Math.abs((r ?? 0) - 200)).toBeLessThanOrEqual(2)
      expect(Math.abs((g ?? 0) - 100)).toBeLessThanOrEqual(2)
      expect(Math.abs((b ?? 0) - 50)).toBeLessThanOrEqual(2)
    }
  })

  it('blends a gradient smoothly across the hole', () => {
    const width = 21
    const rgb = image(width, 5, (x) => [x * 10, 0, 0])
    const masked = new Uint8Array(width * 5)
    for (let y = 0; y < 5; y++) for (let x = 8; x < 13; x++) masked[y * width + x] = 1

    const filled = fillMasked(rgb, masked, width, 5, 400)
    const reds = [7, 8, 10, 12, 13].map((x) => pixel(filled, width, x, 2)[0] ?? 0)
    expect(reds).toEqual([...reds].sort((a, b) => a - b))
    expect(reds[2]).toBeGreaterThan(80)
    expect(reds[2]).toBeLessThan(130)
  })

  it('never changes pixels outside the mask', () => {
    const rgb = image(6, 6, (x, y) => [x * 30, y * 30, 7])
    const masked = new Uint8Array(36)
    masked[14] = 1
    const filled = fillMasked(rgb, masked, 6, 6)
    for (let i = 0; i < 36; i++) {
      if (!masked[i])
        expect(pixel(filled, 6, i % 6, Math.floor(i / 6))).toEqual(
          pixel(rgb, 6, i % 6, Math.floor(i / 6)),
        )
    }
  })

  it('returns the image unchanged when nothing or everything is masked', () => {
    const rgb = image(4, 4, () => [10, 20, 30])
    expect(fillMasked(rgb, new Uint8Array(16), 4, 4)).toEqual(rgb)
    expect(fillMasked(rgb, new Uint8Array(16).fill(1), 4, 4)).toEqual(rgb)
  })
})

describe('dilate', () => {
  it('grows a single pixel into a square', () => {
    const mask = new Uint8Array(25)
    mask[12] = 1
    const grown = dilate(mask, 5, 5, 1)
    expect(grown.reduce((sum, v) => sum + v, 0)).toBe(9)
    expect(grown[0]).toBe(0)
  })

  it('leaves the mask alone at radius zero and respects the borders', () => {
    const mask = new Uint8Array(9)
    mask[0] = 1
    expect([...dilate(mask, 3, 3, 0)]).toEqual([...mask])
    expect(dilate(mask, 3, 3, 1).reduce((sum, v) => sum + v, 0)).toBe(4)
  })
})
