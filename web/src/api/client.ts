import createClient from 'openapi-fetch'
import type { components, paths } from './schema'

export type ApiClient = ReturnType<typeof createApiClient>
export type ImageRecord = components['schemas']['ImageRecord']
export type JobEvent = components['schemas']['JobEvent']
export type ApiErrorBody = components['schemas']['ErrorBody']

interface ClientOptions {
  baseUrl?: string
  fetch?: typeof fetch
}

export function createApiClient({ baseUrl = '', fetch }: ClientOptions = {}) {
  return createClient<paths>({ baseUrl, ...(fetch ? { fetch } : {}) })
}

export function uploadImage(client: ApiClient, file: Blob) {
  return client.POST('/api/v1/images', {
    body: { file: '' },
    bodySerializer: () => {
      const form = new FormData()
      form.append('file', file)
      return form
    },
  })
}
