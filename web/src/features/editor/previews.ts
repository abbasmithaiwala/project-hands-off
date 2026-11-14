import type { BBox, DetectedObject, Edit, EditChange, PreviewKind, Widget } from '@/api/client'
import { affectedRegion } from '@/lib/regions'
import {
  compose,
  dragControlPoints,
  objectRegion,
  PREVIEW_MARGIN,
  stretchAlongAxis,
  type Deformation,
} from '@/lib/warp'
import type { Point } from '@/lib/geometry'
import type { Size } from '@/lib/viewport'
import type { PendingEdit } from '@/state/editorStore'
import { isUnchanged } from '@/lib/widgetMath'
import type { PreviewSpec } from './canvas/stage'

const toPixels = (p: Point, size: Size): Point => ({ x: p.x * size.width, y: p.y * size.height })

/** How an edit moves the object's pixels, or `null` if the change does not fit the widget. */
export function deformationFor(
  widget: Widget,
  change: EditChange,
  size: Size,
  bbox: BBox,
): Deformation | null {
  if (isUnchanged(change, widget)) return null
  if (change.kind === 'slider' && widget.kind === 'slider') {
    return stretchAlongAxis(
      toPixels(widget.axis.start, size),
      toPixels(widget.axis.end, size),
      change.value / widget.initial,
    )
  }
  if (change.kind !== 'slider' && widget.kind !== 'slider' && change.kind === widget.kind) {
    const pins = pinPoints(bbox, size)
    return dragControlPoints(
      change.origin.map((p) => toPixels(p, size)),
      change.target.map((p) => toPixels(p, size)),
      pins,
    )
  }
  return null
}

function pinPoints(bbox: BBox, size: Size): Point[] {
  const region = objectRegion(bbox, size, PREVIEW_MARGIN)
  const { x, y, width, height } = region
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

export interface ObjectPreview {
  objectId: number
  spec: PreviewSpec
}

/** One preview per edited object, combining all of its pending edits in order. */
export function buildPreviews(
  edits: readonly PendingEdit[],
  objects: readonly DetectedObject[],
  size: Size,
  maskUrl: (objectId: number) => string,
): ObjectPreview[] {
  const previews: ObjectPreview[] = []
  for (const object of objects) {
    const mine = edits.filter((e) => e.objectId === object.id)
    const deformations = mine
      .map((e) => deformationFor(e.widget, e.edit.change, size, object.bbox))
      .filter((d): d is Deformation => d !== null)
    if (deformations.length === 0) continue
    previews.push({
      objectId: object.id,
      spec: {
        bbox: object.bbox,
        maskUrl: maskUrl(object.id),
        deform: compose(deformations),
        showPixels: mine.every((e) => e.widget.kind === 'slider'),
      },
    })
  }
  return previews
}

/** How far a point may be dragged: a generous fraction of the object's diagonal. */
export function reachFor(object: DetectedObject, size: Size): number {
  const diagonal = Math.hypot(object.bbox.width * size.width, object.bbox.height * size.height)
  return diagonal * 0.6
}

/** The edits to send to the server. Sliders carry their axis so the edit can be put in words. */
export function requestEdits(pending: readonly PendingEdit[]): Edit[] {
  return pending.map(({ edit, widget }) =>
    edit.change.kind === 'slider' && widget.kind === 'slider'
      ? { ...edit, change: { ...edit.change, axis: widget.axis } }
      : edit,
  )
}

/** For each edited object, the area of the photo its edits can change. */
export function affectedRegions(
  edits: readonly PendingEdit[],
  objects: readonly DetectedObject[],
  size: Size,
): BBox[] {
  const regions: BBox[] = []
  for (const object of objects) {
    const deformations = edits
      .filter((e) => e.objectId === object.id)
      .map((e) => deformationFor(e.widget, e.edit.change, size, object.bbox))
      .filter((d): d is Deformation => d !== null)
    if (deformations.length > 0) regions.push(affectedRegion(object, compose(deformations), size))
  }
  return regions
}

/**
 * What to show the image model alongside the photo. Reshaping with control points is best
 * explained by drawing the new outline; everything else (sliders, handles) by a warped preview.
 */
export function referenceKind(edits: readonly PendingEdit[]): PreviewKind {
  const reshapesOnly =
    edits.length > 0 && edits.every((e) => e.edit.change.kind === 'control_points')
  return reshapesOnly ? 'outline' : 'warped'
}

/** The new outline of each reshaped object, in `0..1` image coordinates. */
export function outlineShapes(edits: readonly PendingEdit[]): Point[][] {
  return edits.flatMap((e) =>
    e.edit.change.kind === 'control_points' ? [e.edit.change.target.map((p) => ({ ...p }))] : [],
  )
}
