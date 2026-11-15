import { clamp } from '@/lib/numbers'
import { normalisedToScreen } from '@/lib/viewport'
import { describeChange, sliderHandlePoint, sliderValueAt } from '@/lib/widgetMath'
import { Handle } from './Handle'
import type { WidgetProps } from './types'
import styles from './WidgetLayer.module.css'

const STEP = 0.05
const FINE_STEP = 0.01
const LABEL_CHAR_PX = 8
const LABEL_PADDING_PX = 48

/** A translucent bar laid on the object with a fixed end and a handle: drag to stretch it. */
export function SliderWidget({
  widget,
  change,
  propertyLabel,
  objectLabel,
  context,
  onChange,
}: WidgetProps<'slider'>) {
  const { imageSize, view } = context
  const value = change?.value ?? widget.initial
  const handle = sliderHandlePoint(widget, value, imageSize)
  const toScreen = (p: { x: number; y: number }) => normalisedToScreen(view, imageSize, p)
  const start = toScreen(widget.axis.start)
  const now = toScreen(handle)

  const length = Math.hypot(now.x - start.x, now.y - start.y)
  const fitsLabel = length > propertyLabel.length * LABEL_CHAR_PX + LABEL_PADDING_PX
  // Keep the text readable: never upside down.
  const angle = (Math.atan2(now.y - start.y, now.x - start.x) * 180) / Math.PI
  const upright = angle > 90 || angle < -90 ? angle + 180 : angle

  const commit = (next: number) =>
    onChange({ kind: 'slider', value: clamp(next, widget.min, widget.max) })

  return (
    <>
      <svg className={styles.guides} aria-hidden="true">
        <line className={styles.bar} x1={start.x} y1={start.y} x2={now.x} y2={now.y} />
        {fitsLabel && (
          <text
            className={styles.barLabel}
            transform={`translate(${(start.x + now.x) / 2} ${(start.y + now.y) / 2}) rotate(${upright})`}
          >
            {propertyLabel}
          </text>
        )}
        <circle className={styles.anchor} cx={start.x} cy={start.y} r={7} />
      </svg>
      <Handle
        point={handle}
        context={context}
        role="slider"
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
