import { describe, expect, it, vi } from 'vitest'
import type { JobEvent } from './client'
import { subscribeToJob } from './jobEvents'

class FakeSource {
  closed = false
  onerror: (() => void) | null = null
  private listeners = new Map<string, Array<(m: MessageEvent<string>) => void>>()

  addEventListener(type: string, listener: (m: MessageEvent<string>) => void) {
    this.listeners.set(type, [...(this.listeners.get(type) ?? []), listener])
  }

  close() {
    this.closed = true
  }

  emit(type: string, data: unknown) {
    const message = {
      data: typeof data === 'string' ? data : JSON.stringify(data),
    } as MessageEvent<string>
    this.listeners.get(type)?.forEach((listener) => listener(message))
  }
}

function setup() {
  const source = new FakeSource()
  const events: JobEvent[] = []
  const lost = vi.fn()
  const urls: string[] = []
  const stop = subscribeToJob(
    'job-1',
    (event) => events.push(event),
    lost,
    (url) => {
      urls.push(url)
      return source as unknown as EventSource
    },
  )
  return { source, events, lost, urls, stop }
}

describe('subscribeToJob', () => {
  it('connects to the job event stream', () => {
    expect(setup().urls).toEqual(['/api/v1/jobs/job-1/events'])
  })

  it('forwards typed events in order', () => {
    const { source, events } = setup()
    source.emit('progress', { type: 'progress', stage: 'detecting', message: 'Finding objects' })
    source.emit('log', { type: 'log', line: 'hello' })
    expect(events.map((e) => e.type)).toEqual(['progress', 'log'])
    expect(source.closed).toBe(false)
  })

  it.each(['completed', 'failed', 'cancelled'])('closes after the %s event', (type) => {
    const { source, events } = setup()
    source.emit(type, { type, result: null, code: 'internal', message: 'x' })
    expect(events).toHaveLength(1)
    expect(source.closed).toBe(true)
  })

  it('ignores malformed payloads', () => {
    const { source, events } = setup()
    source.emit('log', 'not json{')
    expect(events).toHaveLength(0)
  })

  it('reports a lost connection once, but not after a normal finish', () => {
    const lost = setup()
    lost.source.onerror?.()
    lost.source.onerror?.()
    expect(lost.lost).toHaveBeenCalledTimes(1)
    expect(lost.source.closed).toBe(true)

    const finished = setup()
    finished.source.emit('completed', { type: 'completed', result: null })
    finished.source.onerror?.()
    expect(finished.lost).not.toHaveBeenCalled()
  })

  it('can be stopped by the caller', () => {
    const { source, stop } = setup()
    stop()
    expect(source.closed).toBe(true)
  })
})
