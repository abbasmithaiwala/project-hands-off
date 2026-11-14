import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { image } from '@/test/utils'
import { TopBar } from './TopBar'

function setup(overrides: Partial<Parameters<typeof TopBar>[0]> = {}) {
  const props = {
    projectName: 'Untitled',
    versions: [image],
    current: image,
    canUndo: true,
    canRedo: false,
    historyOpen: false,
    onRename: vi.fn(),
    onUndo: vi.fn(),
    onRedo: vi.fn(),
    onToggleHistory: vi.fn(),
    analysis: 'done' as const,
    objectCount: 2,
    outputOpen: false,
    onNewImage: vi.fn(),
    onToggleOutput: vi.fn(),
    onOpenSettings: vi.fn(),
    ...overrides,
  }
  render(<TopBar {...props} />)
  return props
}

describe('TopBar', () => {
  it('shows the analysis status', () => {
    setup()
    expect(screen.getByText('2 objects found')).toBeInTheDocument()
  })

  it('reflects whether the activity panel is open', () => {
    setup({ outputOpen: true })
    expect(screen.getByRole('button', { name: 'Activity' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('wires up its three actions', async () => {
    const props = setup()
    await userEvent.click(screen.getByRole('button', { name: 'New image' }))
    await userEvent.click(screen.getByRole('button', { name: 'Activity' }))
    await userEvent.click(screen.getByRole('button', { name: 'Settings' }))
    expect(props.onNewImage).toHaveBeenCalled()
    expect(props.onToggleOutput).toHaveBeenCalled()
    expect(props.onOpenSettings).toHaveBeenCalled()
  })

  it('offers undo and redo only when there is somewhere to go', async () => {
    const props = setup({ canUndo: true, canRedo: false })
    expect(screen.getByRole('button', { name: 'Redo' })).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: 'Undo' }))
    expect(props.onUndo).toHaveBeenCalled()
  })

  it('shows the project name and toggles the history', async () => {
    const props = setup({ projectName: 'Living room', historyOpen: true })
    expect(screen.getByRole('button', { name: /Living room/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'History' })).toHaveAttribute('aria-pressed', 'true')
    await userEvent.click(screen.getByRole('button', { name: 'History' }))
    expect(props.onToggleHistory).toHaveBeenCalled()
  })
})
