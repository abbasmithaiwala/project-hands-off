import createClient from 'openapi-fetch'
import type { components, paths } from './schema'

export type ApiClient = ReturnType<typeof createApiClient>
export type ImageRecord = components['schemas']['ImageRecord']
export type DetectedObject = components['schemas']['DetectedObject']
export type JobEvent = components['schemas']['JobEvent']
export type Property = components['schemas']['Property']
export type PropertyList = components['schemas']['PropertyList']
export type Widget = components['schemas']['Widget']
export type ModelSettings = components['schemas']['ModelSettings']
export type Edit = components['schemas']['Edit']
export type SettingsView = components['schemas']['SettingsView']
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
      form.append('file', file, file instanceof File ? file.name : 'upload')
      return form
    },
  })
}
