import { PointsWidget } from './PointsWidget'
import type { WidgetProps } from './types'

/** Independent handles on parts of the object, like joints of a puppet. */
export function MultiHandleWidget({
  widget,
  change,
  propertyLabel,
  objectLabel,
  context,
  onChange,
}: WidgetProps<'multi_handle'>) {
  return (
    <PointsWidget
      kind="multi_handle"
      initial={widget.handles}
      target={change?.target ?? null}
      outline={false}
      pointName={`${propertyLabel} handle`}
      objectLabel={objectLabel}
      context={context}
      onChange={onChange}
    />
  )
}
