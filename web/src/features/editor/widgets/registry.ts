import type { ComponentType } from 'react'
import type { Widget } from '@/api/client'
import { ControlPointsWidget } from './ControlPointsWidget'
import { MoveWidget } from './MoveWidget'
import { MultiHandleWidget } from './MultiHandleWidget'
import { RotateWidget } from './RotateWidget'
import { ScaleWidget } from './ScaleWidget'
import { SliderWidget } from './SliderWidget'
import type { WidgetProps } from './types'

/**
 * One component per widget kind. `satisfies` makes the compiler fail if a kind is added to the
 * API without a component here, or a component takes the wrong props.
 */
export const widgetRegistry = {
  slider: SliderWidget,
  control_points: ControlPointsWidget,
  multi_handle: MultiHandleWidget,
  rotate: RotateWidget,
  scale: ScaleWidget,
  move: MoveWidget,
} satisfies { [K in Widget['kind']]: ComponentType<WidgetProps<K>> }
