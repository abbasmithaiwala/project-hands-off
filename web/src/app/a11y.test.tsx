import { act, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { configureAxe } from 'vitest-axe'
import { describe, expect, it, beforeAll } from 'vitest'
import { App } from '@/app/App'
import { useEditorStore } from '@/state/editorStore'
import {
  detectedObject,
  fakeStage,
  image,
  installFakeEventSource,
  renderWithProviders,
} from '@/test/utils'

// jsdom cannot compute colours, so contrast is checked in the real browser instead.
const axe = configureAxe({ rules: { 'color-contrast': { enabled: false } } })

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
      draft_generator: 'a/b',
      fallback: 'a/b',
    },
  }),
}

// The editor is loaded on demand; load it once up front so tests are not at the mercy of module-loading speed.
beforeAll(() => import('@/features/editor/EditorScreen'), 60_000)

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

  it('editing controls and the pending-changes list have no violations', async () => {
    installFakeEventSource()
    const stage = fakeStage()
    const { container } = renderWithProviders(<App createStage={stage.createStage} />, {
      ...routes,
      'POST /api/v1/images/img-1/objects/0/widget': () => ({
        kind: 'slider',
        axis: { start: { x: 0.1, y: 0.5 }, end: { x: 0.9, y: 0.5 } },
        min: 0.5,
        max: 1.5,
        initial: 1,
      }),
    })
    act(() => useEditorStore.getState().openImage(image))
    await screen.findByTestId('canvas-stage')
    act(() => {
      useEditorStore.getState().addObject(detectedObject(0, 'couch'))
      useEditorStore.getState().select(0)
      stage.callbacks().onViewChange({ scale: 0.5, x: 0, y: 0 })
    })
    await userEvent.click(await screen.findByRole('button', { name: 'Couch length' }))
    const handle = await screen.findByRole('slider', { name: 'Couch length of couch' })
    handle.focus()
    await userEvent.keyboard('{ArrowLeft}')
    await userEvent.click(await screen.findByRole('button', { name: '1 change' }))
    expect(await axe(container)).toHaveNoViolations()
  })

  it('the review state (comparison slider and actions) has no violations', async () => {
    installFakeEventSource()
    const stage = fakeStage()
    const { container } = renderWithProviders(<App createStage={stage.createStage} />, routes)
    act(() => useEditorStore.getState().openImage(image))
    await screen.findByTestId('canvas-stage')
    act(() => {
      stage.callbacks().onViewChange({ scale: 0.5, x: 0, y: 0 })
      useEditorStore.getState().finishApply('job-9', {
        summary: 'Shortened the couch',
        changes: [{ edit_index: 0, text: 'Shortened the couch to about 60% of its length' }],
        instruction: 'Make the couch shorter.',
        model: 'google/gemini-3-pro-image',
        cost_usd: 0.14,
        generation_ms: 19200,
      })
    })
    await screen.findByRole('slider', { name: 'Compare before and after' })
    expect(await axe(container)).toHaveNoViolations()
  })

  it('the progress state has no violations', async () => {
    installFakeEventSource()
    const stage = fakeStage()
    const { container } = renderWithProviders(<App createStage={stage.createStage} />, routes)
    act(() => useEditorStore.getState().openImage(image))
    await screen.findByTestId('canvas-stage')
    act(() => useEditorStore.getState().beginApply())
    await screen.findByRole('status', { name: 'Applying your edits' })
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

  it('landing with saved projects has no violations', async () => {
    const { container } = renderWithProviders(<App />, {
      'GET /api/v1/images': () => [{ ...image, name: 'Living room' }],
    })
    await screen.findByRole('button', { name: 'Open Living room' })
    expect(await axe(container)).toHaveNoViolations()
  })

  it('history panel, export menu and first-run tip have no violations', async () => {
    installFakeEventSource()
    window.localStorage.clear()
    const stage = fakeStage()
    const { container } = renderWithProviders(<App createStage={stage.createStage} />, routes)
    act(() => {
      useEditorStore.getState().openProject([
        image,
        {
          ...image,
          id: 'img-2',
          edit: {
            summary: 'Shortened the couch',
            changes: [],
            instruction: 'i',
            model: 'm',
            cost_usd: null,
            generation_ms: 1,
          },
        },
      ])
      useEditorStore.getState().addObject(detectedObject(0, 'couch'))
    })
    await userEvent.click(await screen.findByRole('button', { name: 'History' }))
    await userEvent.click(screen.getByRole('button', { name: 'Export' }))
    await screen.findByRole('complementary', { name: 'Version history' })
    await screen.findByRole('group', { name: 'Export options' })
    expect(await axe(container)).toHaveNoViolations()
  })
})
