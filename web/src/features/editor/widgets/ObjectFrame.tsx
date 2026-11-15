import type { BBox } from '@/api/client'
import type { Point } from '@/lib/geometry'
import { imageToScreen, type Size, type Viewport } from '@/lib/viewport'
import type { Deformation } from '@/lib/warp'
import { frameCorners } from '@/lib/widgetMath'
import styles from './WidgetLayer.module.css'

const toPolygon = (points: readonly Point[]) => points.map((p) => `${p.x},${p.y}`).join(' ')

interface ObjectFrameProps {
  /** The object's box, as a share of the image. */
  box: BBox
  imageSize: Size
  view: Viewport
  /** How the edit moves the object's pixels, so the frame follows. */
  deform: Deformation
  /** Draw where the box started too, so the change reads at a glance. */
  edited: boolean
}

/** A dashed frame round the object that turns, grows or travels along with its control. */
export function ObjectFrame({ box, imageSize, view, deform, edited }: ObjectFrameProps) {
  const onScreen = (d: Deformation) =>
    frameCorners(box, imageSize, d).map((corner) => imageToScreen(view, corner))
  return (
    <>
      {edited && <polygon className={styles.ghost} points={toPolygon(onScreen((p) => p))} />}
      <polygon className={styles.frame} points={toPolygon(onScreen(deform))} />
    </>
  )
}
