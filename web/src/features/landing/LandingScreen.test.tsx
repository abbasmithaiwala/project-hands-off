import { fireEvent, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { useToastStore } from '@/design/toastStore'
import { useEditorStore } from '@/state/editorStore'
import { apiError, image, renderWithProviders } from '@/test/utils'
import { LandingScreen } from './LandingScreen'

const png = () => new File(['pixels'], 'photo.png', { type: 'image/png' })

describe('LandingScreen', () => {
  it('invites the user to add a photo', () => {
    renderWithProviders(<LandingScreen />)
    expect(
      screen.getByRole('heading', { name: 'Edit by dragging, not typing.' }),
    ).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Choose an image' })).toBeEnabled()
  })

  it('uploads the chosen file and opens it in the editor', async () => {
    const { calls } = renderWithProviders(<LandingScreen />, { 'POST /api/v1/images': () => image })

    await userEvent.upload(screen.getByTestId('file-input'), png())

    await waitFor(() => expect(useEditorStore.getState().image?.id).toBe('img-1'))
    expect(calls[0]?.key).toBe('POST /api/v1/images')
    expect(calls[0]?.request.headers.get('content-type')).toContain('multipart/form-data')
  })

  it('rejects unsupported files without calling the server', () => {
    const { calls } = renderWithProviders(<LandingScreen />)
    fireEvent.change(screen.getByTestId('file-input'), {
      target: { files: [new File(['x'], 'doc.pdf', { type: 'application/pdf' })] },
    })
    expect(useToastStore.getState().toasts[0]?.message).toMatch(/JPEG, PNG or WebP/)
    expect(calls).toHaveLength(0)
    expect(useEditorStore.getState().image).toBeNull()
  })

  it('shows the server message when the upload fails', async () => {
    renderWithProviders(<LandingScreen />, {
      'POST /api/v1/images': () => apiError('payload_too_large', 'image is too large', 413),
    })
    await userEvent.upload(screen.getByTestId('file-input'), png())
    await waitFor(() =>
      expect(useToastStore.getState().toasts[0]?.message).toBe('image is too large'),
    )
    expect(useEditorStore.getState().image).toBeNull()
  })

  it('accepts dropped files and highlights the drop zone while dragging', async () => {
    renderWithProviders(<LandingScreen />, { 'POST /api/v1/images': () => image })
    const zone = screen.getByTestId('drop-zone')

    fireEvent.dragOver(zone)
    expect(zone).toHaveAttribute('data-dragging', 'true')
    fireEvent.dragLeave(zone)
    expect(zone).toHaveAttribute('data-dragging', 'false')

    fireEvent.drop(zone, { dataTransfer: { files: [png()] } })
    await waitFor(() => expect(useEditorStore.getState().image).not.toBeNull())
  })

  it('accepts images pasted from the clipboard', async () => {
    renderWithProviders(<LandingScreen />, { 'POST /api/v1/images': () => image })
    const event = new Event('paste') as ClipboardEvent
    Object.defineProperty(event, 'clipboardData', { value: { files: [png()] } })
    window.dispatchEvent(event)
    await waitFor(() => expect(useEditorStore.getState().image).not.toBeNull())
  })
})
