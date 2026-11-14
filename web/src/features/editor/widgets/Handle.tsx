import { useRef, useState, type KeyboardEvent, type PointerEvent } from 'react'
import { cn } from '@/lib/cn'
import type { Point } from '@/lib/geometry'
import { imageToScreen } from '@/lib/viewport'
import styles from './Handle.module.css'
import type { WidgetContext } from './types'

interface HandleProps {
  /** Position in normalised image coordinates. */
  point: Point
  context: WidgetContext
  label: string
  /** Read out by screen readers after the label, for example "80 percent". */
  valueText?: string
  /** ARIA role: `slider` for single-value controls, otherwise a plain button. */
  role?: 'slider' | 'button'
  ariaValue?: { min: number; max: number; now: number }
  moved: boolean
  onDrag: (clientX: number, clientY: number, modifiers: { shift: boolean }) => void
  /** Arrow-key nudge: direction in `-1..1` per axis, and whether the fine step was asked for. */
  onNudge: (dx: number, dy: number, fine: boolean) => void
  onReset: () => void
}

const ARROWS: Record<string, [number, number]> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
}

/** A draggable, keyboard-operable point drawn over the canvas. */
export function Handle({
  point,
  context,
  label,
  valueText,
  role = 'button',
  ariaValue,
  moved,
  onDrag,
  onNudge,
  onReset,
}: HandleProps) {
  const [dragging, setDragging] = useState(false)
  const element = useRef<HTMLDivElement>(null)
  const screen = imageToScreen(context.view, {
    x: point.x * context.imageSize.width,
    y: point.y * context.imageSize.height,
  })

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    event.preventDefault()
    event.stopPropagation()
    element.current?.focus()
    event.currentTarget.setPointerCapture(event.pointerId)
    setDragging(true)
  }
  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (dragging) onDrag(event.clientX, event.clientY, { shift: event.shiftKey })
  }
  const stopDragging = (event: PointerEvent<HTMLDivElement>) => {
    if (!dragging) return
    setDragging(false)
    event.currentTarget.releasePointerCapture(event.pointerId)
  }

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const arrow = ARROWS[event.key]
    if (arrow) {
      event.preventDefault()
      event.stopPropagation()
      onNudge(arrow[0], arrow[1], event.shiftKey)
    } else if (event.key === 'Home' || event.key === 'Backspace' || event.key === 'Delete') {
      event.preventDefault()
      event.stopPropagation()
      onReset()
    }
  }

  return (
    <div
      ref={element}
      className={cn(styles.handle, dragging && styles.dragging, moved && styles.moved)}
      style={{ left: screen.x, top: screen.y }}
      tabIndex={0}
      role={role}
      aria-label={label}
      aria-roledescription="draggable handle"
      {...(valueText ? { 'aria-valuetext': valueText } : {})}
      {...(role === 'slider' && ariaValue
        ? {
            'aria-valuemin': ariaValue.min,
            'aria-valuemax': ariaValue.max,
            'aria-valuenow': ariaValue.now,
          }
        : {})}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={stopDragging}
      onPointerCancel={stopDragging}
      onKeyDown={onKeyDown}
      data-testid="widget-handle"
    />
  )
}
