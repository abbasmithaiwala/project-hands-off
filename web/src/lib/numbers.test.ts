import { describe, expect, it } from 'vitest'
import { clamp } from './numbers'

describe('clamp', () => {
  it('limits a value to a range', () => {
    expect(clamp(5, 0, 3)).toBe(3)
    expect(clamp(-5, 0, 3)).toBe(0)
    expect(clamp(2, 0, 3)).toBe(2)
  })
})
