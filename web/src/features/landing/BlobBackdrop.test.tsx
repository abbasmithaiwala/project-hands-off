import { render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { BlobBackdrop } from './BlobBackdrop'

/** A WebGL stand-in where every call succeeds, so the shader plumbing can be exercised. */
function fakeGl() {
  const calls: string[] = []
  const gl = new Proxy(
    {},
    {
      get: (_target, name: string) =>
        /^[A-Z_0-9]+$/.test(name)
          ? 1
          : (...args: unknown[]) => {
              calls.push(name)
              return name.startsWith('get') || name.startsWith('create') ? { args } : undefined
            },
    },
  )
  return { gl, calls }
}

let frames: FrameRequestCallback[] = []

beforeEach(() => {
  frames = []
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => frames.push(callback))
  vi.stubGlobal('cancelAnimationFrame', vi.fn())
})

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

function stubMotion(reduced: boolean) {
  vi.spyOn(window, 'matchMedia').mockImplementation(
    (query) => ({ matches: reduced && query.includes('reduce'), media: query }) as MediaQueryList,
  )
}

describe('BlobBackdrop', () => {
  it('keeps the CSS fallback when WebGL is unavailable', () => {
    stubMotion(false)
    render(<BlobBackdrop />)
    expect(screen.getByTestId('blob-canvas')).not.toHaveAttribute('data-ready')
    expect(frames).toHaveLength(0)
  })

  it('animates when WebGL works and stops when it unmounts', () => {
    stubMotion(false)
    const { gl, calls } = fakeGl()
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(gl as never)
    const { unmount } = render(<BlobBackdrop />)
    expect(screen.getByTestId('blob-canvas')).toHaveAttribute('data-ready', 'true')
    expect(frames).toHaveLength(1)
    frames[0]?.(0)
    expect(calls).toContain('drawArrays')
    unmount()
    expect(calls).toContain('deleteProgram')
  })

  it('paints a single still frame when motion is reduced', () => {
    stubMotion(true)
    const { gl, calls } = fakeGl()
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(gl as never)
    render(<BlobBackdrop />)
    expect(calls.filter((name) => name === 'drawArrays')).toHaveLength(1)
    expect(frames).toHaveLength(0)
  })
})
