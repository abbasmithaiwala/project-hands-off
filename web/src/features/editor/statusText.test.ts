import { describe, expect, it } from 'vitest'
import { statusText } from './statusText'

describe('statusText', () => {
  it('describes each analysis state', () => {
    expect(statusText('idle', 0)).toBe('Ready')
    expect(statusText('running', 0)).toBe('Finding objects…')
    expect(statusText('running', 2)).toBe('Finding objects · 2 so far')
    expect(statusText('failed', 0)).toBe('Analysis failed')
  })

  it('pluralises the result', () => {
    expect(statusText('done', 1)).toBe('1 object found')
    expect(statusText('done', 0)).toBe('0 objects found')
    expect(statusText('done', 3)).toBe('3 objects found')
  })
})
