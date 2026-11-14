import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { App } from '@/app/App'
import { Providers } from '@/app/Providers'
import { applyStoredTheme } from '@/app/theme'
import '@/design/tokens.css'
import '@/design/global.css'

applyStoredTheme()

const container = document.getElementById('root')
if (!container) {
  throw new Error('Root element #root not found')
}

createRoot(container).render(
  <StrictMode>
    <Providers>
      <App />
    </Providers>
  </StrictMode>,
)
