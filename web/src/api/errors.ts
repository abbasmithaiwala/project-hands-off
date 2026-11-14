import type { ApiErrorBody } from './client'

/** A failed API call, carrying the server's stable error code and human message. */
export class ApiError extends Error {
  readonly code: ApiErrorBody['code'] | 'network'
  readonly status: number

  constructor(code: ApiError['code'], message: string, status: number) {
    super(message)
    this.name = 'ApiError'
    this.code = code
    this.status = status
  }
}

interface Result<T> {
  data?: T | undefined
  error?: ApiErrorBody | undefined
  response: Response
}

async function settle<T>(call: Promise<Result<T>>): Promise<Result<T>> {
  let result: Result<T>
  try {
    result = await call
  } catch {
    throw new ApiError('network', 'Could not reach the server. Is it running?', 0)
  }
  if (result.error) {
    throw new ApiError(result.error.code, result.error.message, result.response.status)
  }
  return result
}

/** Succeed for calls that return no body (such as `204`), or throw an [`ApiError`]. */
export async function expectOk(call: Promise<Result<unknown>>): Promise<void> {
  await settle(call)
}

/** Return the data of a client call or throw an [`ApiError`]. */
export async function unwrap<T>(call: Promise<Result<T>>): Promise<T> {
  const result = await settle(call)
  if (result.data === undefined) {
    throw new ApiError('internal', 'The server sent an empty response.', result.response.status)
  }
  return result.data
}

/** True when the failure means the OpenRouter key is missing or rejected. */
export function isKeyProblem(error: unknown): boolean {
  return error instanceof ApiError && error.code === 'unavailable' && /api key/i.test(error.message)
}

/** A short message that is safe to show to a user. */
export function describeError(error: unknown): string {
  if (error instanceof ApiError) return error.message
  return 'Something went wrong. Please try again.'
}
