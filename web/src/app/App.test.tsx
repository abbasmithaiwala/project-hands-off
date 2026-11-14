import { act, screen } from '@testing-library/react'
import { describe, expect, it, beforeAll } from 'vitest'
import { useEditorStore } from '@/state/editorStore'
import { fakeStage, image, installFakeEventSource, renderWithProviders } from '@/test/utils'
import { App } from './App'

// The editor is loaded on demand; load it once up front so tests are not at the mercy of module-loading speed.
beforeAll(() => import('@/features/editor/EditorScreen'), 60_000)

describe('App', () => {
  it('starts on the landing screen', () => {
    renderWithProviders(<App />)
    expect(
      screen.getByRole('heading', { name: 'Edit by dragging, not typing.' }),
    ).toBeInTheDocument()
  })

  it('switches to the editor once an image is open and back on reset', async () => {
    installFakeEventSource()
    renderWithProviders(<App createStage={fakeStage().createStage} />, {
      'POST /api/v1/images/img-1/analyze': () => ({ job_id: 'job-1' }),
    })
    act(() => useEditorStore.getState().openImage(image))
    expect(await screen.findByTestId('canvas-stage')).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: /dragging/ })).not.toBeInTheDocument()

    act(() => useEditorStore.getState().reset())
    expect(await screen.findByRole('heading', { name: /dragging/ })).toBeInTheDocument()
  })
})
