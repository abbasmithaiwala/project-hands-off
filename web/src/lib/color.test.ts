import { describe, expect, it } from 'vitest'
import { parseHexColor } from './color'

describe('parseHexColor', () => {
  it('parses six and three digit hex colours, with or without a hash', () => {
    expect(parseHexColor('#ff8a1f')).toBe(0xff8a1f)
    expect(parseHexColor(' ff8a1f ')).toBe(0xff8a1f)
    expect(parseHexColor('#fa0')).toBe(0xffaa00)
  })

  it('falls back for anything it cannot parse', () => {
    expect(parseHexColor('rgba(0,0,0,0.5)', 0x123456)).toBe(0x123456)
    expect(parseHexColor('', 0xabcdef)).toBe(0xabcdef)
    expect(parseHexColor('#12345', 0x1)).toBe(0x1)
  })
})
