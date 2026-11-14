import { describe, expect, it, vi } from 'vitest'
import { createApiClient, uploadImage } from './client'

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

function clientWith(response: Response) {
  const fetchMock = vi.fn(async (_request: Request) => response)
  return {
    client: createApiClient({
      baseUrl: 'http://localhost',
      fetch: fetchMock as unknown as typeof fetch,
    }),
    fetchMock,
  }
}

describe('api client', () => {
  it('returns typed data from the health endpoint', async () => {
    const { client, fetchMock } = clientWith(jsonResponse({ status: 'ok', version: '0.1.0' }))

    const { data, error } = await client.GET('/api/v1/health')

    expect(error).toBeUndefined()
    expect(data?.status).toBe('ok')
    expect(fetchMock.mock.calls[0]?.[0].url).toContain('/api/v1/health')
  })

  it('substitutes path parameters', async () => {
    const { client, fetchMock } = clientWith(jsonResponse({ code: 'not_found', message: 'x' }, 404))

    const { error } = await client.GET('/api/v1/images/{id}', {
      params: { path: { id: 'abc' } },
    })

    expect(error?.code).toBe('not_found')
    expect(fetchMock.mock.calls[0]?.[0].url).toContain('/api/v1/images/abc')
  })

  it('uploads the file as multipart form data', async () => {
    const { client, fetchMock } = clientWith(jsonResponse({ id: 'i' }, 201))
    const file = new Blob(['pixels'], { type: 'image/png' })

    await uploadImage(client, file)

    const request = fetchMock.mock.calls[0]?.[0]
    expect(request?.method).toBe('POST')
    expect(request?.headers.get('content-type')).toContain('multipart/form-data')
    const form = await request?.formData()
    const uploaded = form?.get('file') as File | null
    expect(uploaded?.size).toBe(file.size)
    expect(uploaded?.type).toBe('image/png')
  })
})
