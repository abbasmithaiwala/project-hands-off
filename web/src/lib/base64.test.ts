import { describe, expect, it } from 'vitest'
import { blobToBase64 } from './base64'

describe('blobToBase64', () => {
  it('encodes small blobs', async () => {
    expect(await blobToBase64(new Blob(['hello']))).toBe('aGVsbG8=')
  })

  it('encodes an empty blob as an empty string', async () => {
    expect(await blobToBase64(new Blob([]))).toBe('')
  })

  it('round-trips data larger than one chunk', async () => {
    const bytes = Uint8Array.from({ length: 100_000 }, (_, i) => i % 256)
    const encoded = await blobToBase64(new Blob([bytes]))
    const decoded = Uint8Array.from(atob(encoded), (c) => c.charCodeAt(0))
    expect(decoded).toEqual(bytes)
  })
})
