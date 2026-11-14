import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ActionBar, type PendingItem } from './ActionBar'

const items: PendingItem[] = [
  { key: '0:length', label: 'Couch length · couch', summary: '80%' },
  { key: '0:shape', label: 'Outline shape · couch', summary: '2 points moved' },
]

function setup(overrides: Partial<Parameters<typeof ActionBar>[0]> = {}) {
  const props = { items, onReset: vi.fn(), onApply: vi.fn(), onCancel: vi.fn(), ...overrides }
  render(<ActionBar {...props} />)
  return props
}

describe('ActionBar', () => {
  it('stays hidden when nothing is pending', () => {
    const { container } = render(
      <ActionBar items={[]} onReset={vi.fn()} onApply={vi.fn()} onCancel={vi.fn()} />,
    )
    expect(container).toBeEmptyDOMElement()
  })

  it('summarises the number of pending changes', () => {
    setup()
    expect(screen.getByRole('button', { name: '2 changes' })).toBeInTheDocument()
  })

  it('uses the singular for one change', () => {
    setup({ items: items.slice(0, 1) })
    expect(screen.getByRole('button', { name: '1 change' })).toBeInTheDocument()
  })

  it('lists each change with what was done when expanded', async () => {
    setup()
    const toggle = screen.getByRole('button', { name: '2 changes' })
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    expect(screen.queryByRole('list')).not.toBeInTheDocument()

    await userEvent.click(toggle)
    expect(toggle).toHaveAttribute('aria-expanded', 'true')
    const rows = screen.getAllByRole('listitem').map((row) => row.textContent)
    expect(rows).toEqual(['Couch length · couch80%', 'Outline shape · couch2 points moved'])
  })

  it('resets a single change', async () => {
    const { onReset } = setup()
    await userEvent.click(screen.getByRole('button', { name: '2 changes' }))
    await userEvent.click(screen.getByRole('button', { name: 'Reset Outline shape · couch' }))
    expect(onReset).toHaveBeenCalledWith('0:shape')
  })

  it('reports Apply and Cancel', async () => {
    const { onApply, onCancel } = setup()
    await userEvent.click(screen.getByRole('button', { name: 'Apply' }))
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onApply).toHaveBeenCalledTimes(1)
    expect(onCancel).toHaveBeenCalledTimes(1)
  })

  it('disables Apply and explains why when it is unavailable', async () => {
    const { onApply } = setup({ applyUnavailable: 'Not connected yet.' })
    const apply = screen.getByRole('button', { name: 'Apply' })
    expect(apply).toBeDisabled()
    expect(apply).toHaveAccessibleDescription('Not connected yet.')
    await userEvent.click(apply)
    expect(onApply).not.toHaveBeenCalled()
  })
})
