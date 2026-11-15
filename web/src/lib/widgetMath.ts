import type { BBox, EditChange, Widget } from '@/api/client'
import type { Point } from './geometry'
import { clamp } from './numbers'
import type { Size } from './viewport'
import type { Deformation } from './warp'

/** Widgets whose handle runs along a line from a fixed start, and whose value is a multiplier. */
type AxisWidget = Extract<Widget, { kind: 'slider' | 'scale' }>
type RotateWidget = Extract<Widget, { kind: 'rotate' }>

const toPixels = (p: Point, size: Size): Point => ({ x: p.x * size.width, y: p.y * size.height })
const toNormalised = (p: Point, size: Size): Point => ({
  x: p.x / size.width,
  y: p.y / size.height,
})

/** Where an axis handle sits for `value`: at the axis end for the initial value, scaling from the start. */
export function sliderHandlePoint(widget: AxisWidget, value: number, size: Size): Point {
  const start = toPixels(widget.axis.start, size)
  const end = toPixels(widget.axis.end, size)
  const factor = value / widget.initial
  return toNormalised(
    { x: start.x + (end.x - start.x) * factor, y: start.y + (end.y - start.y) * factor },
    size,
  )
}

/** The multiplier for a handle dragged to `point`, clamped to the allowed range. */
export function sliderValueAt(widget: AxisWidget, point: Point, size: Size): number {
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

const RADIANS_PER_DEGREE = Math.PI / 180

/** An angle in degrees brought into `(-180, 180]`. */
export function wrapDegrees(degrees: number): number {
  const wrapped = ((((degrees + 180) % 360) + 360) % 360) - 180
  return wrapped === -180 ? 180 : wrapped
}

/** Where a rotate handle sits after turning `degrees` clockwise: on its ring around the pivot. */
export function rotateHandlePoint(widget: RotateWidget, degrees: number, size: Size): Point {
  const pivot = toPixels(widget.pivot, size)
  const handle = toPixels(widget.handle, size)
  const radians = degrees * RADIANS_PER_DEGREE
  const dx = handle.x - pivot.x
  const dy = handle.y - pivot.y
  return toNormalised(
    {
      x: pivot.x + dx * Math.cos(radians) - dy * Math.sin(radians),
      y: pivot.y + dx * Math.sin(radians) + dy * Math.cos(radians),
    },
    size,
  )
}

/** The turn, in degrees clockwise, for a handle dragged to `point`, clamped to the allowed range. */
export function rotateDegreesAt(widget: RotateWidget, point: Point, size: Size): number {
  const pivot = toPixels(widget.pivot, size)
  const handle = toPixels(widget.handle, size)
  const target = toPixels(point, size)
  if (target.x === pivot.x && target.y === pivot.y) return 0
  const from = Math.atan2(handle.y - pivot.y, handle.x - pivot.x)
  const to = Math.atan2(target.y - pivot.y, target.x - pivot.x)
  return clamp(wrapDegrees((to - from) / RADIANS_PER_DEGREE), widget.min, widget.max)
}

/** The corners of `box` in image pixels after `deform`, for drawing the frame that follows an edit. */
export function frameCorners(box: BBox, size: Size, deform: Deformation): Point[] {
  const corners = [
    { x: box.x, y: box.y },
    { x: box.x + box.width, y: box.y },
    { x: box.x + box.width, y: box.y + box.height },
    { x: box.x, y: box.y + box.height },
  ]
  return corners.map((corner) => deform(toPixels(corner, size)))
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
/** A turn smaller than this is a slip of the hand, not an edit. */
const MIN_DEGREES = 0.05

/** True when a change leaves everything where it started. */
export function isUnchanged(change: EditChange, widget: Widget): boolean {
  switch (change.kind) {
    case 'slider':
      return widget.kind === 'slider' && Math.abs(change.value - widget.initial) < EPSILON
    case 'scale':
      return widget.kind === 'scale' && Math.abs(change.value - widget.initial) < EPSILON
    case 'rotate':
      return Math.abs(change.degrees) < MIN_DEGREES
    case 'move':
      return Math.abs(change.dx) < EPSILON && Math.abs(change.dy) < EPSILON
    case 'control_points':
    case 'multi_handle':
      return change.origin.every((p, i) => {
        const q = change.target[i]
        return q !== undefined && Math.abs(p.x - q.x) < EPSILON && Math.abs(p.y - q.y) < EPSILON
      })
  }
}

const percent = (value: number) => Math.round(value * 100)

/** "12% right, 5% down": the shares of the image a move covers, leaving out a zero direction. */
export function describeMove(dx: number, dy: number): string {
  const parts = [
    percent(Math.abs(dx)) > 0 ? `${percent(Math.abs(dx))}% ${dx < 0 ? 'left' : 'right'}` : null,
    percent(Math.abs(dy)) > 0 ? `${percent(Math.abs(dy))}% ${dy < 0 ? 'up' : 'down'}` : null,
  ].filter((part): part is string => part !== null)
  return parts.length > 0 ? parts.join(', ') : '0%'
}

/** A short human description of a change, for the pending-edits list. */
export function describeChange(change: EditChange): string {
  switch (change.kind) {
    case 'slider':
    case 'scale':
      return `${percent(change.value)}%`
    case 'rotate': {
      const degrees = Math.round(change.degrees)
      return `${degrees > 0 ? '+' : ''}${degrees}°`
    }
    case 'move':
      return describeMove(change.dx, change.dy)
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
export function initialPoints(
  widget: Extract<Widget, { kind: 'control_points' | 'multi_handle' }>,
): Point[] {
  return widget.kind === 'control_points' ? [...widget.points] : [...widget.handles]
}
