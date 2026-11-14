import { afterEach, describe, expect, it, vi } from 'vitest'
import { downloadText, downloadUrl } from './download'

afterEach(() => vi.restoreAllMocks())

describe('downloads', () => {
  it('clicks a link carrying the file name', () => {
    const click = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (
      this: HTMLAnchorElement,
    ) {
      expect(this.download).toBe('photo.png')
      expect(this.getAttribute('href')).toBe('/x')
    })
    downloadUrl('/x', 'photo.png')
    expect(click).toHaveBeenCalledOnce()
  })

  it('saves text through an object URL that is released afterwards', () => {
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => undefined)
    const create = vi.fn(() => 'blob:fake')
    const revoke = vi.fn()
    vi.stubGlobal('URL', { createObjectURL: create, revokeObjectURL: revoke })
    downloadText('{}', 'log.json', 'application/json')
    expect(create).toHaveBeenCalledOnce()
    expect(revoke).toHaveBeenCalledWith('blob:fake')
    vi.unstubAllGlobals()
  })
})
