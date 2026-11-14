import type { EditChange, Widget } from '@/api/client'
import type { Point } from './geometry'
import { clamp } from './numbers'
import type { Size } from './viewport'

type SliderWidget = Extract<Widget, { kind: 'slider' }>

const toPixels = (p: Point, size: Size): Point => ({ x: p.x * size.width, y: p.y * size.height })
const toNormalised = (p: Point, size: Size): Point => ({
  x: p.x / size.width,
  y: p.y / size.height,
})

/** Where a slider's handle sits for `value`: at the axis end for the initial value, scaling from the start. */
export function sliderHandlePoint(widget: SliderWidget, value: number, size: Size): Point {
  const start = toPixels(widget.axis.start, size)
  const end = toPixels(widget.axis.end, size)
  const factor = value / widget.initial
  return toNormalised(
    { x: start.x + (end.x - start.x) * factor, y: start.y + (end.y - start.y) * factor },
    size,
  )
}

/** The slider value for a handle dragged to `point`, clamped to the allowed range. */
export function sliderValueAt(widget: SliderWidget, point: Point, size: Size): number {
  const start = toPixels(widget.axis.start, size)
  const end = toPixels(widget.axis.end, size)
  const p = toPixels(point, size)
  const dx = end.x - start.x
  const dy = end.y - start.y
  const lengthSquared = dx * dx + dy * dy
  if (lengthSquared === 0) return widget.initial
  const along = ((p.x - start.x) * dx + (p.y - start.y) * dy) / lengthSquared
  return clamp(along * widget.initial, widget.min, widget.max)
}

export interface ConstraintOptions {
  /** Keep the point within this many image pixels of where it started. */
  maxDistance: number
  /** Snap back to the start when closer than this many image pixels. */
  snapRadius: number
  /** Restrict movement to the dominant axis (horizontal or vertical) from the start. */
  axisLock: boolean
}

/** Keep a dragged point inside the image and within a sensible range of its origin. */
export function constrainPoint(
  point: Point,
  origin: Point,
  size: Size,
  options: ConstraintOptions,
): Point {
  let p = { x: clamp(point.x, 0, 1), y: clamp(point.y, 0, 1) }

  if (options.axisLock) {
    const dx = Math.abs(p.x - origin.x) * size.width
    const dy = Math.abs(p.y - origin.y) * size.height
    p = dx >= dy ? { x: p.x, y: origin.y } : { x: origin.x, y: p.y }
  }

  const px = (p.x - origin.x) * size.width
  const py = (p.y - origin.y) * size.height
  const distance = Math.hypot(px, py)
  if (distance <= options.snapRadius) return { ...origin }
  if (distance > options.maxDistance) {
    const factor = options.maxDistance / distance
    p = { x: origin.x + (px * factor) / size.width, y: origin.y + (py * factor) / size.height }
  }
  return { x: clamp(p.x, 0, 1), y: clamp(p.y, 0, 1) }
}

const EPSILON = 1e-4

/** True when a change leaves everything where it started. */
export function isUnchanged(change: EditChange, widget: Widget): boolean {
  switch (change.kind) {
    case 'slider':
      return widget.kind === 'slider' && Math.abs(change.value - widget.initial) < EPSILON
    case 'control_points':
    case 'multi_handle':
      return change.origin.every((p, i) => {
        const q = change.target[i]
        return q !== undefined && Math.abs(p.x - q.x) < EPSILON && Math.abs(p.y - q.y) < EPSILON
      })
  }
}

/** A short human description of a change, for the pending-edits list. */
export function describeChange(change: EditChange): string {
  switch (change.kind) {
    case 'slider':
      return `${Math.round(change.value * 100)}%`
    case 'control_points':
    case 'multi_handle': {
      const moved = change.origin.filter((p, i) => {
        const q = change.target[i]
        return q !== undefined && (Math.abs(p.x - q.x) >= EPSILON || Math.abs(p.y - q.y) >= EPSILON)
      }).length
      return moved === 1 ? '1 point moved' : `${moved} points moved`
    }
  }
}

/** The starting positions of a widget's draggable points. */
export function initialPoints(widget: Exclude<Widget, { kind: 'slider' }>): Point[] {
  return widget.kind === 'control_points' ? [...widget.points] : [...widget.handles]
}
