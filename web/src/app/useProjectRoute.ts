import { useEffect, useRef } from 'react'
import { useApi } from '@/api/apiContext'
import { describeError, unwrap } from '@/api/errors'
import { toast } from '@/design/toastStore'
import { useEditorStore } from '@/state/editorStore'

const PREFIX = '#/p/'

/** The project id in a location hash such as `#/p/<id>`, or `null`. */
export function projectIdFromHash(hash: string): string | null {
  if (!hash.startsWith(PREFIX)) return null
  const id = hash.slice(PREFIX.length)
  return /^[0-9a-f-]{36}$/i.test(id) ? id : null
}

function writeHash(imageId: string | null): void {
  const wanted = imageId ? `${PREFIX}${imageId}` : ''
  if (window.location.hash === wanted) return
  window.history.replaceState(null, '', window.location.pathname + window.location.search + wanted)
}

/**
 * Keep the address bar pointing at the open project so a reload (or a bookmark) reopens it.
 *
 * The version being viewed is not part of the address: reopening always shows the latest one.
 */
export function useProjectRoute() {
  const api = useApi()
  const imageId = useEditorStore((s) => s.image?.id ?? null)
  const settled = useRef(false)

  useEffect(() => {
    const id = projectIdFromHash(window.location.hash)
    if (!id) {
      settled.current = true
      return
    }
    unwrap(api.GET('/api/v1/images/{id}/history', { params: { path: { id } } }))
      .then((versions) => useEditorStore.getState().openProject(versions))
      .catch((error: unknown) => {
        toast.error(`That project could not be reopened. ${describeError(error)}`)
      })
      .finally(() => {
        settled.current = true
        writeHash(useEditorStore.getState().image?.id ?? null)
      })
  }, [api])

  useEffect(() => {
    if (settled.current) writeHash(imageId)
  }, [imageId])
}
