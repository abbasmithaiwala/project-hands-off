import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { ImageRecord } from '@/api/client'
import { HistoryPanel } from './HistoryPanel'

const versions = [
  { id: 'a', created_at: '2026-01-01T00:00:00Z' },
  { id: 'b', created_at: '2026-01-01T01:00:00Z', edit: { summary: 'Shortened the couch' } },
] as unknown as ImageRecord[]

describe('HistoryPanel', () => {
  it('lists the newest version first and marks the current one', () => {
    render(<HistoryPanel versions={versions} cursor={1} onSelect={() => undefined} />)
    const items = screen.getAllByRole('button')
    expect(items[0]).toHaveTextContent('Shortened the couch')
    expect(items[0]).toHaveAttribute('aria-current', 'step')
    expect(items[1]).toHaveTextContent('Original')
    expect(items[1]).not.toHaveAttribute('aria-current')
  })

  it('reports the chosen version by its position in the project', async () => {
    const onSelect = vi.fn()
    render(<HistoryPanel versions={versions} cursor={1} onSelect={onSelect} />)
    await userEvent.click(screen.getByRole('button', { name: /Original/ }))
    expect(onSelect).toHaveBeenCalledWith(0)
  })
})
