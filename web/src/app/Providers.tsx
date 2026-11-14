import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useState, type ReactNode } from 'react'
import { ApiProvider } from '@/api/ApiProvider'
import type { ApiClient } from '@/api/client'
import { Toaster } from '@/design/Toaster'
import { ErrorBoundary } from './ErrorBoundary'

interface ProvidersProps {
  client?: ApiClient
  /** Replaceable so tests can disable retries. */
  queryClient?: QueryClient
  children: ReactNode
}

function createQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { refetchOnWindowFocus: false }, mutations: { retry: false } },
  })
}

export function Providers({ client, queryClient: provided, children }: ProvidersProps) {
  const [created] = useState(createQueryClient)
  const queryClient = provided ?? created
  return (
    <ErrorBoundary>
      <ApiProvider {...(client ? { client } : {})}>
        <QueryClientProvider client={queryClient}>
          {children}
          <Toaster />
        </QueryClientProvider>
      </ApiProvider>
    </ErrorBoundary>
  )
}
