import { act, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { axe } from 'vitest-axe'
import { describe, expect, it } from 'vitest'
import { App } from '@/app/App'
import { useEditorStore } from '@/state/editorStore'
import {
  detectedObject,
  fakeStage,
  image,
  installFakeEventSource,
  renderWithProviders,
} from '@/test/utils'

const routes = {
  'POST /api/v1/images/img-1/analyze': () => ({ job_id: 'job-1' }),
  'POST /api/v1/images/img-1/objects/0/properties': () => ({
    properties: [{ key: 'length', label: 'Couch length', widget: 'slider' }],
  }),
  'GET /api/v1/settings': () => ({
    openrouter: { configured: false, origin: 'none' },
    models: {
      suggester: 'a/b',
      ui_planner: 'a/b',
      summariser: 'a/b',
      generator: 'a/b',
      fallback: 'a/b',
    },
  }),
}

describe('accessibility (axe)', () => {
  it('landing screen has no violations', async () => {
    const { container } = renderWithProviders(<App />)
    expect(await axe(container)).toHaveNoViolations()
  })

  it('editor with a selected object and property menu has no violations', async () => {
    installFakeEventSource()
    const stage = fakeStage()
    const { container } = renderWithProviders(<App createStage={stage.createStage} />, routes)
    act(() => useEditorStore.getState().openImage(image))
    await screen.findByTestId('canvas-stage')
    act(() => {
      useEditorStore.getState().addObject(detectedObject(0, 'couch'))
      useEditorStore.getState().select(0)
    })
    await screen.findByRole('button', { name: 'Couch length' })
    expect(await axe(container)).toHaveNoViolations()
  })

  it('settings dialog has no violations', async () => {
    installFakeEventSource()
    renderWithProviders(<App createStage={fakeStage().createStage} />, routes)
    act(() => useEditorStore.getState().openImage(image))
    await userEvent.click(await screen.findByRole('button', { name: 'Settings' }))
    await screen.findByRole('dialog', { name: 'Settings' })
    expect(await axe(document.body)).toHaveNoViolations()
  })

  it('API key dialog has no violations', async () => {
    installFakeEventSource()
    renderWithProviders(<App createStage={fakeStage().createStage} />, routes)
    act(() => useEditorStore.getState().openImage(image))
    await userEvent.click(await screen.findByRole('button', { name: 'Settings' }))
    await userEvent.click(await screen.findByRole('button', { name: 'Add key' }))
    await screen.findByRole('dialog', { name: 'Connect OpenRouter' })
    expect(await axe(document.body)).toHaveNoViolations()
  })
})
