import { describe, expect, it } from 'vitest'
import { isSupportedImage } from './files'

describe('isSupportedImage', () => {
  it('accepts JPEG, PNG and WebP', () => {
    for (const type of ['image/jpeg', 'image/png', 'image/webp']) {
      expect(isSupportedImage({ type })).toBe(true)
    }
  })

  it('rejects everything else', () => {
    for (const type of ['image/gif', 'image/svg+xml', 'application/pdf', '']) {
      expect(isSupportedImage({ type })).toBe(false)
    }
  })
})
