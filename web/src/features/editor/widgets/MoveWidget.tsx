import { Move } from 'lucide-react'
import type { Point } from '@/lib/geometry'
import { normalisedToScreen } from '@/lib/viewport'
import { translate } from '@/lib/warp'
import { constrainPoint, describeChange, describeMove } from '@/lib/widgetMath'
import { Handle } from './Handle'
import { ObjectFrame } from './ObjectFrame'
import type { WidgetProps } from './types'
import styles from './WidgetLayer.module.css'

const SNAP_RADIUS_SCREEN_PX = 6
const STEP = 0.01
const FINE_STEP = 0.001

/** A handle on the object's body: drag it to carry the whole object somewhere else. */
export function MoveWidget({
  widget,
  change,
  propertyLabel,
  objectLabel,
  context,
  onChange,
}: WidgetProps<'move'>) {
  const { imageSize, view, objectBox } = context
  const dx = change?.dx ?? 0
  const dy = change?.dy ?? 0
  const handle = { x: widget.handle.x + dx, y: widget.handle.y + dy }
  const from = normalisedToScreen(view, imageSize, widget.handle)
  const now = normalisedToScreen(view, imageSize, handle)

  const moveTo = (raw: Point, snap: boolean, lock: boolean) => {
    const kept = constrainPoint(raw, widget.handle, imageSize, {
      maxDistance: context.reach,
      snapRadius: snap ? SNAP_RADIUS_SCREEN_PX / view.scale : 0,
      axisLock: lock,
    })
    onChange({ kind: 'move', dx: kept.x - widget.handle.x, dy: kept.y - widget.handle.y })
  }

  return (
    <>
      <svg className={styles.guides} aria-hidden="true">
        <ObjectFrame
          box={objectBox}
          imageSize={imageSize}
          view={view}
          deform={translate(dx * imageSize.width, dy * imageSize.height)}
          edited={change !== null}
        />
        {change && <line className={styles.spoke} x1={from.x} y1={from.y} x2={now.x} y2={now.y} />}
        {change && <circle className={styles.anchor} cx={from.x} cy={from.y} r={4} />}
      </svg>
      <Handle
        point={handle}
        context={context}
        roleDescription="move handle"
        glyph={<Move size={14} strokeWidth={2.5} />}
        label={`${propertyLabel} of ${objectLabel}`}
        valueText={change ? `moved ${describeMove(dx, dy)}` : 'not moved'}
        moved={change !== null}
        onDrag={(x, y, modifiers) => moveTo(context.toImagePoint(x, y), true, modifiers.shift)}
        onNudge={(stepX, stepY, fine) => {
          const step = fine ? FINE_STEP : STEP
          moveTo({ x: handle.x + stepX * step, y: handle.y + stepY * step }, false, false)
        }}
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
