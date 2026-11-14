import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ActionBar } from './ActionBar'

describe('ActionBar', () => {
  it('stays hidden when nothing is pending', () => {
    const { container } = render(
      <ActionBar pendingCount={0} onApply={vi.fn()} onCancel={vi.fn()} />,
    )
    expect(container).toBeEmptyDOMElement()
  })

  it('summarises pending changes and reports Apply and Cancel', async () => {
    const onApply = vi.fn()
    const onCancel = vi.fn()
    const { rerender } = render(
      <ActionBar pendingCount={1} onApply={onApply} onCancel={onCancel} />,
    )
    expect(screen.getByText('1 change')).toBeInTheDocument()

    rerender(<ActionBar pendingCount={3} onApply={onApply} onCancel={onCancel} />)
    expect(screen.getByText('3 changes')).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Apply' }))
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onApply).toHaveBeenCalledTimes(1)
    expect(onCancel).toHaveBeenCalledTimes(1)
  })
})
