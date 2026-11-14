import type { Point } from './geometry'

/** The colour of the guide lines; unlike anything in a photo, so the model cannot mistake them. */
export const GUIDE_COLOUR = '#ff00e5'

const MIN_LINE_PX = 3
const LINE_PX_PER_IMAGE_PX = 1 / 320
const DOT_RADIUS_IN_LINES = 1.6

/** The subset of a 2D canvas context the guide needs, so it can be drawn and tested anywhere. */
export type GuideContext = Pick<
  CanvasRenderingContext2D,
  | 'beginPath'
  | 'moveTo'
  | 'lineTo'
  | 'closePath'
  | 'stroke'
  | 'arc'
  | 'fill'
  | 'strokeStyle'
  | 'fillStyle'
  | 'lineWidth'
  | 'lineJoin'
>

/**
 * Draw each closed shape (points in `0..1` image coordinates) as a thin line with a dot on every
 * corner, over a canvas of `width` by `height` pixels.
 */
export function drawGuide(
  context: GuideContext,
  shapes: readonly (readonly Point[])[],
  width: number,
  height: number,
): void {
  const line = Math.max(MIN_LINE_PX, width * LINE_PX_PER_IMAGE_PX)
  context.strokeStyle = GUIDE_COLOUR
  context.fillStyle = GUIDE_COLOUR
  context.lineWidth = line
  context.lineJoin = 'round'
  for (const shape of shapes) {
    const [first, ...rest] = shape
    if (!first) continue
    context.beginPath()
    context.moveTo(first.x * width, first.y * height)
    for (const point of rest) context.lineTo(point.x * width, point.y * height)
    context.closePath()
    context.stroke()
    for (const point of shape) {
      context.beginPath()
      context.arc(point.x * width, point.y * height, line * DOT_RADIUS_IN_LINES, 0, Math.PI * 2)
      context.fill()
    }
  }
}
