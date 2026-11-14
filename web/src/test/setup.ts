import '@testing-library/jest-dom/vitest'
import { cleanup, configure } from '@testing-library/react'
import { afterEach, expect } from 'vitest'
import * as axeMatchers from 'vitest-axe/matchers'
import { useToastStore } from '@/design/toastStore'
import { useEditorStore } from '@/state/editorStore'

expect.extend(axeMatchers)

// Lazily loaded screens can take a moment to appear when the whole suite runs in parallel.
configure({ asyncUtilTimeout: 5000 })

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= ResizeObserverStub as unknown as typeof ResizeObserver

// jsdom does not implement pointer capture, which draggable handles rely on.
Element.prototype.setPointerCapture ??= () => undefined
Element.prototype.releasePointerCapture ??= () => undefined

afterEach(() => {
  cleanup()
  useEditorStore.getState().reset()
  useEditorStore.setState({ outputOpen: false })
  useToastStore.setState({ toasts: [] })
})
