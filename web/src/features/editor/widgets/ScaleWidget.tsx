import { Scaling } from 'lucide-react'
import { clamp } from '@/lib/numbers'
import { normalisedToScreen } from '@/lib/viewport'
import { scaleAbout } from '@/lib/warp'
import { describeChange, sliderHandlePoint, sliderValueAt } from '@/lib/widgetMath'
import { Handle } from './Handle'
import { ObjectFrame } from './ObjectFrame'
import type { WidgetProps } from './types'
import styles from './WidgetLayer.module.css'

const STEP = 0.05
const FINE_STEP = 0.01

/** A corner handle that resizes the whole object evenly about a fixed anchor. */
export function ScaleWidget({
  widget,
  change,
  propertyLabel,
  objectLabel,
  context,
  onChange,
}: WidgetProps<'scale'>) {
  const { imageSize, view, objectBox } = context
  const value = change?.value ?? widget.initial
  const handle = sliderHandlePoint(widget, value, imageSize)
  const anchor = normalisedToScreen(view, imageSize, widget.axis.start)
  const now = normalisedToScreen(view, imageSize, handle)

  const grow = scaleAbout(
    { x: widget.axis.start.x * imageSize.width, y: widget.axis.start.y * imageSize.height },
    value / widget.initial,
  )

  const commit = (next: number) =>
    onChange({
      kind: 'scale',
      value: clamp(next, widget.min, widget.max),
      pivot: widget.axis.start,
    })

  return (
    <>
      <svg className={styles.guides} aria-hidden="true">
        <ObjectFrame
          box={objectBox}
          imageSize={imageSize}
          view={view}
          deform={grow}
          edited={change !== null}
        />
        <line className={styles.spoke} x1={anchor.x} y1={anchor.y} x2={now.x} y2={now.y} />
        <circle className={styles.anchor} cx={anchor.x} cy={anchor.y} r={5} />
      </svg>
      <Handle
        point={handle}
        context={context}
        role="slider"
        roleDescription="resize handle"
        glyph={<Scaling size={14} strokeWidth={2.5} />}
        label={`${propertyLabel} of ${objectLabel}`}
        valueText={`${Math.round(value * 100)} percent`}
        ariaValue={{
          min: Math.round(widget.min * 100),
          max: Math.round(widget.max * 100),
          now: Math.round(value * 100),
        }}
        moved={change !== null}
        onDrag={(x, y) => commit(sliderValueAt(widget, context.toImagePoint(x, y), imageSize))}
        onNudge={(dx, dy, fine) => commit(value + (dx - dy) * (fine ? FINE_STEP : STEP))}
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
