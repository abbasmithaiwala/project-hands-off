import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApplyProgress } from './ApplyProgress'

afterEach(() => vi.useRealTimers())

describe('ApplyProgress', () => {
  it('names the current stage and offers Cancel', async () => {
    const onCancel = vi.fn()
    render(<ApplyProgress stage="generating" startedAt={Date.now()} onCancel={onCancel} />)
    expect(screen.getByRole('status', { name: 'Applying your edits' })).toHaveTextContent(
      'Generating the image…',
    )
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onCancel).toHaveBeenCalled()
  })

  it('counts the seconds that have passed', () => {
    vi.useFakeTimers()
    render(<ApplyProgress stage="generating" startedAt={Date.now()} onCancel={vi.fn()} />)
    expect(screen.getByText(/^0s/)).toBeInTheDocument()
    act(() => vi.advanceTimersByTime(12_100))
    expect(screen.getByText(/^12s/)).toBeInTheDocument()
  })
})
