import type { Point } from './geometry'

/** Moves a point. All warp maths works in image pixels so that aspect ratio never skews it. */
export type Deformation = (point: Point) => Point

export const identity: Deformation = (point) => point

/** Apply several deformations in order. */
export function compose(deformations: readonly Deformation[]): Deformation {
  if (deformations.length === 0) return identity
  return (point) => deformations.reduce((moved, deform) => deform(moved), point)
}

/**
 * Scale everything along the line from `start` to `end` by `scale`, keeping `start` fixed.
 * Distances across the line are unchanged, so an object stretches without skewing.
 */
export function stretchAlongAxis(start: Point, end: Point, scale: number): Deformation {
  const dx = end.x - start.x
  const dy = end.y - start.y
  const length = Math.hypot(dx, dy)
  if (length === 0 || scale === 1) return identity
  const ux = dx / length
  const uy = dy / length
  return (point) => {
    const rx = point.x - start.x
    const ry = point.y - start.y
    const along = rx * ux + ry * uy
    const across = -rx * uy + ry * ux
    const stretched = along * scale
    return {
      x: start.x + stretched * ux - across * uy,
      y: start.y + stretched * uy + across * ux,
    }
  }
}

/** Turn everything about `pivot` by `radians`, clockwise on screen (y grows downward). */
export function rotateAbout(pivot: Point, radians: number): Deformation {
  if (radians === 0) return identity
  const cos = Math.cos(radians)
  const sin = Math.sin(radians)
  return (point) => {
    const rx = point.x - pivot.x
    const ry = point.y - pivot.y
    return { x: pivot.x + rx * cos - ry * sin, y: pivot.y + rx * sin + ry * cos }
  }
}

/** Resize everything evenly about `pivot`, which stays where it is. */
export function scaleAbout(pivot: Point, factor: number): Deformation {
  if (factor === 1) return identity
  return (point) => ({
    x: pivot.x + (point.x - pivot.x) * factor,
    y: pivot.y + (point.y - pivot.y) * factor,
  })
}

/** Carry everything by the same offset. */
export function translate(dx: number, dy: number): Deformation {
  if (dx === 0 && dy === 0) return identity
  return (point) => ({ x: point.x + dx, y: point.y + dy })
}

const NEAR = 1e-6
const SINGULAR = 1e-9

/**
 * Affine Moving Least Squares (Schaefer, McPhail and Warren, 2006): each point moves with
 * the best-fit affine transform of the control points, weighted by inverse distance, so the
 * result is smooth and passes exactly through every control point.
 *
 * `origin[i]` is where control point `i` starts and `target[i]` where it ends up.
 */
export function movingLeastSquares(
  origin: readonly Point[],
  target: readonly Point[],
  alpha = 1,
): Deformation {
  const count = Math.min(origin.length, target.length)
  if (count === 0) return identity

  return (v) => {
    const weights: number[] = []
    let total = 0
    let pStarX = 0
    let pStarY = 0
    let qStarX = 0
    let qStarY = 0
    for (let i = 0; i < count; i++) {
      const p = origin[i] as Point
      const q = target[i] as Point
      const distanceSquared = (p.x - v.x) ** 2 + (p.y - v.y) ** 2
      if (distanceSquared < NEAR) return { x: q.x, y: q.y }
      const w = 1 / distanceSquared ** alpha
      weights.push(w)
      total += w
      pStarX += w * p.x
      pStarY += w * p.y
      qStarX += w * q.x
      qStarY += w * q.y
    }
    pStarX /= total
    pStarY /= total
    qStarX /= total
    qStarY /= total

    // M = (sum w p^T p)^-1 (sum w p^T q), with p and q relative to their weighted centroids.
    let a = 0
    let b = 0
    let d = 0
    let m00 = 0
    let m01 = 0
    let m10 = 0
    let m11 = 0
    for (let i = 0; i < count; i++) {
      const p = origin[i] as Point
      const q = target[i] as Point
      const w = weights[i] as number
      const px = p.x - pStarX
      const py = p.y - pStarY
      const qx = q.x - qStarX
      const qy = q.y - qStarY
      a += w * px * px
      b += w * px * py
      d += w * py * py
      m00 += w * px * qx
      m01 += w * px * qy
      m10 += w * py * qx
      m11 += w * py * qy
    }
    const determinant = a * d - b * b
    const rx = v.x - pStarX
    const ry = v.y - pStarY
    if (Math.abs(determinant) < SINGULAR * Math.max(a * a, d * d, 1)) {
      return { x: rx + qStarX, y: ry + qStarY }
    }
    // Inverse of the symmetric matrix [[a, b], [b, d]] applied to the right-hand side.
    const i00 = d / determinant
    const i01 = -b / determinant
    const i11 = a / determinant
    const t00 = i00 * m00 + i01 * m10
    const t01 = i00 * m01 + i01 * m11
    const t10 = i01 * m00 + i11 * m10
    const t11 = i01 * m01 + i11 * m11
    return {
      x: rx * t00 + ry * t10 + qStarX,
      y: rx * t01 + ry * t11 + qStarY,
    }
  }
}

/** How far past the object's box a preview reaches, as a fraction of the box size. */
export const PREVIEW_MARGIN = 0.04

export interface Rect {
  x: number
  y: number
  width: number
  height: number
}

/**
 * The part of the image an edit can touch: the object's box grown by `margin` (a fraction of
 * its size), in whole pixels and clamped to the image. `bbox` is in normalised coordinates.
 */
export function objectRegion(
  bbox: Rect,
  size: { width: number; height: number },
  margin: number,
): Rect {
  const clamp = (value: number, max: number) => Math.min(Math.max(value, 0), max)
  const x0 = clamp(Math.floor((bbox.x - bbox.width * margin) * size.width), size.width - 1)
  const y0 = clamp(Math.floor((bbox.y - bbox.height * margin) * size.height), size.height - 1)
  const x1 = clamp(Math.ceil((bbox.x + bbox.width * (1 + margin)) * size.width), size.width)
  const y1 = clamp(Math.ceil((bbox.y + bbox.height * (1 + margin)) * size.height), size.height)
  return { x: x0, y: y0, width: Math.max(1, x1 - x0), height: Math.max(1, y1 - y0) }
}

/** Eight fixed points around the edge of `rect`, to keep a warp local to the object. */
export function pinsAround(rect: Rect): Point[] {
  const { x, y, width, height } = rect
  const midX = x + width / 2
  const midY = y + height / 2
  return [
    { x, y },
    { x: midX, y },
    { x: x + width, y },
    { x: x + width, y: midY },
    { x: x + width, y: y + height },
    { x: midX, y: y + height },
    { x, y: y + height },
    { x, y: midY },
  ]
}

/**
 * A warp that drags control points from `origin` to `target` while pinning `pins` in place.
 * Returns the identity when nothing has moved.
 */
export function dragControlPoints(
  origin: readonly Point[],
  target: readonly Point[],
  pins: readonly Point[],
): Deformation {
  const moved = origin.some((p, i) => {
    const q = target[i]
    return q !== undefined && (Math.abs(p.x - q.x) > NEAR || Math.abs(p.y - q.y) > NEAR)
  })
  if (!moved) return identity
  return movingLeastSquares([...origin, ...pins], [...target, ...pins])
}

export interface GridMesh {
  /** Vertex positions in image pixels, `[x0, y0, x1, y1, ...]`. */
  positions: Float32Array
  /** Texture coordinates in `0..1`, same layout. */
  uvs: Float32Array
  indices: Uint32Array
}

/** A regular grid of `columns` x `rows` quads covering `rect`. */
export function gridMesh(rect: Rect, columns: number, rows: number): GridMesh {
  const cols = Math.max(1, Math.floor(columns))
  const rws = Math.max(1, Math.floor(rows))
  const stride = cols + 1
  const positions = new Float32Array(stride * (rws + 1) * 2)
  const uvs = new Float32Array(positions.length)
  for (let row = 0; row <= rws; row++) {
    for (let col = 0; col <= cols; col++) {
      const index = (row * stride + col) * 2
      const u = col / cols
      const v = row / rws
      positions[index] = rect.x + u * rect.width
      positions[index + 1] = rect.y + v * rect.height
      uvs[index] = u
      uvs[index + 1] = v
    }
  }
  const indices = new Uint32Array(cols * rws * 6)
  let cursor = 0
  for (let row = 0; row < rws; row++) {
    for (let col = 0; col < cols; col++) {
      const topLeft = row * stride + col
      const bottomLeft = topLeft + stride
      indices.set(
        [topLeft, topLeft + 1, bottomLeft, topLeft + 1, bottomLeft + 1, bottomLeft],
        cursor,
      )
      cursor += 6
    }
  }
  return { positions, uvs, indices }
}

/** Apply `deform` to every vertex of `positions` (pixel coordinates), writing into `out`. */
export function deformPositions(
  positions: Float32Array,
  deform: Deformation,
  out: Float32Array = new Float32Array(positions.length),
): Float32Array {
  for (let i = 0; i < positions.length; i += 2) {
    const moved = deform({ x: positions[i] as number, y: positions[i + 1] as number })
    out[i] = moved.x
    out[i + 1] = moved.y
  }
  return out
}
