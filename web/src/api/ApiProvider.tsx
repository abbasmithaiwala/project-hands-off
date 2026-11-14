import type { ReactNode } from 'react'
import { ApiContext } from './apiContext'
import { createApiClient, type ApiClient } from './client'

const defaultClient = createApiClient()

export function ApiProvider({
  client = defaultClient,
  children,
}: {
  client?: ApiClient
  children: ReactNode
}) {
  return <ApiContext.Provider value={client}>{children}</ApiContext.Provider>
}
