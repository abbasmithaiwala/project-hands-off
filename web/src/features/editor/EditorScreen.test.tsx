import { act, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Widget } from '@/api/client'
import { useToastStore } from '@/design/toastStore'
import { useEditorStore } from '@/state/editorStore'
import {
  apiError,
  detectedObject,
  fakeStage,
  image,
  installFakeEventSource,
  renderWithProviders,
  type Routes,
} from '@/test/utils'
import { EditorScreen } from './EditorScreen'

const slider: Widget = {
  kind: 'slider',
  axis: { start: { x: 0.1, y: 0.5 }, end: { x: 0.9, y: 0.5 } },
  min: 0.5,
  max: 1.5,
  initial: 1,
}

const shape: Widget = {
  kind: 'control_points',
  points: [
    { x: 0.2, y: 0.2 },
    { x: 0.6, y: 0.2 },
    { x: 0.4, y: 0.6 },
  ],
}

const properties = {
  properties: [
    { key: 'length', label: 'Couch length', widget: 'slider' },
    { key: 'shape', label: 'Outline shape', widget: 'control_points' },
  ],
}

const baseRoutes: Routes = {
  'POST /api/v1/images/img-1/analyze': () => ({ job_id: 'job-1' }),
  'POST /api/v1/images/img-1/objects/0/properties': () => properties,
  'POST /api/v1/images/img-1/objects/0/widget': () => slider,
  'GET /api/v1/settings': () => ({
    openrouter: { configured: true, origin: 'stored' },
    models: {},
  }),
}

async function open(routes: Routes = {}) {
  const events = installFakeEventSource()
  const stage = fakeStage()
  const view = renderWithProviders(<EditorScreen image={image} createStage={stage.createStage} />, {
    ...baseRoutes,
    ...routes,
  })
  await waitFor(() => expect(events.instances).toHaveLength(1))
  return { ...view, events, stage }
}

function emit(
  events: ReturnType<typeof installFakeEventSource>,
  event: { type: string } & Record<string, unknown>,
) {
  act(() => events.latest()?.emit(event))
}

async function withObjects(routes: Routes = {}) {
  const context = await open(routes)
  emit(context.events, { type: 'object_found', object: detectedObject(0, 'couch') })
  emit(context.events, { type: 'completed', result: null })
  await screen.findByText('1 object found')
  await waitFor(() => expect(context.stage.handle.setObjects).toHaveBeenCalled())
  return context
}

beforeEach(() => {
  useEditorStore.getState().openImage(image)
})

describe('EditorScreen analysis', () => {
  it('requests analysis for the image and follows the job', async () => {
    const { events } = await open()
    expect(events.latest()?.url).toBe('/api/v1/jobs/job-1/events')
    expect(screen.getByText('Finding objects…')).toBeInTheDocument()
  })

  it('streams objects into the canvas and reports completion', async () => {
    const { events, stage } = await open()
    emit(events, { type: 'progress', stage: 'detecting', message: 'Finding objects' })
    emit(events, { type: 'object_found', object: detectedObject(0, 'couch') })
    expect(await screen.findByText('Finding objects · 1 so far')).toBeInTheDocument()
    emit(events, { type: 'completed', result: null })

    expect(await screen.findByText('1 object found')).toBeInTheDocument()
    await waitFor(() =>
      expect(stage.handle.setObjects).toHaveBeenLastCalledWith([detectedObject(0, 'couch')]),
    )
    expect(events.latest()?.closed).toBe(true)
  })

  it('shows what the models are doing in the activity panel', async () => {
    const { events } = await open()
    emit(events, { type: 'log', line: 'Loading detection model' })
    await userEvent.click(screen.getByRole('button', { name: 'Activity' }))
    expect(screen.getByRole('complementary', { name: 'Activity log' })).toHaveTextContent(
      'Loading detection model',
    )
    await userEvent.click(screen.getByRole('button', { name: 'Close activity' }))
    expect(screen.queryByRole('complementary')).not.toBeInTheDocument()
  })

  it('tells the user when the job fails', async () => {
    const { events } = await open()
    emit(events, {
      type: 'failed',
      code: 'unavailable',
      message: 'model is not installed: run `just models`',
    })
    expect(await screen.findByText('Analysis failed')).toBeInTheDocument()
    expect(useToastStore.getState().toasts[0]?.message).toContain('just models')
  })

  it('tells the user when the request itself fails', async () => {
    installFakeEventSource()
    renderWithProviders(<EditorScreen image={image} createStage={fakeStage().createStage} />, {
      'POST /api/v1/images/img-1/analyze': () => apiError('not_found', 'image not found', 404),
    })
    expect(await screen.findByText('Analysis failed')).toBeInTheDocument()
    expect(useToastStore.getState().toasts[0]?.message).toBe('image not found')
  })

  it('reports a dropped connection', async () => {
    const { events } = await open()
    act(() => events.latest()?.onerror?.())
    expect(await screen.findByText('Analysis failed')).toBeInTheDocument()
  })
})

describe('EditorScreen editing', () => {
  it('loads the image into the canvas and highlights the selection', async () => {
    const { stage } = await withObjects()
    expect(stage.handle.setImage).toHaveBeenCalledWith('/api/v1/images/img-1/original')

    act(() => stage.callbacks().onSelect(0))
    await waitFor(() => expect(stage.handle.setHighlight).toHaveBeenLastCalledWith(null, 0))
  })

  it('suggests properties for the selected object', async () => {
    const { stage } = await withObjects()
    act(() => stage.callbacks().onSelect(0))

    expect(await screen.findByRole('region', { name: 'Edit couch' })).toBeInTheDocument()
    expect(await screen.findByRole('button', { name: 'Couch length' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Outline shape' })).toBeInTheDocument()
  })

  it('shows the planned control as an accessible slider when a property is picked', async () => {
    const { stage } = await withObjects()
    act(() => stage.callbacks().onSelect(0))
    await userEvent.click(await screen.findByRole('button', { name: 'Couch length' }))
    act(() => stage.callbacks().onViewChange({ scale: 0.5, x: 0, y: 0 }))

    const handle = await screen.findByRole('slider', { name: 'Couch length of couch' })
    expect(handle).toHaveAttribute('aria-valuenow', '100')
    expect(screen.getByRole('button', { name: 'Couch length' })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
  })

  it('removes the control when the selection is cleared', async () => {
    const { stage } = await withObjects()
    act(() => stage.callbacks().onSelect(0))
    await userEvent.click(await screen.findByRole('button', { name: 'Couch length' }))
    act(() => stage.callbacks().onViewChange({ scale: 0.5, x: 0, y: 0 }))
    await screen.findByRole('slider')

    act(() => stage.callbacks().onSelect(null))
    await waitFor(() => expect(screen.queryByRole('slider')).not.toBeInTheDocument())
    expect(screen.queryByRole('region', { name: 'Edit couch' })).not.toBeInTheDocument()
  })

  it('announces the selection to screen readers', async () => {
    const { stage } = await withObjects()
    act(() => stage.callbacks().onSelect(0))
    expect(await screen.findByText('Selected couch')).toBeInTheDocument()
  })

  it('moves between objects with the keyboard', async () => {
    const { stage } = await withObjects()
    const canvas = screen.getByTestId('canvas-stage')
    canvas.focus()

    await userEvent.keyboard('{ArrowRight}')
    expect(useEditorStore.getState().selectedId).toBe(0)
    await userEvent.keyboard('{Escape}')
    expect(useEditorStore.getState().selectedId).toBeNull()

    await userEvent.keyboard('+')
    await userEvent.keyboard('-')
    await userEvent.keyboard('0')
    expect(stage.handle.zoomBy).toHaveBeenCalledTimes(2)
    expect(stage.handle.fit).toHaveBeenCalledTimes(1)
  })

  it('starts a new image from the top bar', async () => {
    await withObjects()
    await userEvent.click(screen.getByRole('button', { name: 'New image' }))
    expect(useEditorStore.getState().image).toBeNull()
  })

  it('destroys the canvas when it unmounts', async () => {
    const { stage, unmount } = await open()
    unmount()
    expect(stage.handle.destroy).toHaveBeenCalled()
  })
})

describe('EditorScreen edits', () => {
  async function editing() {
    const context = await withObjects({
      'POST /api/v1/images/img-1/objects/0/widget': async (request) => {
        const { property } = (await request.clone().json()) as { property: string }
        return property === 'shape' ? shape : slider
      },
    })
    act(() => context.stage.callbacks().onSelect(0))
    await userEvent.click(await screen.findByRole('button', { name: 'Couch length' }))
    act(() => context.stage.callbacks().onViewChange({ scale: 0.5, x: 0, y: 0 }))
    const handle = await screen.findByRole('slider', { name: 'Couch length of couch' })
    handle.focus()
    return { ...context, handle }
  }

  it('records a typed, normalised edit and previews it on the canvas', async () => {
    const { stage } = await editing()
    await userEvent.keyboard('{ArrowLeft}')

    const [pending] = useEditorStore.getState().pendingEdits
    expect(pending?.edit).toEqual({
      object_id: 0,
      property: 'length',
      change: { kind: 'slider', value: expect.closeTo(0.95, 5) },
    })
    expect(pending).toMatchObject({
      key: '0:length',
      objectLabel: 'couch',
      propertyLabel: 'Couch length',
    })

    await waitFor(() => expect(stage.handle.setPreview).toHaveBeenCalled())
    const [objectId, spec] = vi.mocked(stage.handle.setPreview).mock.calls.at(-1) ?? []
    expect(objectId).toBe(0)
    expect(spec?.maskUrl).toBe('/api/v1/images/img-1/objects/0/mask')
    // The axis runs from x=120 to x=1080 (0.1 and 0.9 of 1200 px); 95% of it ends at x=1032.
    const moved = spec?.deform?.({ x: 1080, y: 400 })
    expect(moved?.x).toBeCloseTo(1032, 3)
    expect(moved?.y).toBeCloseTo(400, 3)
    expect(screen.getByRole('note')).toHaveTextContent('Preview is approximate')
  })

  it('shows the pending change in the action bar and marks the property as edited', async () => {
    await editing()
    await userEvent.keyboard('{ArrowLeft}{ArrowLeft}')
    await userEvent.click(await screen.findByRole('button', { name: '1 change' }))
    expect(screen.getByRole('list', { name: 'Pending changes' })).toHaveTextContent(
      'Couch length · couch90%',
    )
    expect(screen.getByRole('img', { name: 'edited' })).toBeInTheDocument()
  })

  it('drops the edit and its preview when the control returns to where it started', async () => {
    const { stage } = await editing()
    await userEvent.keyboard('{ArrowLeft}')
    expect(useEditorStore.getState().pendingEdits).toHaveLength(1)

    await userEvent.keyboard('{ArrowRight}')
    expect(useEditorStore.getState().pendingEdits).toHaveLength(0)
    await waitFor(() => expect(stage.handle.setPreview).toHaveBeenLastCalledWith(0, null))
  })

  it('resets one edit from the list and restores the control', async () => {
    const { handle } = await editing()
    await userEvent.keyboard('{ArrowLeft}')
    await userEvent.click(await screen.findByRole('button', { name: '1 change' }))
    await userEvent.click(screen.getByRole('button', { name: 'Reset Couch length · couch' }))

    expect(useEditorStore.getState().pendingEdits).toEqual([])
    expect(handle).toHaveAttribute('aria-valuenow', '100')
  })

  it('cancels every pending edit', async () => {
    await editing()
    await userEvent.keyboard('{ArrowLeft}')
    await userEvent.click(await screen.findByRole('button', { name: 'Cancel' }))
    expect(useEditorStore.getState().pendingEdits).toEqual([])
    expect(screen.queryByRole('group', { name: 'Pending edits' })).not.toBeInTheDocument()
  })

  it('keeps an edit when another property is chosen, and restores the control later', async () => {
    await editing()
    await userEvent.keyboard('{ArrowLeft}{ArrowLeft}')

    await userEvent.click(screen.getByRole('button', { name: /Outline shape/ }))
    await waitFor(() => expect(screen.queryByRole('slider')).not.toBeInTheDocument())
    expect(useEditorStore.getState().pendingEdits).toHaveLength(1)

    await userEvent.click(screen.getByRole('button', { name: /Couch length/ }))
    expect(await screen.findByRole('slider', { name: 'Couch length of couch' })).toHaveAttribute(
      'aria-valuenow',
      '90',
    )
  })

  it('keeps previewing an edited object after it is deselected', async () => {
    const { stage } = await editing()
    await userEvent.keyboard('{ArrowLeft}')
    act(() => stage.callbacks().onSelect(null))
    await waitFor(() =>
      expect(screen.queryByRole('region', { name: 'Edit couch' })).not.toBeInTheDocument(),
    )
    expect(vi.mocked(stage.handle.setPreview).mock.calls.at(-1)?.[1]).not.toBeNull()
    expect(screen.getByRole('button', { name: '1 change' })).toBeInTheDocument()
  })

  it('says Apply is not available yet', async () => {
    await editing()
    await userEvent.keyboard('{ArrowLeft}')
    const apply = await screen.findByRole('button', { name: 'Apply' })
    expect(apply).toBeDisabled()
    expect(apply).toHaveAccessibleDescription(/not connected yet/)
  })
})

describe('EditorScreen API key prompt', () => {
  const keyMissing = () =>
    apiError('unavailable', 'An OpenRouter API key is required. Add one in Settings.', 503)

  it('asks for the key only when an AI call needs it, then retries', async () => {
    let configured = false
    const { stage, calls } = await withObjects({
      'POST /api/v1/images/img-1/objects/0/properties': () =>
        configured ? properties : keyMissing(),
      'PUT /api/v1/settings/openrouter-key': () => {
        configured = true
        return new Response(null, { status: 204 })
      },
    })
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()

    act(() => stage.callbacks().onSelect(0))
    const dialog = await screen.findByRole('dialog', { name: 'Connect OpenRouter' })
    await userEvent.type(screen.getByLabelText('OpenRouter API key'), 'sk-or-v1-abcdef')
    await userEvent.click(screen.getByRole('button', { name: 'Save key' }))

    expect(await screen.findByRole('button', { name: 'Couch length' })).toBeInTheDocument()
    await waitFor(() => expect(dialog).not.toBeInTheDocument())
    expect(calls.filter((c) => c.key.endsWith('/properties')).length).toBeGreaterThanOrEqual(2)
  })

  it('stays out of the way once dismissed', async () => {
    const { stage } = await withObjects({
      'POST /api/v1/images/img-1/objects/0/properties': keyMissing,
    })
    act(() => stage.callbacks().onSelect(0))
    await screen.findByRole('dialog', { name: 'Connect OpenRouter' })
    await userEvent.click(screen.getByRole('button', { name: 'Not now' }))

    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    expect(screen.getByRole('alert')).toHaveTextContent('API key')
  })

  it('offers the key dialog from settings', async () => {
    await open({
      'GET /api/v1/settings': () => ({
        openrouter: { configured: false, origin: 'none' },
        models: {},
      }),
    })
    await userEvent.click(screen.getByRole('button', { name: 'Settings' }))
    await userEvent.click(await screen.findByRole('button', { name: 'Add key' }))
    expect(await screen.findByRole('dialog', { name: 'Connect OpenRouter' })).toBeInTheDocument()
  })
})
