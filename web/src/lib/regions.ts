import type { BBox, DetectedObject } from '@/api/client'
import type { Point } from './geometry'
import type { Size } from './viewport'
import type { Deformation } from './warp'

/** Room added around an edited area, as a fraction of its size, so soft edges are not cut off. */
const MARGIN = 0.03

const clamp01 = (value: number) => Math.min(Math.max(value, 0), 1)

/** The smallest box containing every point (normalised coordinates), or `null` for none. */
export function boundingBox(points: readonly Point[]): BBox | null {
  if (points.length === 0) return null
  const xs = points.map((p) => p.x)
  const ys = points.map((p) => p.y)
  const x = Math.min(...xs)
  const y = Math.min(...ys)
  return { x, y, width: Math.max(...xs) - x, height: Math.max(...ys) - y }
}

/** The smallest box containing both. */
export function unionBox(a: BBox, b: BBox): BBox {
  const x = Math.min(a.x, b.x)
  const y = Math.min(a.y, b.y)
  return {
    x,
    y,
    width: Math.max(a.x + a.width, b.x + b.width) - x,
    height: Math.max(a.y + a.height, b.y + b.height) - y,
  }
}

/** Grow a box by `MARGIN` on every side and keep it inside the image. */
function pad(box: BBox): BBox {
  const x0 = clamp01(box.x - box.width * MARGIN)
  const y0 = clamp01(box.y - box.height * MARGIN)
  const x1 = clamp01(box.x + box.width * (1 + MARGIN))
  const y1 = clamp01(box.y + box.height * (1 + MARGIN))
  return { x: x0, y: y0, width: Math.max(x1 - x0, 1e-6), height: Math.max(y1 - y0, 1e-6) }
}

/**
 * Where an edit can change pixels: the object's box together with the box of its outline after
 * the deformation, so both the place it left and the place it moved to are covered.
 */
export function affectedRegion(object: DetectedObject, deform: Deformation, size: Size): BBox {
  const moved = object.outline.map((p) => {
    const pixel = deform({ x: p.x * size.width, y: p.y * size.height })
    return { x: clamp01(pixel.x / size.width), y: clamp01(pixel.y / size.height) }
  })
  const after = boundingBox(moved)
  return pad(after ? unionBox(object.bbox, after) : object.bbox)
}
