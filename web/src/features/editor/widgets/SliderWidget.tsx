import { clamp } from '@/lib/numbers'
import { imageToScreen } from '@/lib/viewport'
import { describeChange, sliderHandlePoint, sliderValueAt } from '@/lib/widgetMath'
import { Handle } from './Handle'
import type { WidgetProps } from './types'
import styles from './WidgetLayer.module.css'

const STEP = 0.05
const FINE_STEP = 0.01

/** One handle on a line: dragging it stretches the object along that line. */
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
  const reach = sliderHandlePoint(widget, widget.max, imageSize)
  const shortest = sliderHandlePoint(widget, widget.min, imageSize)

  const toScreen = (p: { x: number; y: number }) =>
    imageToScreen(view, { x: p.x * imageSize.width, y: p.y * imageSize.height })
  const start = toScreen(widget.axis.start)
  const far = toScreen(reach)
  const near = toScreen(shortest)
  const now = toScreen(handle)

  const commit = (next: number) =>
    onChange({ kind: 'slider', value: clamp(next, widget.min, widget.max) })

  return (
    <>
      <svg className={styles.guides} aria-hidden="true">
        <line className={styles.trackReach} x1={near.x} y1={near.y} x2={far.x} y2={far.y} />
        <line className={styles.track} x1={start.x} y1={start.y} x2={now.x} y2={now.y} />
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
