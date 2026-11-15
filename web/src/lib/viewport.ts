import type { Point } from './geometry'

/** Maps image pixels to screen pixels: `screen = image * scale + (x, y)`. */
export interface Viewport {
  scale: number
  x: number
  y: number
}

export interface Size {
  width: number
  height: number
}

export const MIN_SCALE_FACTOR = 0.25
export const MAX_SCALE = 16

export interface Padding {
  top: number
  right: number
  bottom: number
  left: number
}

/**
 * Centre the image in the part of the container left after `padding` (a number for all sides,
 * or one value per side), as large as fits. Per-side padding keeps floating panels off the image.
 */
export function fitViewport(image: Size, container: Size, padding: number | Padding = 0): Viewport {
  const { top, right, bottom, left } =
    typeof padding === 'number'
      ? { top: padding, right: padding, bottom: padding, left: padding }
      : padding
  const availableWidth = Math.max(container.width - left - right, 1)
  const availableHeight = Math.max(container.height - top - bottom, 1)
  const scale = Math.min(availableWidth / image.width, availableHeight / image.height)
  return {
    scale,
    x: left + (availableWidth - image.width * scale) / 2,
    y: top + (availableHeight - image.height * scale) / 2,
  }
}

/** Zoom by `factor` keeping the image point under `anchor` (a screen point) fixed. */
export function zoomAt(view: Viewport, factor: number, anchor: Point, fitScale: number): Viewport {
  const scale = Math.min(Math.max(view.scale * factor, fitScale * MIN_SCALE_FACTOR), MAX_SCALE)
  const ratio = scale / view.scale
  return {
    scale,
    x: anchor.x - (anchor.x - view.x) * ratio,
    y: anchor.y - (anchor.y - view.y) * ratio,
  }
}

export function panBy(view: Viewport, dx: number, dy: number): Viewport {
  return { ...view, x: view.x + dx, y: view.y + dy }
}

export function screenToImage(view: Viewport, point: Point): Point {
  return { x: (point.x - view.x) / view.scale, y: (point.y - view.y) / view.scale }
}

export function imageToScreen(view: Viewport, point: Point): Point {
  return { x: point.x * view.scale + view.x, y: point.y * view.scale + view.y }
}

/** Where a point given as a share of the image (`0..1`) appears on screen. */
export function normalisedToScreen(view: Viewport, size: Size, point: Point): Point {
  return imageToScreen(view, { x: point.x * size.width, y: point.y * size.height })
}
