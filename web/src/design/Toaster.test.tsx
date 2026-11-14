import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { Toaster } from './Toaster'
import { toast, useToastStore } from './toastStore'

afterEach(() => vi.useRealTimers())

describe('Toaster', () => {
  it('shows toasts in a polite live region', () => {
    render(<Toaster />)
    act(() => toast.info('Saved'))
    expect(screen.getByRole('status')).toHaveTextContent('Saved')
  })

  it('can be dismissed', async () => {
    render(<Toaster />)
    act(() => toast.error('Upload failed'))
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss' }))
    expect(screen.queryByText('Upload failed')).not.toBeInTheDocument()
  })

  it('dismisses itself after a few seconds', () => {
    vi.useFakeTimers()
    render(<Toaster />)
    act(() => toast.info('Heads up'))
    act(() => vi.advanceTimersByTime(6100))
    expect(screen.queryByText('Heads up')).not.toBeInTheDocument()
  })

  it('keeps only the latest few', () => {
    for (let i = 0; i < 6; i++) toast.info(`message ${i}`)
    expect(useToastStore.getState().toasts.map((t) => t.message)).toEqual([
      'message 3',
      'message 4',
      'message 5',
    ])
  })
})
