import { QueryClient } from '@tanstack/react-query'
import { render } from '@testing-library/react'
import type { ReactElement } from 'react'
import { vi } from 'vitest'
import type { ApiErrorBody, DetectedObject, ImageRecord } from '@/api/client'
import { createApiClient } from '@/api/client'
import { Providers } from '@/app/Providers'
import type { CreateStage, StageCallbacks, StageHandle } from '@/features/editor/canvas/stage'

type Handler = (request: Request) => unknown
export type Routes = Record<string, Handler>

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } })

export const apiError = (code: ApiErrorBody['code'], message: string, status: number) =>
  json({ code, message }, status)

/** A fetch that answers from `routes`, keyed like `POST /api/v1/images`. Records every call. */
export function mockFetch(routes: Routes) {
  const calls: Array<{ key: string; request: Request }> = []
  const fetchMock = vi.fn(async (input: Request | string | URL) => {
    const request = input instanceof Request ? input : new Request(input)
    const key = `${request.method} ${new URL(request.url).pathname}`
    calls.push({ key, request })
    const handler = routes[key]
    if (!handler) return apiError('not_found', `no route for ${key}`, 404)
    const result = await handler(request)
    return result instanceof Response ? result : json(result)
  })
  return { fetch: fetchMock as unknown as typeof fetch, calls }
}

export function renderWithProviders(ui: ReactElement, routes: Routes = {}) {
  const { fetch, calls } = mockFetch(routes)
  const client = createApiClient({ baseUrl: 'http://localhost', fetch })
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity }, mutations: { retry: false } },
  })
  const view = render(
    <Providers client={client} queryClient={queryClient}>
      {ui}
    </Providers>,
  )
  return {
    ...view,
    calls,
    rerenderUi: (next: ReactElement) =>
      view.rerender(
        <Providers client={client} queryClient={queryClient}>
          {next}
        </Providers>,
      ),
  }
}

export const image: ImageRecord = {
  id: 'img-1',
  hash: 'a'.repeat(64),
  format: 'jpeg',
  width: 1200,
  height: 800,
  size_bytes: 1000,
  created_at: '2026-01-01T00:00:00Z',
}

export function detectedObject(id: number, label: string): DetectedObject {
  return {
    id,
    label,
    score: 0.9,
    bbox: { x: 0.1, y: 0.1, width: 0.5, height: 0.5 },
    outline: [
      { x: 0.1, y: 0.1 },
      { x: 0.6, y: 0.1 },
      { x: 0.6, y: 0.6 },
    ],
  }
}

/** A stand-in canvas that records what it was asked to do and lets tests fire its callbacks. */
export function fakeStage() {
  const handle: StageHandle = {
    setImage: vi.fn(async () => ({ width: 1200, height: 800 })),
    setObjects: vi.fn(),
    setHighlight: vi.fn(),
    setPreview: vi.fn(),
    fit: vi.fn(),
    zoomBy: vi.fn(),
    destroy: vi.fn(),
  }
  let callbacks: StageCallbacks | null = null
  const createStage: CreateStage = async (_container, cb) => {
    callbacks = cb
    return handle
  }
  return {
    handle,
    createStage,
    callbacks: () => {
      if (!callbacks) throw new Error('the stage has not been created yet')
      return callbacks
    },
  }
}

/** Replaces `EventSource` with a controllable fake. */
export function installFakeEventSource() {
  const instances: FakeEventSource[] = []
  class FakeEventSource {
    onerror: (() => void) | null = null
    closed = false
    private listeners = new Map<string, Array<(m: MessageEvent<string>) => void>>()
    url: string
    constructor(url: string) {
      this.url = url
      instances.push(this)
    }
    addEventListener(type: string, listener: (m: MessageEvent<string>) => void) {
      this.listeners.set(type, [...(this.listeners.get(type) ?? []), listener])
    }
    close() {
      this.closed = true
    }
    emit(event: { type: string } & Record<string, unknown>) {
      const message = { data: JSON.stringify(event) } as MessageEvent<string>
      this.listeners.get(event.type)?.forEach((listener) => listener(message))
    }
  }
  vi.stubGlobal('EventSource', FakeEventSource)
  return { instances, latest: () => instances[instances.length - 1] }
}
