import type { CreateStage } from '@/features/editor/canvas/stage'
import { LandingScreen } from '@/features/landing/LandingScreen'
import { lazy, Suspense } from 'react'
import { useEditorStore } from '@/state/editorStore'

// The editor pulls in the WebGL renderer, so it is loaded only once an image is opened.
const EditorScreen = lazy(() =>
  import('@/features/editor/EditorScreen').then((module) => ({ default: module.EditorScreen })),
)

interface AppProps {
  createStage?: CreateStage
}

export function App({ createStage }: AppProps) {
  const image = useEditorStore((state) => state.image)
  if (!image) return <LandingScreen />
  return (
    <Suspense fallback={<EditorFallback />}>
      <EditorScreen key={image.id} image={image} {...(createStage ? { createStage } : {})} />
    </Suspense>
  )
}

function EditorFallback() {
  return (
    <div
      role="status"
      aria-label="Loading the editor"
      style={{ height: '100%', background: 'var(--color-canvas)' }}
    />
  )
}
