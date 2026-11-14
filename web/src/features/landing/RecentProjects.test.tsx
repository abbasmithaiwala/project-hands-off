import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { ImageRecord } from '@/api/client'
import { useEditorStore } from '@/state/editorStore'
import { image, renderWithProviders } from '@/test/utils'
import { RecentProjects } from './RecentProjects'

const project: ImageRecord = { ...image, id: 'p2', name: 'Living room' }

describe('RecentProjects', () => {
  it('shows nothing when there are no projects', async () => {
    renderWithProviders(<RecentProjects />, { 'GET /api/v1/images': () => [] })
    await waitFor(() =>
      expect(screen.queryByRole('status', { name: 'Loading recent projects' })).toBeNull(),
    )
    expect(screen.queryByRole('heading', { name: 'Recent projects' })).toBeNull()
  })

  it('opens a project with its whole version history', async () => {
    const versions = [{ ...image, id: 'p1', name: 'Living room' }, project]
    renderWithProviders(<RecentProjects />, {
      'GET /api/v1/images': () => [project],
      'GET /api/v1/images/p2/history': () => versions,
    })
    await userEvent.click(await screen.findByRole('button', { name: 'Open Living room' }))
    await waitFor(() => expect(useEditorStore.getState().image?.id).toBe('p2'))
    expect(useEditorStore.getState().versions).toHaveLength(2)
  })

  it('asks before deleting and only then removes the project', async () => {
    const { calls } = renderWithProviders(<RecentProjects />, {
      'GET /api/v1/images': () => [project],
      'DELETE /api/v1/images/p2': () => new Response(null, { status: 204 }),
    })
    await userEvent.click(await screen.findByRole('button', { name: 'Delete Living room' }))
    expect(calls.some((call) => call.key.startsWith('DELETE'))).toBe(false)

    await userEvent.click(screen.getByRole('button', { name: 'Keep it' }))
    expect(calls.some((call) => call.key.startsWith('DELETE'))).toBe(false)

    await userEvent.click(screen.getByRole('button', { name: 'Delete Living room' }))
    await userEvent.click(screen.getByRole('button', { name: 'Delete' }))
    await waitFor(() =>
      expect(calls.some((call) => call.key === 'DELETE /api/v1/images/p2')).toBe(true),
    )
  })
})
