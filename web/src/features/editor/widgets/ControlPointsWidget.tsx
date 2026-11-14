import { PointsWidget } from './PointsWidget'
import type { WidgetProps } from './types'

/** Points around an outline: drag them to reshape the object. */
export function ControlPointsWidget({
  widget,
  change,
  propertyLabel,
  objectLabel,
  context,
  onChange,
}: WidgetProps<'control_points'>) {
  return (
    <PointsWidget
      kind="control_points"
      initial={widget.points}
      target={change?.target ?? null}
      outline
      pointName={`${propertyLabel} point`}
      objectLabel={objectLabel}
      context={context}
      onChange={onChange}
    />
  )
}
