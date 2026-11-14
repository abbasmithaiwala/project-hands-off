import { describe, expect, it } from 'vitest'
import { ApiError, describeError, expectOk, isKeyProblem, mentionsApiKey, unwrap } from './errors'

const response = (status = 200) => new Response(null, { status })

describe('unwrap', () => {
  it('returns data from a successful call', async () => {
    await expect(
      unwrap(Promise.resolve({ data: { ok: 1 }, response: response() })),
    ).resolves.toEqual({ ok: 1 })
  })

  it('throws an ApiError carrying the server code, message and status', async () => {
    const call = Promise.resolve({
      error: { code: 'not_found' as const, message: 'image not found' },
      response: response(404),
    })
    await expect(unwrap(call)).rejects.toMatchObject({
      code: 'not_found',
      message: 'image not found',
      status: 404,
    })
  })

  it('turns network failures into a friendly error', async () => {
    await expect(unwrap(Promise.reject(new TypeError('Failed to fetch')))).rejects.toMatchObject({
      code: 'network',
      status: 0,
    })
  })

  it('rejects empty successful responses', async () => {
    await expect(unwrap(Promise.resolve({ response: response(204) }))).rejects.toMatchObject({
      code: 'internal',
    })
  })
})

describe('expectOk', () => {
  it('accepts bodiless success responses', async () => {
    await expect(expectOk(Promise.resolve({ response: response(204) }))).resolves.toBeUndefined()
  })

  it('still reports errors', async () => {
    const call = Promise.resolve({
      error: { code: 'validation' as const, message: 'that does not look like an API key' },
      response: response(400),
    })
    await expect(expectOk(call)).rejects.toMatchObject({ code: 'validation', status: 400 })
  })
})

describe('isKeyProblem', () => {
  it('recognises missing or rejected key errors only', () => {
    expect(
      isKeyProblem(new ApiError('unavailable', 'An OpenRouter API key is required.', 503)),
    ).toBe(true)
    expect(
      isKeyProblem(new ApiError('unavailable', 'OpenRouter rejected the API key. Check it.', 503)),
    ).toBe(true)
    expect(isKeyProblem(new ApiError('unavailable', 'model is not installed', 503))).toBe(false)
    expect(isKeyProblem(new ApiError('not_found', 'API key', 404))).toBe(false)
    expect(isKeyProblem(new Error('API key'))).toBe(false)
  })
})

describe('mentionsApiKey', () => {
  it('spots messages about the key', () => {
    expect(mentionsApiKey('An OpenRouter API key is required.')).toBe(true)
    expect(mentionsApiKey('OpenRouter rejected the api key')).toBe(true)
    expect(mentionsApiKey('Your account is out of credits.')).toBe(false)
  })
})

describe('describeError', () => {
  it('shows server messages and hides unknown errors', () => {
    expect(describeError(new ApiError('validation', 'bad input', 400))).toBe('bad input')
    expect(describeError(new Error('stack trace here'))).toBe(
      'Something went wrong. Please try again.',
    )
  })
})
