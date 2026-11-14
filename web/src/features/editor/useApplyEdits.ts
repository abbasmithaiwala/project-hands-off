import { useCallback, useEffect, useRef, type MutableRefObject } from 'react'
import { useApi } from '@/api/apiContext'
import type { EditRequest, EditResult, ImageRecord, JobEvent } from '@/api/client'
import { describeError, expectOk, mentionsApiKey, unwrap } from '@/api/errors'
import { subscribeToJob } from '@/api/jobEvents'
import { toast } from '@/design/toastStore'
import { blobToBase64 } from '@/lib/base64'
import { useEditorStore } from '@/state/editorStore'
import type { StageHandle } from './canvas/stage'
import { affectedRegions, requestEdits } from './previews'

/** The preview sent to the model is capped at this size on its longest side. */
const PREVIEW_MAX_SIDE = 1280

interface Options {
  image: ImageRecord
  stageRef: MutableRefObject<StageHandle | null>
  /** Called when a failure means the OpenRouter key is missing or rejected. */
  onKeyProblem: () => void
}

function isEditResult(value: unknown): value is EditResult {
  const result = value as Partial<EditResult> | null
  return (
    typeof result?.summary === 'string' &&
    Array.isArray(result.changes) &&
    typeof result.model === 'string'
  )
}

/** Applying the pending edits: start the job, follow it, then accept, retry or discard. */
export function useApplyEdits({ image, stageRef, onKeyProblem }: Options) {
  const api = useApi()
  const stopListening = useRef<(() => void) | null>(null)

  useEffect(() => () => stopListening.current?.(), [])

  const fail = useCallback(
    (message: string) => {
      stopListening.current?.()
      useEditorStore.getState().closeApply()
      toast.error(message)
      if (mentionsApiKey(message)) onKeyProblem()
    },
    [onKeyProblem],
  )

  const handleEvent = useCallback(
    (jobId: string) => (event: JobEvent) => {
      const state = useEditorStore.getState()
      switch (event.type) {
        case 'progress':
          state.setApplyStage(event.stage)
          state.appendLog(event.message)
          break
        case 'log':
          state.appendLog(event.line)
          break
        case 'completed':
          if (isEditResult(event.result)) state.finishApply(jobId, event.result)
          else fail('The server sent an unexpected result.')
          break
        case 'failed':
          state.appendLog(event.message)
          fail(event.message)
          break
        case 'cancelled':
          state.closeApply()
          break
        case 'object_found':
          break
      }
    },
    [fail],
  )

  const run = useCallback(
    async (model?: string) => {
      const state = useEditorStore.getState()
      state.beginApply()
      try {
        const size = { width: image.width, height: image.height }
        const blob = await stageRef.current?.capturePreview(PREVIEW_MAX_SIDE)
        const previewPng = blob ? await blobToBase64(blob) : null
        const body: EditRequest = {
          edits: requestEdits(state.pendingEdits),
          regions: affectedRegions(state.pendingEdits, state.objects, size),
          ...(previewPng ? { preview_png: previewPng } : {}),
          ...(model ? { model } : {}),
        }
        const { job_id } = await unwrap(
          api.POST('/api/v1/images/{id}/edits', { params: { path: { id: image.id } }, body }),
        )
        state.setApplyJob(job_id)
        stopListening.current = subscribeToJob(job_id, handleEvent(job_id), () =>
          fail('Lost connection to the server.'),
        )
      } catch (error) {
        fail(describeError(error))
      }
    },
    [api, image.height, image.id, image.width, stageRef, handleEvent, fail],
  )

  const cancel = useCallback(async () => {
    const { apply, closeApply } = useEditorStore.getState()
    stopListening.current?.()
    closeApply()
    if (apply.status === 'running' && apply.jobId) {
      await expectOk(
        api.DELETE('/api/v1/jobs/{id}', { params: { path: { id: apply.jobId } } }),
      ).catch(() => undefined)
    }
  }, [api])

  const accept = useCallback(async () => {
    const { apply, openImage } = useEditorStore.getState()
    if (apply.status !== 'review') return
    try {
      const record = await unwrap(
        api.POST('/api/v1/jobs/{id}/accept', { params: { path: { id: apply.jobId } } }),
      )
      toast.info(`Applied: ${apply.result.summary}`)
      openImage(record)
    } catch (error) {
      toast.error(describeError(error))
    }
  }, [api])

  const discard = useCallback(() => useEditorStore.getState().closeApply(), [])

  return { run, cancel, accept, discard }
}
