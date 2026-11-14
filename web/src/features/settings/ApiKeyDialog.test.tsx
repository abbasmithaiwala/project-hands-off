import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { apiError, renderWithProviders } from '@/test/utils'
import { ApiKeyDialog } from './ApiKeyDialog'

describe('ApiKeyDialog', () => {
  it('saves a pasted key and closes', async () => {
    const onOpenChange = vi.fn()
    const { calls } = renderWithProviders(<ApiKeyDialog open onOpenChange={onOpenChange} />, {
      'PUT /api/v1/settings/openrouter-key': () => new Response(null, { status: 204 }),
    })

    await userEvent.type(screen.getByLabelText('OpenRouter API key'), '  sk-or-v1-abcdef  ')
    await userEvent.click(screen.getByRole('button', { name: 'Save key' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(await calls[0]?.request.json()).toEqual({ key: 'sk-or-v1-abcdef' })
  })

  it('cannot be submitted empty', () => {
    renderWithProviders(<ApiKeyDialog open onOpenChange={vi.fn()} />)
    expect(screen.getByRole('button', { name: 'Save key' })).toBeDisabled()
  })

  it('keeps the key masked', () => {
    renderWithProviders(<ApiKeyDialog open onOpenChange={vi.fn()} />)
    expect(screen.getByLabelText('OpenRouter API key')).toHaveAttribute('type', 'password')
  })

  it('shows the server message and stays open when the key is rejected', async () => {
    const onOpenChange = vi.fn()
    renderWithProviders(<ApiKeyDialog open onOpenChange={onOpenChange} />, {
      'PUT /api/v1/settings/openrouter-key': () =>
        apiError('validation', 'that does not look like an API key', 400),
    })

    await userEvent.type(screen.getByLabelText('OpenRouter API key'), 'nope-nope')
    await userEvent.click(screen.getByRole('button', { name: 'Save key' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('does not look like an API key')
    expect(onOpenChange).not.toHaveBeenCalled()
  })

  it('can be dismissed with "Not now"', async () => {
    const onOpenChange = vi.fn()
    renderWithProviders(<ApiKeyDialog open onOpenChange={onOpenChange} />)
    await userEvent.click(screen.getByRole('button', { name: 'Not now' }))
    expect(onOpenChange).toHaveBeenCalledWith(false)
  })
})
