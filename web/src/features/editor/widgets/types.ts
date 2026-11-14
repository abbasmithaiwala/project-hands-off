import type { EditChange, Widget } from '@/api/client'
import type { Point } from '@/lib/geometry'
import type { Size, Viewport } from '@/lib/viewport'

/** Everything a widget needs to know about where it is drawn and how far it may reach. */
export interface WidgetContext {
  view: Viewport
  imageSize: Size
  /** How far (in image pixels) a point may travel from where it started. */
  reach: number
  /** Convert a pointer position to a normalised image point. */
  toImagePoint: (clientX: number, clientY: number) => Point
}

export interface WidgetProps<K extends Widget['kind']> {
  widget: Extract<Widget, { kind: K }>
  /** The edit already made with this widget, if any. */
  change: Extract<EditChange, { kind: K }> | null
  /** Names used in accessible labels. */
  propertyLabel: string
  objectLabel: string
  context: WidgetContext
  /** Report a new change, or `null` to clear it. */
  onChange: (change: EditChange | null) => void
}
