import { describe, expect, it } from 'vitest'
import { cn } from './cn'

describe('cn', () => {
  it('joins truthy parts and skips the rest', () => {
    expect(cn('a', false, 'b', null, undefined, '', 'c')).toBe('a b c')
  })

  it('returns an empty string when nothing applies', () => {
    expect(cn(false, null)).toBe('')
  })
})
