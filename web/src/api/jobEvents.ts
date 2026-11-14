import type { JobEvent } from './client'

const EVENT_TYPES = ['progress', 'log', 'object_found', 'completed', 'failed', 'cancelled'] as const

type EventSourceFactory = (url: string) => EventSource

/**
 * Follow a job's server-sent events. Calls `onEvent` for each event and closes itself after
 * the terminal one. Returns a function that stops listening.
 */
export function subscribeToJob(
  jobId: string,
  onEvent: (event: JobEvent) => void,
  onConnectionLost: () => void,
  createSource: EventSourceFactory = (url) => new EventSource(url),
): () => void {
  const source = createSource(`/api/v1/jobs/${jobId}/events`)
  let finished = false

  const finish = () => {
    finished = true
    source.close()
  }

  for (const type of EVENT_TYPES) {
    source.addEventListener(type, (message) => {
      let event: JobEvent
      try {
        event = JSON.parse((message as MessageEvent<string>).data) as JobEvent
      } catch {
        return
      }
      onEvent(event)
      if (event.type === 'completed' || event.type === 'failed' || event.type === 'cancelled') {
        finish()
      }
    })
  }
  source.onerror = () => {
    if (!finished) {
      finish()
      onConnectionLost()
    }
  }
  return finish
}
