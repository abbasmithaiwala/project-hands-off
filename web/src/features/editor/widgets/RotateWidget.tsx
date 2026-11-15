import { RotateCw } from 'lucide-react'
import { clamp } from '@/lib/numbers'
import { normalisedToScreen } from '@/lib/viewport'
import { rotateAbout } from '@/lib/warp'
import { describeChange, rotateDegreesAt, rotateHandlePoint } from '@/lib/widgetMath'
import { Handle } from './Handle'
import { ObjectFrame } from './ObjectFrame'
import type { WidgetProps } from './types'
import styles from './WidgetLayer.module.css'

const STEP_DEGREES = 5
const FINE_STEP_DEGREES = 1
const TICK_PX = 7
const RADIANS_PER_DEGREE = Math.PI / 180

/** A ring round a pivot with a handle on it: drag the handle round the ring to turn the object. */
export function RotateWidget({
  widget,
  change,
  propertyLabel,
  objectLabel,
  context,
  onChange,
}: WidgetProps<'rotate'>) {
  const { imageSize, view, objectBox } = context
  const degrees = change?.degrees ?? 0
  const toScreen = (p: { x: number; y: number }) => normalisedToScreen(view, imageSize, p)

  const pivot = toScreen(widget.pivot)
  const rest = toScreen(widget.handle)
  const now = toScreen(rotateHandlePoint(widget, degrees, imageSize))
  const radius = Math.hypot(rest.x - pivot.x, rest.y - pivot.y)

  const arc = (from: { x: number; y: number }, to: { x: number; y: number }) =>
    `M ${pivot.x} ${pivot.y} L ${from.x} ${from.y} A ${radius} ${radius} 0 ${Math.abs(degrees) > 180 ? 1 : 0} ${degrees >= 0 ? 1 : 0} ${to.x} ${to.y} Z`

  const tick = (limit: number) => {
    const at = toScreen(rotateHandlePoint(widget, limit, imageSize))
    const ux = (at.x - pivot.x) / radius
    const uy = (at.y - pivot.y) / radius
    return {
      x1: at.x - ux * TICK_PX,
      y1: at.y - uy * TICK_PX,
      x2: at.x + ux * TICK_PX,
      y2: at.y + uy * TICK_PX,
    }
  }

  const turn = rotateAbout(
    { x: widget.pivot.x * imageSize.width, y: widget.pivot.y * imageSize.height },
    degrees * RADIANS_PER_DEGREE,
  )

  const commit = (next: number) =>
    onChange({ kind: 'rotate', degrees: clamp(next, widget.min, widget.max), pivot: widget.pivot })

  const rounded = Math.round(degrees)
  const direction = rounded < 0 ? 'anticlockwise' : 'clockwise'

  return (
    <>
      <svg className={styles.guides} aria-hidden="true">
        <ObjectFrame
          box={objectBox}
          imageSize={imageSize}
          view={view}
          deform={turn}
          edited={change !== null}
        />
        <circle className={styles.ring} cx={pivot.x} cy={pivot.y} r={radius} />
        <line className={styles.tick} {...tick(widget.min)} />
        <line className={styles.tick} {...tick(widget.max)} />
        {change && <path className={styles.sector} d={arc(rest, now)} />}
        <line className={styles.spoke} x1={pivot.x} y1={pivot.y} x2={now.x} y2={now.y} />
        <circle className={styles.anchor} cx={pivot.x} cy={pivot.y} r={5} />
      </svg>
      <Handle
        point={rotateHandlePoint(widget, degrees, imageSize)}
        context={context}
        role="slider"
        roleDescription="rotation handle"
        glyph={<RotateCw size={14} strokeWidth={2.5} />}
        label={`${propertyLabel} of ${objectLabel}`}
        valueText={rounded === 0 ? 'not turned' : `${Math.abs(rounded)} degrees ${direction}`}
        ariaValue={{ min: Math.round(widget.min), max: Math.round(widget.max), now: rounded }}
        moved={change !== null}
        onDrag={(x, y) => commit(rotateDegreesAt(widget, context.toImagePoint(x, y), imageSize))}
        onNudge={(dx, dy, fine) =>
          commit(degrees + (dx - dy) * (fine ? FINE_STEP_DEGREES : STEP_DEGREES))
        }
        onReset={() => onChange(null)}
      />
      {change && (
        <span className={styles.badge} style={{ left: now.x, top: now.y }}>
          {describeChange(change)}
        </span>
      )}
    </>
  )
}
