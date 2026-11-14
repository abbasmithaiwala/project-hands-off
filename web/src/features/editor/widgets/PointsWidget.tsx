import type { EditChange, NormalizedPoint } from '@/api/client'
import type { Point } from '@/lib/geometry'
import { imageToScreen } from '@/lib/viewport'
import { constrainPoint } from '@/lib/widgetMath'
import { Handle } from './Handle'
import type { WidgetContext } from './types'
import styles from './WidgetLayer.module.css'

const SNAP_RADIUS_SCREEN_PX = 6
const STEP = 0.01
const FINE_STEP = 0.001
const MOVED_EPSILON = 1e-4

interface PointsWidgetProps {
  kind: 'control_points' | 'multi_handle'
  /** Where the points start. */
  initial: readonly NormalizedPoint[]
  /** Where the user has moved them to, if at all. */
  target: readonly NormalizedPoint[] | null
  /** Draw a closed outline through the points. */
  outline: boolean
  /** What one point is called, for accessible labels, for example "Shape point". */
  pointName: string
  objectLabel: string
  context: WidgetContext
  onChange: (change: EditChange | null) => void
}

/** Draggable points, either joined into an outline (reshaping) or free-standing (posing). */
export function PointsWidget({
  kind,
  initial,
  target,
  outline,
  pointName,
  objectLabel,
  context,
  onChange,
}: PointsWidgetProps) {
  const { imageSize, view } = context
  const points = target ?? initial

  const toScreen = (p: Point) =>
    imageToScreen(view, { x: p.x * imageSize.width, y: p.y * imageSize.height })
  const emit = (next: readonly NormalizedPoint[]) =>
    onChange({ kind, origin: [...initial], target: [...next] } as EditChange)

  const move = (index: number, raw: Point, options: { snap: boolean; lock: boolean }) => {
    const origin = initial[index]
    if (!origin) return
    const constrained = constrainPoint(raw, origin, imageSize, {
      maxDistance: context.reach,
      snapRadius: options.snap ? SNAP_RADIUS_SCREEN_PX / view.scale : 0,
      axisLock: options.lock,
    })
    emit(points.map((p, i) => (i === index ? constrained : p)))
  }

  const polygon = points
    .map((p) => toScreen(p))
    .map((p) => `${p.x},${p.y}`)
    .join(' ')
  const ghost = initial
    .map((p) => toScreen(p))
    .map((p) => `${p.x},${p.y}`)
    .join(' ')

  return (
    <>
      {outline && (
        <svg className={styles.guides} aria-hidden="true">
          {target && <polygon className={styles.ghost} points={ghost} />}
          <polygon className={styles.outline} points={polygon} />
        </svg>
      )}
      {points.map((point, index) => {
        const origin = initial[index] ?? point
        const moved =
          Math.abs(point.x - origin.x) > MOVED_EPSILON ||
          Math.abs(point.y - origin.y) > MOVED_EPSILON
        return (
          <Handle
            // The points are a fixed, ordered set, so the index is a stable identity.
            key={index}
            point={point}
            context={context}
            label={`${pointName} ${index + 1} of ${points.length} on ${objectLabel}`}
            valueText={`${Math.round(point.x * 100)} percent across, ${Math.round(point.y * 100)} percent down`}
            moved={moved}
            onDrag={(x, y, modifiers) =>
              move(index, context.toImagePoint(x, y), { snap: true, lock: modifiers.shift })
            }
            onNudge={(dx, dy, fine) => {
              const step = fine ? FINE_STEP : STEP
              move(
                index,
                { x: point.x + dx * step, y: point.y + dy * step },
                { snap: false, lock: false },
              )
            }}
            onReset={() => emit(points.map((p, i) => (i === index ? origin : p)))}
          />
        )
      })}
    </>
  )
}
