import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from 'react'
import type { DetectedObject } from '@/api/client'
import { labelAnchor } from '@/lib/geometry'
import { imageToScreen } from '@/lib/viewport'
import type { Size, Viewport } from '@/lib/viewport'
import type { ObjectPreview } from '../previews'
import styles from './CanvasStage.module.css'
import { ObjectLabel } from './ObjectLabel'
import { BUTTON_ZOOM_FACTOR, createPixiStage, type CreateStage, type StageHandle } from './stage'

interface CanvasStageProps {
  imageUrl: string
  objects: readonly DetectedObject[]
  hoveredId: number | null
  selectedId: number | null
  /** Warped previews to show, one per edited object. */
  previews: readonly ObjectPreview[]
  /** Drawn over the canvas, given the current view; used for the editing controls. */
  overlay?: (context: { view: Viewport; imageSize: Size }) => ReactNode
  onHover: (id: number | null) => void
  onSelect: (id: number | null) => void
  onCycle: (direction: 1 | -1) => void
  /** Replaceable so tests can run without WebGL. */
  createStage?: CreateStage
}

const INSTRUCTIONS =
  'Image canvas. Arrow keys move between objects, Escape clears the selection, plus and minus zoom, zero fits the image.'

function useLatest<T>(value: T) {
  const ref = useRef(value)
  useEffect(() => {
    ref.current = value
  })
  return ref
}

export function CanvasStage({
  imageUrl,
  objects,
  hoveredId,
  selectedId,
  previews,
  overlay,
  onHover,
  onSelect,
  onCycle,
  createStage = createPixiStage,
}: CanvasStageProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const [stage, setStage] = useState<StageHandle | null>(null)
  const [view, setView] = useState<Viewport | null>(null)
  const [imageSize, setImageSize] = useState<Size | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const handlers = useLatest({ onHover, onSelect })

  useEffect(() => {
    const container = containerRef.current
    if (!container) return
    let cancelled = false
    let created: StageHandle | undefined
    createStage(container, {
      onHover: (id) => handlers.current.onHover(id),
      onSelect: (id) => handlers.current.onSelect(id),
      onViewChange: setView,
    })
      .then((handle) => {
        if (cancelled) handle.destroy()
        else {
          created = handle
          setStage(handle)
        }
      })
      .catch(() => setProblem('Your browser could not start the image canvas (WebGL is required).'))
    return () => {
      cancelled = true
      created?.destroy()
      setStage(null)
    }
  }, [createStage, handlers])

  useEffect(() => {
    if (!stage) return
    let current = true
    stage
      .setImage(imageUrl)
      .then((size) => current && setImageSize(size))
      .catch(() => current && setProblem('The image could not be displayed.'))
    return () => {
      current = false
    }
  }, [stage, imageUrl])

  useEffect(() => stage?.setObjects(objects), [stage, objects])
  useEffect(() => stage?.setHighlight(hoveredId, selectedId), [stage, hoveredId, selectedId])
  const shownPreviews = useRef(new Set<number>())
  useEffect(() => {
    if (!stage) return
    const ids = new Set(previews.map((p) => p.objectId))
    for (const id of shownPreviews.current) if (!ids.has(id)) stage.setPreview(id, null)
    for (const preview of previews) stage.setPreview(preview.objectId, preview.spec)
    shownPreviews.current = ids
  }, [stage, previews])

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const actions: Record<string, () => void> = {
      ArrowRight: () => onCycle(1),
      ArrowDown: () => onCycle(1),
      ArrowLeft: () => onCycle(-1),
      ArrowUp: () => onCycle(-1),
      Escape: () => onSelect(null),
      '+': () => stage?.zoomBy(BUTTON_ZOOM_FACTOR),
      '=': () => stage?.zoomBy(BUTTON_ZOOM_FACTOR),
      '-': () => stage?.zoomBy(1 / BUTTON_ZOOM_FACTOR),
      '0': () => stage?.fit(),
    }
    const action = actions[event.key]
    if (action) {
      event.preventDefault()
      action()
    }
  }

  const labelled = objects.filter((o) => o.id === selectedId || o.id === hoveredId)

  return (
    <div
      ref={containerRef}
      className={styles.stage}
      role="application"
      aria-label={INSTRUCTIONS}
      tabIndex={0}
      onKeyDown={onKeyDown}
      data-testid="canvas-stage"
    >
      {view &&
        imageSize &&
        labelled.map((object) => {
          const anchor = labelAnchor(object.outline) ?? object.bbox
          const corner = imageToScreen(view, {
            x: anchor.x * imageSize.width,
            y: anchor.y * imageSize.height,
          })
          return (
            <ObjectLabel
              key={object.id}
              text={object.label}
              x={corner.x}
              y={corner.y}
              selected={object.id === selectedId}
            />
          )
        })}
      {view && imageSize && overlay?.({ view, imageSize })}
      {previews.length > 0 && (
        <p className={styles.approximate} role="note">
          Preview is approximate. The final image is generated when you apply.
        </p>
      )}
      {problem && (
        <div className={styles.message} role="alert">
          {problem}
        </div>
      )}
    </div>
  )
}
