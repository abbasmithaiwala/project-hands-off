import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ProjectName } from './ProjectName'

describe('ProjectName', () => {
  it('renames on Enter, trimming the text', async () => {
    const onRename = vi.fn()
    render(<ProjectName name="Untitled" onRename={onRename} />)
    await userEvent.click(screen.getByRole('button', { name: /Rename/ }))
    const input = screen.getByRole('textbox', { name: 'Project name' })
    await userEvent.clear(input)
    await userEvent.type(input, '  Living room {Enter}')
    expect(onRename).toHaveBeenCalledWith('Living room')
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
  })

  it('cancels on Escape and ignores empty or unchanged names', async () => {
    const onRename = vi.fn()
    render(<ProjectName name="Untitled" onRename={onRename} />)
    await userEvent.click(screen.getByRole('button', { name: /Rename/ }))
    await userEvent.type(screen.getByRole('textbox'), 'x{Escape}')
    expect(onRename).not.toHaveBeenCalled()

    await userEvent.click(screen.getByRole('button', { name: /Rename/ }))
    await userEvent.clear(screen.getByRole('textbox'))
    await userEvent.keyboard('{Enter}')
    expect(onRename).not.toHaveBeenCalled()
  })
})
