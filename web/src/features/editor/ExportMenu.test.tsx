import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { ImageRecord } from '@/api/client'
import * as download from '@/lib/download'
import { ExportMenu } from './ExportMenu'

const version = { id: 'abc', name: 'Living room', format: 'jpeg', created_at: 'x' } as ImageRecord

afterEach(() => vi.restoreAllMocks())

describe('ExportMenu', () => {
  it('downloads the image under a tidy file name', async () => {
    const save = vi.spyOn(download, 'downloadUrl').mockImplementation(() => undefined)
    render(<ExportMenu versions={[version]} current={version} />)
    await userEvent.click(screen.getByRole('button', { name: 'Export' }))
    await userEvent.click(screen.getByRole('button', { name: 'Download image' }))
    expect(save).toHaveBeenCalledWith('/api/v1/images/abc/original', 'living-room.jpg')
    expect(screen.queryByRole('group', { name: 'Export options' })).not.toBeInTheDocument()
  })

  it('downloads the edit log as JSON', async () => {
    const save = vi.spyOn(download, 'downloadText').mockImplementation(() => undefined)
    render(<ExportMenu versions={[version]} current={version} />)
    await userEvent.click(screen.getByRole('button', { name: 'Export' }))
    await userEvent.click(screen.getByRole('button', { name: 'Download edit log' }))
    expect(save).toHaveBeenCalledWith(
      expect.stringContaining('"versions"'),
      'living-room-edits.json',
      'application/json',
    )
  })

  it('closes on Escape', async () => {
    render(<ExportMenu versions={[version]} current={version} />)
    await userEvent.click(screen.getByRole('button', { name: 'Export' }))
    await userEvent.keyboard('{Escape}')
    expect(screen.queryByRole('group', { name: 'Export options' })).not.toBeInTheDocument()
  })
})
