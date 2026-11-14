import { useEffect } from 'react'
import { useApi } from '@/api/apiContext'
import { describeError, unwrap } from '@/api/errors'
import { subscribeToJob } from '@/api/jobEvents'
import { useEditorStore } from '@/state/editorStore'

/** Analyse an image and stream the detected objects into the editor store. */
export function useAnalysis(imageId: string) {
  const api = useApi()

  useEffect(() => {
    const store = useEditorStore.getState()
    let cancelled = false
    let stop: (() => void) | undefined
    store.setAnalysis('running')

    unwrap(api.POST('/api/v1/images/{id}/analyze', { params: { path: { id: imageId } } }))
      .then(({ job_id }) => {
        if (cancelled) return
        stop = subscribeToJob(
          job_id,
          (event) => {
            const state = useEditorStore.getState()
            switch (event.type) {
              case 'progress':
                state.appendLog(event.message)
                break
              case 'log':
                state.appendLog(event.line)
                break
              case 'object_found':
                state.addObject(event.object)
                break
              case 'completed':
                state.setAnalysis('done')
                break
              case 'failed':
                state.appendLog(event.message)
                state.setAnalysis('failed', event.message)
                break
              case 'cancelled':
                state.setAnalysis('failed', 'Analysis was cancelled.')
                break
            }
          },
          () => useEditorStore.getState().setAnalysis('failed', 'Lost connection to the server.'),
        )
      })
      .catch((error: unknown) => {
        if (!cancelled) useEditorStore.getState().setAnalysis('failed', describeError(error))
      })

    return () => {
      cancelled = true
      stop?.()
    }
  }, [api, imageId])
}
