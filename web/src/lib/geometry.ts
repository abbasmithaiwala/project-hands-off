export interface Point {
  x: number
  y: number
}

export interface Pickable {
  id: number
  bbox: { width: number; height: number }
  outline: readonly Point[]
}

/** Ray-casting test; points on an edge may count as either side. */
export function pointInPolygon(point: Point, polygon: readonly Point[]): boolean {
  let inside = false
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const a = polygon[i]
    const b = polygon[j]
    if (!a || !b) continue
    const crosses = a.y > point.y !== b.y > point.y
    if (crosses && point.x < ((b.x - a.x) * (point.y - a.y)) / (b.y - a.y) + a.x) {
      inside = !inside
    }
  }
  return inside
}

/** The id of the smallest object whose outline contains `point`, so inner objects stay reachable. */
export function pickObject(objects: readonly Pickable[], point: Point): number | null {
  let best: { id: number; area: number } | null = null
  for (const object of objects) {
    if (!pointInPolygon(point, object.outline)) continue
    const area = object.bbox.width * object.bbox.height
    if (!best || area < best.area) best = { id: object.id, area }
  }
  return best ? best.id : null
}

/** Step through `ids` cyclically. Starts at the first (or last) item when nothing is current. */
export function cycle(
  ids: readonly number[],
  current: number | null,
  direction: 1 | -1,
): number | null {
  if (ids.length === 0) return null
  const index = current === null ? -1 : ids.indexOf(current)
  if (index === -1) return (direction === 1 ? ids[0] : ids[ids.length - 1]) ?? null
  return ids[(index + direction + ids.length) % ids.length] ?? null
}

/** The outline point nearest the top (leftmost on ties): a natural place to attach a label. */
export function labelAnchor(outline: readonly Point[]): Point | null {
  let best: Point | null = null
  for (const point of outline) {
    if (!best || point.y < best.y || (point.y === best.y && point.x < best.x)) best = point
  }
  return best
}
