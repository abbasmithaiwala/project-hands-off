import { waitFor } from '@testing-library/react'
import { renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it } from 'vitest'
import { useToastStore } from '@/design/toastStore'
import { useEditorStore } from '@/state/editorStore'
import { apiError, image, mockFetch } from '@/test/utils'
import { createApiClient } from '@/api/client'
import { ApiProvider } from '@/api/ApiProvider'
import type { ReactNode } from 'react'
import { projectIdFromHash, useProjectRoute } from './useProjectRoute'

const ID = '11111111-1111-1111-1111-111111111111'

function setup(routes: Parameters<typeof mockFetch>[0]) {
  const { fetch } = mockFetch(routes)
  const client = createApiClient({ baseUrl: 'http://localhost', fetch })
  const wrapper = ({ children }: { children: ReactNode }) => (
    <ApiProvider client={client}>{children}</ApiProvider>
  )
  return renderHook(() => useProjectRoute(), { wrapper })
}

beforeEach(() => {
  useEditorStore.getState().reset()
  window.history.replaceState(null, '', '/')
})

describe('projectIdFromHash', () => {
  it('accepts only well-formed project addresses', () => {
    expect(projectIdFromHash(`#/p/${ID}`)).toBe(ID)
    expect(projectIdFromHash('#/p/not-an-id')).toBeNull()
    expect(projectIdFromHash('#/other')).toBeNull()
    expect(projectIdFromHash('')).toBeNull()
  })
})

describe('useProjectRoute', () => {
  it('reopens the project named in the address', async () => {
    window.history.replaceState(null, '', `/#/p/${ID}`)
    setup({ [`GET /api/v1/images/${ID}/history`]: () => [{ ...image, id: ID }] })
    await waitFor(() => expect(useEditorStore.getState().image?.id).toBe(ID))
    expect(window.location.hash).toBe(`#/p/${ID}`)
  })

  it('points the address at the open project and clears it when the editor closes', async () => {
    const { rerender } = setup({})
    await waitFor(() => expect(useEditorStore.getState()).toBeDefined())
    useEditorStore.getState().openImage({ ...image, id: ID })
    await waitFor(() => expect(window.location.hash).toBe(`#/p/${ID}`))
    useEditorStore.getState().reset()
    rerender()
    await waitFor(() => expect(window.location.hash).toBe(''))
  })

  it('explains when a project cannot be reopened and clears the address', async () => {
    window.history.replaceState(null, '', `/#/p/${ID}`)
    setup({
      [`GET /api/v1/images/${ID}/history`]: () => apiError('not_found', 'image not found', 404),
    })
    await waitFor(() => expect(useToastStore.getState().toasts[0]?.message).toMatch(/reopened/))
    expect(window.location.hash).toBe('')
    expect(useEditorStore.getState().image).toBeNull()
  })
})
