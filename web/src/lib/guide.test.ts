import { describe, expect, it, vi } from 'vitest'
import { drawGuide, GUIDE_COLOUR, type GuideContext } from './guide'

function fakeContext() {
  const calls: string[] = []
  const record = (name: string) =>
    vi.fn(
      (...args: unknown[]) =>
        void calls.push(
          `${name}(${args.map((a) => (typeof a === 'number' ? Math.round(a) : a)).join(',')})`,
        ),
    )
  const context = {
    beginPath: record('begin'),
    moveTo: record('move'),
    lineTo: record('line'),
    closePath: record('close'),
    stroke: record('stroke'),
    arc: record('arc'),
    fill: record('fill'),
    strokeStyle: '',
    fillStyle: '',
    lineWidth: 0,
    lineJoin: 'miter',
  } as unknown as GuideContext
  return { context, calls }
}

const square = [
  { x: 0.25, y: 0.25 },
  { x: 0.75, y: 0.25 },
  { x: 0.75, y: 0.75 },
  { x: 0.25, y: 0.75 },
]

describe('drawGuide', () => {
  it('traces the closed shape in pixels and marks every corner', () => {
    const { context, calls } = fakeContext()
    drawGuide(context, [square], 1000, 500)
    expect(calls.slice(0, 6)).toEqual([
      'begin()',
      'move(250,125)',
      'line(750,125)',
      'line(750,375)',
      'line(250,375)',
      'close()',
    ])
    expect(calls.filter((c) => c.startsWith('arc'))).toHaveLength(4)
    expect(context.strokeStyle).toBe(GUIDE_COLOUR)
  })

  it('keeps lines visible on small images and scales them up on large ones', () => {
    const small = fakeContext()
    drawGuide(small.context, [square], 400, 300)
    expect(small.context.lineWidth).toBe(3)
    const large = fakeContext()
    drawGuide(large.context, [square], 3200, 2400)
    expect(large.context.lineWidth).toBe(10)
  })

  it('ignores empty shapes', () => {
    const { context, calls } = fakeContext()
    drawGuide(context, [[]], 100, 100)
    expect(calls).toEqual([])
  })
})
