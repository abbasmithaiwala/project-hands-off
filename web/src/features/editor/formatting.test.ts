import { describe, expect, it } from 'vitest'
import type { EditResult } from '@/api/client'
import { describeRun, stageLabel } from './formatting'

const result = (overrides: Partial<EditResult> = {}): EditResult => ({
  summary: 's',
  changes: [],
  instruction: 'i',
  model: 'google/gemini-3-pro-image',
  cost_usd: 0.14043,
  generation_ms: 19215,
  ...overrides,
})

describe('stageLabel', () => {
  it('names each pipeline stage and falls back for unknown ones', () => {
    expect(stageLabel('generating')).toBe('Generating the image')
    expect(stageLabel('compositing')).toBe('Blending it into your photo')
    expect(stageLabel('something-new')).toBe('Working')
  })
})

describe('describeRun', () => {
  it('shows the short model name, seconds and dollars', () => {
    expect(describeRun(result())).toBe('gemini-3-pro-image · 19.2s · $0.14')
  })

  it('leaves out the cost when it is not reported', () => {
    expect(describeRun(result({ cost_usd: null }))).toBe('gemini-3-pro-image · 19.2s')
  })

  it('handles model ids without a vendor prefix', () => {
    expect(describeRun(result({ model: 'demo', cost_usd: null }))).toBe('demo · 19.2s')
  })
})
