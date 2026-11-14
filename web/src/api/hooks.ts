import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useApi } from './apiContext'
import { uploadImage, type ModelSettings, type Widget } from './client'
import { expectOk, isKeyProblem, unwrap } from './errors'

export const queryKeys = {
  settings: ['settings'] as const,
  projects: ['projects'] as const,
  properties: (imageId: string, objectId: number) => ['properties', imageId, objectId] as const,
  widget: (imageId: string, objectId: number, property: string) =>
    ['widget', imageId, objectId, property] as const,
}

export function useSettings() {
  const api = useApi()
  return useQuery({
    queryKey: queryKeys.settings,
    queryFn: () => unwrap(api.GET('/api/v1/settings')),
  })
}

export function useSaveApiKey() {
  const api = useApi()
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (key: string) =>
      expectOk(api.PUT('/api/v1/settings/openrouter-key', { body: { key } })),
    onSuccess: () => queryClient.invalidateQueries(),
  })
}

export function useClearApiKey() {
  const api = useApi()
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: () => expectOk(api.DELETE('/api/v1/settings/openrouter-key')),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: queryKeys.settings }),
  })
}

export function useSaveModels() {
  const api = useApi()
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (models: ModelSettings) =>
      unwrap(api.PUT('/api/v1/settings/models', { body: models })),
    onSuccess: (settings) => queryClient.setQueryData(queryKeys.settings, settings),
  })
}

export function useUploadImage() {
  const api = useApi()
  return useMutation({ mutationFn: (file: Blob) => unwrap(uploadImage(api, file)) })
}

/** Editable properties of an object. Retries are skipped for key problems, which need the user. */
export function useProperties(imageId: string | null, objectId: number | null) {
  const api = useApi()
  return useQuery({
    enabled: imageId !== null && objectId !== null,
    queryKey: queryKeys.properties(imageId ?? '', objectId ?? -1),
    queryFn: () =>
      unwrap(
        api.POST('/api/v1/images/{id}/objects/{object_id}/properties', {
          params: { path: { id: imageId ?? '', object_id: objectId ?? 0 } },
        }),
      ),
    staleTime: Infinity,
    retry: (count, error) => count < 1 && !isKeyProblem(error),
  })
}

export function useWidget(
  imageId: string | null,
  objectId: number | null,
  property: string | null,
) {
  const api = useApi()
  return useQuery<Widget>({
    enabled: imageId !== null && objectId !== null && property !== null,
    queryKey: queryKeys.widget(imageId ?? '', objectId ?? -1, property ?? ''),
    queryFn: () =>
      unwrap(
        api.POST('/api/v1/images/{id}/objects/{object_id}/widget', {
          params: { path: { id: imageId ?? '', object_id: objectId ?? 0 } },
          body: { property: property ?? '' },
        }),
      ),
    staleTime: Infinity,
    retry: (count, error) => count < 1 && !isKeyProblem(error),
  })
}

export function useProjects() {
  const api = useApi()
  return useQuery({
    queryKey: queryKeys.projects,
    queryFn: () => unwrap(api.GET('/api/v1/images', { params: { query: { limit: 12 } } })),
  })
}

/** Load a saved project's version history and open it in the editor. */
export function useOpenProject() {
  const api = useApi()
  return useMutation({
    mutationFn: (id: string) =>
      unwrap(api.GET('/api/v1/images/{id}/history', { params: { path: { id } } })),
  })
}

export function useRenameProject() {
  const api = useApi()
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) =>
      unwrap(api.PATCH('/api/v1/images/{id}', { params: { path: { id } }, body: { name } })),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: queryKeys.projects }),
  })
}

export function useDeleteProject() {
  const api = useApi()
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) =>
      expectOk(api.DELETE('/api/v1/images/{id}', { params: { path: { id } } })),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: queryKeys.projects }),
  })
}
