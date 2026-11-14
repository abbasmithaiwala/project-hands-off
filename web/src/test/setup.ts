import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { afterEach, expect } from 'vitest'
import * as axeMatchers from 'vitest-axe/matchers'
import { useToastStore } from '@/design/toastStore'
import { useEditorStore } from '@/state/editorStore'

expect.extend(axeMatchers)

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= ResizeObserverStub as unknown as typeof ResizeObserver

afterEach(() => {
  cleanup()
  useEditorStore.getState().reset()
  useEditorStore.setState({ outputOpen: false })
  useToastStore.setState({ toasts: [] })
})
