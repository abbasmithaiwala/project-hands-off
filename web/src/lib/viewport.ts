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

/** Centre the image in the container, as large as fits inside `padding`. */
export function fitViewport(image: Size, container: Size, padding = 0): Viewport {
  const availableWidth = Math.max(container.width - 2 * padding, 1)
  const availableHeight = Math.max(container.height - 2 * padding, 1)
  const scale = Math.min(availableWidth / image.width, availableHeight / image.height)
  return {
    scale,
    x: (container.width - image.width * scale) / 2,
    y: (container.height - image.height * scale) / 2,
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
