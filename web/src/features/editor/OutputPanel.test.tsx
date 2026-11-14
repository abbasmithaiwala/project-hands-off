import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { OutputPanel } from './OutputPanel'

describe('OutputPanel', () => {
  it('explains the empty state', () => {
    render(<OutputPanel lines={[]} onClose={vi.fn()} />)
    expect(screen.getByText(/Nothing yet/)).toBeInTheDocument()
  })

  it('lists log lines in order and can be closed', async () => {
    const onClose = vi.fn()
    render(<OutputPanel lines={['Loading model', 'Found 2 objects']} onClose={onClose} />)
    const items = screen.getAllByRole('listitem').map((li) => li.textContent)
    expect(items).toEqual(['Loading model', 'Found 2 objects'])
    await userEvent.click(screen.getByRole('button', { name: 'Close activity' }))
    expect(onClose).toHaveBeenCalled()
  })
})
