import { useCallback, useRef } from 'react'
import type { BBox, EditChange, Widget } from '@/api/client'
import { screenToImage, type Size, type Viewport } from '@/lib/viewport'
import styles from './WidgetLayer.module.css'
import { widgetRegistry } from './registry'
import type { WidgetContext } from './types'

interface WidgetLayerProps {
  widget: Widget
  change: EditChange | null
  propertyLabel: string
  objectLabel: string
  view: Viewport
  imageSize: Size
  /** The edited object's box, so controls can draw the frame that follows the edit. */
  objectBox: BBox
  /** How far (in image pixels) a point may travel from where it started. */
  reach: number
  onChange: (change: EditChange | null) => void
}

/** Draws the planned control for the active property over the canvas and reports edits. */
export function WidgetLayer({
  widget,
  change,
  propertyLabel,
  objectLabel,
  view,
  imageSize,
  objectBox,
  reach,
  onChange,
}: WidgetLayerProps) {
  const layer = useRef<HTMLDivElement>(null)

  const toImagePoint = useCallback(
    (clientX: number, clientY: number) => {
      const rect = layer.current?.getBoundingClientRect()
      const pixel = screenToImage(view, {
        x: clientX - (rect?.left ?? 0),
        y: clientY - (rect?.top ?? 0),
      })
      return { x: pixel.x / imageSize.width, y: pixel.y / imageSize.height }
    },
    [view, imageSize],
  )

  const context: WidgetContext = { view, imageSize, objectBox, reach, toImagePoint }
  const shared = { propertyLabel, objectLabel, context, onChange }

  return (
    <div
      ref={layer}
      className={styles.layer}
      role="group"
      aria-label={`${propertyLabel} controls for ${objectLabel}`}
    >
      {widget.kind === 'slider' && (
        <widgetRegistry.slider
          widget={widget}
          change={change?.kind === 'slider' ? change : null}
          {...shared}
        />
      )}
      {widget.kind === 'control_points' && (
        <widgetRegistry.control_points
          widget={widget}
          change={change?.kind === 'control_points' ? change : null}
          {...shared}
        />
      )}
      {widget.kind === 'multi_handle' && (
        <widgetRegistry.multi_handle
          widget={widget}
          change={change?.kind === 'multi_handle' ? change : null}
          {...shared}
        />
      )}
      {widget.kind === 'rotate' && (
        <widgetRegistry.rotate
          widget={widget}
          change={change?.kind === 'rotate' ? change : null}
          {...shared}
        />
      )}
      {widget.kind === 'scale' && (
        <widgetRegistry.scale
          widget={widget}
          change={change?.kind === 'scale' ? change : null}
          {...shared}
        />
      )}
      {widget.kind === 'move' && (
        <widgetRegistry.move
          widget={widget}
          change={change?.kind === 'move' ? change : null}
          {...shared}
        />
      )}
    </div>
  )
}
