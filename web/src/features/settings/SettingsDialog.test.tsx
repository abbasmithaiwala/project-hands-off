import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { SettingsView } from '@/api/client'
import { renderWithProviders } from '@/test/utils'
import { SettingsDialog } from './SettingsDialog'

const models = {
  suggester: 'google/gemini-3.1-flash-lite',
  ui_planner: 'google/gemini-3.1-flash-lite',
  summariser: 'google/gemini-3.1-flash-lite',
  generator: 'google/gemini-3-pro-image',
  draft_generator: 'google/gemini-3.1-flash-image',
  fallback: 'anthropic/claude-haiku-5.5',
}

const settings = (origin: SettingsView['openrouter']['origin']): SettingsView => ({
  openrouter: { configured: origin !== 'none', origin },
  models,
})

afterEach(() => {
  window.localStorage.clear()
  document.documentElement.removeAttribute('data-theme')
})

describe('SettingsDialog', () => {
  it('switches the theme', async () => {
    renderWithProviders(<SettingsDialog open onOpenChange={vi.fn()} onChangeKey={vi.fn()} />, {
      'GET /api/v1/settings': () => settings('none'),
    })
    await userEvent.click(screen.getByRole('radio', { name: 'Dark' }))
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    expect(screen.getByRole('radio', { name: 'Dark' })).toHaveAttribute('aria-checked', 'true')
  })

  it('offers to add a key when none is connected', async () => {
    const onChangeKey = vi.fn()
    renderWithProviders(<SettingsDialog open onOpenChange={vi.fn()} onChangeKey={onChangeKey} />, {
      'GET /api/v1/settings': () => settings('none'),
    })
    expect(await screen.findByText('Not connected')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Add key' }))
    expect(onChangeKey).toHaveBeenCalled()
  })

  it('lets a saved key be replaced or removed', async () => {
    const { calls } = renderWithProviders(
      <SettingsDialog open onOpenChange={vi.fn()} onChangeKey={vi.fn()} />,
      {
        'GET /api/v1/settings': () => settings('stored'),
        'DELETE /api/v1/settings/openrouter-key': () => new Response(null, { status: 204 }),
      },
    )
    expect(await screen.findByRole('button', { name: 'Replace key' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Remove' }))
    await waitFor(() =>
      expect(calls.some((c) => c.key === 'DELETE /api/v1/settings/openrouter-key')).toBe(true),
    )
  })

  it('shows no key controls in demo mode or when the key comes from the server', async () => {
    const { unmount } = renderWithProviders(
      <SettingsDialog open onOpenChange={vi.fn()} onChangeKey={vi.fn()} />,
      {
        'GET /api/v1/settings': () => settings('demo'),
      },
    )
    expect(await screen.findByText('Demo mode: no key needed')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /key/i })).not.toBeInTheDocument()
    unmount()

    renderWithProviders(<SettingsDialog open onOpenChange={vi.fn()} onChangeKey={vi.fn()} />, {
      'GET /api/v1/settings': () => settings('environment'),
    })
    expect(await screen.findByText('Set on the server')).toBeInTheDocument()
  })

  it('saves edited models', async () => {
    const { calls } = renderWithProviders(
      <SettingsDialog open onOpenChange={vi.fn()} onChangeKey={vi.fn()} />,
      {
        'GET /api/v1/settings': () => settings('stored'),
        'PUT /api/v1/settings/models': () => settings('stored'),
      },
    )
    await userEvent.click(await screen.findByText('Advanced: models'))
    const field = await screen.findByLabelText('Suggests what can be changed')
    await userEvent.clear(field)
    await userEvent.type(field, 'typesafe/jev-router')
    await userEvent.click(screen.getByRole('button', { name: 'Save models' }))

    await waitFor(() =>
      expect(calls.some((c) => c.key === 'PUT /api/v1/settings/models')).toBe(true),
    )
    const put = calls.find((c) => c.key === 'PUT /api/v1/settings/models')
    expect(await put?.request.json()).toMatchObject({ suggester: 'typesafe/jev-router' })
  })
})
