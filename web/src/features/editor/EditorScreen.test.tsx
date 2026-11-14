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
})

describe('EditorScreen applying edits', () => {
  const result = {
    summary: 'Shortened the couch',
    changes: [{ edit_index: 0, text: 'Shortened the couch to about 60% of its length' }],
    instruction: 'Make the couch shorter.',
    model: 'google/gemini-3-pro-image',
    cost_usd: 0.14,
    generation_ms: 19200,
  }
  const accepted = { ...image, id: 'img-2' }
  const settings = {
    openrouter: { configured: true, origin: 'stored' },
    models: { draft_generator: 'google/gemini-3.1-flash-image' },
  }

  async function applying(routes: Routes = {}) {
    const context = await withObjects({
      'POST /api/v1/images/img-1/objects/0/widget': () => slider,
      'POST /api/v1/images/img-1/edits': () => ({ job_id: 'job-2' }),
      'POST /api/v1/jobs/job-2/accept': () => accepted,
      'DELETE /api/v1/jobs/job-2': () => new Response(null, { status: 202 }),
      'GET /api/v1/settings': () => settings,
      ...routes,
    })
    act(() => context.stage.callbacks().onSelect(0))
    await userEvent.click(await screen.findByRole('button', { name: 'Couch length' }))
    act(() => context.stage.callbacks().onViewChange({ scale: 0.5, x: 0, y: 0 }))
    ;(await screen.findByRole('slider', { name: 'Couch length of couch' })).focus()
    await userEvent.keyboard('{ArrowLeft}{ArrowLeft}')
    return context
  }

  const editsCall = (calls: Array<{ key: string; request: Request }>, nth = 0) =>
    calls.filter((c) => c.key === 'POST /api/v1/images/img-1/edits')[nth]?.request

  async function startApply(context: Awaited<ReturnType<typeof applying>>) {
    await userEvent.click(await screen.findByRole('button', { name: 'Apply' }))
    await waitFor(() => expect(context.events.instances).toHaveLength(2))
    return context.events.instances[1]
  }

  it('sends typed edits, the affected regions and the captured preview', async () => {
    const context = await applying()
    await startApply(context)

    const body = await editsCall(context.calls)?.json()
    expect(body.edits).toEqual([
      {
        object_id: 0,
        property: 'length',
        change: {
          kind: 'slider',
          value: expect.closeTo(0.9, 5),
          axis: { start: { x: 0.1, y: 0.5 }, end: { x: 0.9, y: 0.5 } },
        },
      },
    ])
    expect(body.regions).toHaveLength(1)
    expect(body.regions[0].width).toBeGreaterThan(0)
    expect(body.preview_png).toBe('iVBORw==')
    expect(body.preview_kind).toBe('warped')
    expect(body).not.toHaveProperty('model')
    expect(context.stage.handle.capturePreview).toHaveBeenCalledWith(1280)
    expect(context.stage.handle.captureGuide).not.toHaveBeenCalled()
  })

  it('sends the new outline, not a warped preview, when only reshaping', async () => {
    const context = await withObjects({
      'POST /api/v1/images/img-1/objects/0/widget': () => shape,
      'POST /api/v1/images/img-1/edits': () => ({ job_id: 'job-2' }),
      'GET /api/v1/settings': () => settings,
    })
    act(() => context.stage.callbacks().onSelect(0))
    await userEvent.click(await screen.findByRole('button', { name: 'Outline shape' }))
    act(() => context.stage.callbacks().onViewChange({ scale: 0.5, x: 0, y: 0 }))
    const [first] = await screen.findAllByRole('button', { name: /Outline shape point 1 of 3/ })
    first?.focus()
    await userEvent.keyboard('{ArrowLeft}')
    await startApply(context)

    const body = await editsCall(context.calls)?.json()
    expect(body.preview_kind).toBe('outline')
    expect(body.preview_png).toBe('iVBORw==')
    expect(context.stage.handle.captureGuide).toHaveBeenCalledTimes(1)
    const [side, shapes] = vi.mocked(context.stage.handle.captureGuide).mock.calls[0] ?? []
    expect(side).toBe(1280)
    expect(shapes).toHaveLength(1)
    expect(shapes?.[0]).toHaveLength(3)
    expect(shapes?.[0]?.[0]?.x).toBeLessThan(0.2)
    expect(context.stage.handle.capturePreview).not.toHaveBeenCalled()
  })

  it('shows progress while generating and hides the editing controls', async () => {
    const context = await applying()
    const source = await startApply(context)
    expect(source?.url).toBe('/api/v1/jobs/job-2/events')

    expect(await screen.findByRole('status', { name: 'Applying your edits' })).toHaveTextContent(
      'Preparing your edits',
    )
    emit(context.events, {
      type: 'progress',
      stage: 'generating',
      message: 'Generating the edited image',
    })
    expect(await screen.findByText('Generating the image…')).toBeInTheDocument()
    expect(screen.queryByRole('slider')).not.toBeInTheDocument()
    expect(screen.queryByRole('region', { name: 'Edit couch' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Apply' })).not.toBeInTheDocument()
  })

  it('records the job log, including cost and time, in the activity panel', async () => {
    const context = await applying()
    const source = await startApply(context)
    act(() =>
      source?.emit({
        type: 'log',
        line: 'Generated by google/gemini-3-pro-image in 19.2s · $0.140',
      }),
    )
    await userEvent.click(screen.getByRole('button', { name: 'Activity' }))
    expect(screen.getByRole('complementary', { name: 'Activity log' })).toHaveTextContent(
      'in 19.2s · $0.140',
    )
  })

  async function reviewing(routes: Routes = {}) {
    const context = await applying(routes)
    const source = await startApply(context)
    act(() => source?.emit({ type: 'completed', result }))
    await screen.findByRole('region', { name: 'Review your edit' })
    return { ...context, source }
  }

  it('offers the result for review with a before and after comparison', async () => {
    await reviewing()
    const review = screen.getByRole('region', { name: 'Review your edit' })
    expect(review).toHaveTextContent('Shortened the couch')
    expect(review).toHaveTextContent('Shortened the couch to about 60% of its length')
    expect(review).toHaveTextContent('gemini-3-pro-image · 19.2s · $0.14')

    const before = await screen.findByAltText('Before: the original photo')
    const after = screen.getByAltText('After: the edited photo')
    expect(before).toHaveAttribute('src', '/api/v1/images/img-1/original')
    expect(after).toHaveAttribute('src', '/api/v1/jobs/job-2/result')
    expect(screen.getByRole('slider', { name: 'Compare before and after' })).toBeInTheDocument()
    expect(screen.queryByRole('note')).not.toBeInTheDocument()
  })

  it('accepting opens the result as a new image', async () => {
    const { calls } = await reviewing()
    await userEvent.click(screen.getByRole('button', { name: 'Accept' }))

    await waitFor(() => expect(useEditorStore.getState().image?.id).toBe('img-2'))
    expect(calls.some((c) => c.key === 'POST /api/v1/jobs/job-2/accept')).toBe(true)
    expect(useEditorStore.getState().pendingEdits).toEqual([])
    expect(useToastStore.getState().toasts.at(-1)?.message).toBe('Applied: Shortened the couch')
  })

  it('discarding returns to editing with the edits intact', async () => {
    await reviewing()
    await userEvent.click(screen.getByRole('button', { name: 'Discard' }))

    expect(screen.queryByRole('region', { name: 'Review your edit' })).not.toBeInTheDocument()
    expect(useEditorStore.getState().pendingEdits).toHaveLength(1)
    expect(await screen.findByRole('slider', { name: 'Couch length of couch' })).toHaveAttribute(
      'aria-valuenow',
      '90',
    )
    expect(screen.getByRole('button', { name: 'Apply' })).toBeEnabled()
  })

  it('retrying sends the same edits again with the same model', async () => {
    const context = await reviewing()
    await userEvent.click(screen.getByRole('button', { name: 'Retry' }))
    await waitFor(() => expect(context.events.instances).toHaveLength(3))
    const body = await editsCall(context.calls, 1)?.json()
    expect(body).toBeDefined()
    expect(body).not.toHaveProperty('model')
  })

  it('offers a faster draft using the configured draft model', async () => {
    const context = await reviewing()
    await userEvent.click(screen.getByRole('button', { name: 'Faster draft' }))
    await waitFor(() => expect(context.events.instances).toHaveLength(3))
    expect(await editsCall(context.calls, 1)?.json()).toMatchObject({
      model: 'google/gemini-3.1-flash-image',
    })
  })

  it('cancels a running edit and goes back to editing', async () => {
    const context = await applying()
    const source = await startApply(context)
    await screen.findByRole('status', { name: 'Applying your edits' })
    // The job id arrives once the request has been accepted.
    await waitFor(() => expect(useEditorStore.getState().apply).toMatchObject({ jobId: 'job-2' }))

    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))

    await waitFor(() =>
      expect(context.calls.some((c) => c.key === 'DELETE /api/v1/jobs/job-2')).toBe(true),
    )
    expect(source?.closed).toBe(true)
    expect(useEditorStore.getState().apply).toEqual({ status: 'idle' })
    expect(useEditorStore.getState().pendingEdits).toHaveLength(1)
  })

  it('reports a failed edit and keeps the edits so it can be retried', async () => {
    const context = await applying()
    const source = await startApply(context)
    act(() =>
      source?.emit({
        type: 'failed',
        code: 'refused',
        message: 'The image model declined this edit on content-policy grounds.',
      }),
    )

    await waitFor(() =>
      expect(useToastStore.getState().toasts.at(-1)?.message).toContain('content-policy'),
    )
    expect(useEditorStore.getState().apply).toEqual({ status: 'idle' })
    expect(useEditorStore.getState().pendingEdits).toHaveLength(1)
    expect(await screen.findByRole('button', { name: 'Apply' })).toBeEnabled()
  })

  it('asks for the key when the failure is about the key', async () => {
    const context = await applying()
    const source = await startApply(context)
    act(() =>
      source?.emit({
        type: 'failed',
        code: 'unavailable',
        message: 'An OpenRouter API key is required. Add one in Settings.',
      }),
    )
    expect(await screen.findByRole('dialog', { name: 'Connect OpenRouter' })).toBeInTheDocument()
  })

  it('reports a request the server rejects without starting a job', async () => {
    const context = await applying({
      'POST /api/v1/images/img-1/edits': () =>
        apiError('validation', 'edit 0 refers to an unknown object', 400),
    })
    await userEvent.click(await screen.findByRole('button', { name: 'Apply' }))
    await waitFor(() =>
      expect(useToastStore.getState().toasts.at(-1)?.message).toBe(
        'edit 0 refers to an unknown object',
      ),
    )
    expect(context.events.instances).toHaveLength(1)
    expect(useEditorStore.getState().apply).toEqual({ status: 'idle' })
  })

  it('sends no preview when the canvas could not capture one', async () => {
    const context = await applying()
    vi.mocked(context.stage.handle.capturePreview).mockResolvedValueOnce(null)
    await startApply(context)
    expect(await editsCall(context.calls)?.json()).not.toHaveProperty('preview_png')
  })

  it('treats a dropped connection as a failure', async () => {
    const context = await applying()
    const source = await startApply(context)
    act(() => source?.onerror?.())
    await waitFor(() =>
      expect(useToastStore.getState().toasts.at(-1)?.message).toBe(
        'Lost connection to the server.',
      ),
    )
    expect(useEditorStore.getState().apply).toEqual({ status: 'idle' })
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
