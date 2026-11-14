import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { EditResult } from '@/api/client'
import { ReviewBar } from './ReviewBar'

const result: EditResult = {
  summary: 'Shortened the couch',
  changes: [
    { edit_index: 0, text: 'Shortened the couch to about 60% of its length' },
    { edit_index: 1, text: 'Moved the left cushion inwards' },
  ],
  instruction: 'Make the couch shorter.',
  model: 'google/gemini-3-pro-image',
  cost_usd: 0.14,
  generation_ms: 19200,
}

function setup(withDraft = true) {
  const handlers = { onAccept: vi.fn(), onRetry: vi.fn(), onDraft: vi.fn(), onDiscard: vi.fn() }
  render(
    <ReviewBar
      result={result}
      draftModel={withDraft ? 'google/gemini-3.1-flash-image' : undefined}
      {...handlers}
    />,
  )
  return handlers
}

describe('ReviewBar', () => {
  it('summarises what changed and how it was made', () => {
    setup()
    const region = screen.getByRole('region', { name: 'Review your edit' })
    expect(region).toHaveTextContent('Shortened the couch')
    expect(screen.getAllByRole('listitem').map((li) => li.textContent)).toEqual([
      'Shortened the couch to about 60% of its length',
      'Moved the left cushion inwards',
    ])
    expect(region).toHaveTextContent('gemini-3-pro-image · 19.2s · $0.14')
  })

  it('reports each choice', async () => {
    const handlers = setup()
    await userEvent.click(screen.getByRole('button', { name: 'Accept' }))
    await userEvent.click(screen.getByRole('button', { name: 'Retry' }))
    await userEvent.click(screen.getByRole('button', { name: 'Faster draft' }))
    await userEvent.click(screen.getByRole('button', { name: 'Discard' }))
    expect(handlers.onAccept).toHaveBeenCalledTimes(1)
    expect(handlers.onRetry).toHaveBeenCalledTimes(1)
    expect(handlers.onDraft).toHaveBeenCalledTimes(1)
    expect(handlers.onDiscard).toHaveBeenCalledTimes(1)
  })

  it('hides the faster draft option when no draft model is configured', () => {
    setup(false)
    expect(screen.queryByRole('button', { name: 'Faster draft' })).not.toBeInTheDocument()
  })
})
