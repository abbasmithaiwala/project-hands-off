import { Component, type ErrorInfo, type ReactNode } from 'react'
import { Button } from '@/design/Button'
import { Panel } from '@/design/Panel'

interface State {
  failed: boolean
}

/** Catches rendering errors so a bug never leaves a blank screen. */
export class ErrorBoundary extends Component<{ children: ReactNode }, State> {
  override state: State = { failed: false }

  static getDerivedStateFromError(): State {
    return { failed: true }
  }

  override componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('Unhandled UI error', error, info.componentStack)
  }

  override render() {
    if (!this.state.failed) return this.props.children
    return (
      <main style={{ display: 'grid', placeItems: 'center', height: '100%', padding: 24 }}>
        <Panel style={{ padding: 32, maxWidth: 420, textAlign: 'center' }} role="alert">
          <h1 style={{ fontSize: 'var(--text-lg)', marginBottom: 8 }}>Something went wrong</h1>
          <p style={{ color: 'var(--color-text-muted)', marginBottom: 20 }}>
            The app hit an unexpected problem. Your images are safe. Reloading usually fixes it.
          </p>
          <Button variant="primary" onClick={() => window.location.reload()}>
            Reload
          </Button>
        </Panel>
      </main>
    )
  }
}
