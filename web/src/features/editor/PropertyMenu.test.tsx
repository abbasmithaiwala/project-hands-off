import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Property } from '@/api/client'
import { ApiError } from '@/api/errors'
import { PropertyMenu } from './PropertyMenu'

const properties: Property[] = [
  { key: 'length', label: 'Couch length', widget: 'slider' },
  { key: 'shape', label: 'Outline shape', widget: 'control_points' },
  { key: 'arms', label: 'Arm position', widget: 'multi_handle' },
]

function setup(overrides: Partial<Parameters<typeof PropertyMenu>[0]> = {}) {
  const props = {
    objectLabel: 'couch',
    properties,
    loading: false,
    error: null,
    activeKey: null,
    onPick: vi.fn(),
    onRetry: vi.fn(),
    ...overrides,
  }
  render(<PropertyMenu {...props} />)
  return props
}

describe('PropertyMenu', () => {
  it('lists each property as a button inside a labelled region', () => {
    setup()
    expect(screen.getByRole('region', { name: 'Edit couch' })).toBeInTheDocument()
    expect(screen.getAllByRole('button')).toHaveLength(3)
    expect(screen.getByRole('button', { name: 'Couch length' })).toHaveAttribute(
      'aria-pressed',
      'false',
    )
  })

  it('marks the active property and reports picks', async () => {
    const { onPick } = setup({ activeKey: 'shape' })
    expect(screen.getByRole('button', { name: 'Outline shape' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
    await userEvent.click(screen.getByRole('button', { name: 'Arm position' }))
    expect(onPick).toHaveBeenCalledWith('arms')
  })

  it('shows placeholders while suggestions load', () => {
    setup({ properties: undefined, loading: true })
    expect(screen.getByLabelText('Loading suggestions')).toBeInTheDocument()
    expect(screen.queryAllByRole('button')).toHaveLength(0)
  })

  it('explains failures and offers a retry', async () => {
    const { onRetry } = setup({
      properties: undefined,
      error: new ApiError('unavailable', 'Models are down.', 503),
    })
    expect(screen.getByRole('alert')).toHaveTextContent('Models are down.')
    await userEvent.click(screen.getByRole('button', { name: 'Try again' }))
    expect(onRetry).toHaveBeenCalled()
  })

  it('marks properties that already have an edit', () => {
    setup({ editedKeys: new Set(['shape']) })
    const edited = screen.getAllByRole('img', { name: 'edited' })
    expect(edited).toHaveLength(1)
    expect(screen.getByRole('button', { name: /Outline shape/ })).toContainElement(
      edited[0] as HTMLElement,
    )
  })

  it('shows a status message politely', () => {
    setup({ status: 'Placing the control…' })
    expect(screen.getByRole('status')).toHaveTextContent('Placing the control…')
  })

  it('never shows raw errors', () => {
    setup({ properties: undefined, error: new Error('TypeError: x is undefined at line 4') })
    expect(screen.getByRole('alert')).toHaveTextContent('Something went wrong. Please try again.')
  })
})
