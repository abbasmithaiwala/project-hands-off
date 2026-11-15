import type { Widget } from '@/api/client'

const STORAGE_KEY = 'hands-off.hints-done'

/** The first-run tips, in the order they are shown. */
export type HintId = 'pick-object' | 'drag-handle'

const NOTHING_GENERATED = 'Nothing is generated until you press Apply.'

/** What dragging does, so the tip matches the control on screen. */
const DRAG_TEXT: Record<Widget['kind'], string> = {
  slider: 'Drag the handle along the bar to stretch it.',
  control_points: 'Drag the points to reshape it.',
  multi_handle: 'Drag the handles to pose it.',
  rotate: 'Drag the handle around the ring to turn it.',
  scale: 'Drag the corner handle to make it bigger or smaller.',
  move: 'Drag the handle to move it.',
}

/** The tip's wording. The drag tip describes the control that is showing, when there is one. */
export function hintText(id: HintId, widgetKind: Widget['kind'] | null): string {
  switch (id) {
    case 'pick-object':
      return 'Hover over something in your photo, then click it to see what you can change.'
    case 'drag-handle':
      return `${widgetKind ? DRAG_TEXT[widgetKind] : 'Drag the handle to change it.'} ${NOTHING_GENERATED}`
  }
}

export interface HintContext {
  hasObjects: boolean
  hasSelection: boolean
  hasActiveWidget: boolean
  hasPendingEdits: boolean
  /** The kind of control showing, if any. */
  widgetKind: Widget['kind'] | null
}

/** Which tip fits what the user is doing right now, if any. */
export function currentHint(context: HintContext): HintId | null {
  if (context.hasPendingEdits) return null
  if (context.hasActiveWidget) return 'drag-handle'
  if (context.hasObjects && !context.hasSelection) return 'pick-object'
  return null
}

export function readDoneHints(): ReadonlySet<HintId> {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    const parsed: unknown = raw ? JSON.parse(raw) : []
    return new Set(
      Array.isArray(parsed)
        ? parsed.filter((id): id is HintId => id === 'pick-object' || id === 'drag-handle')
        : [],
    )
  } catch {
    return new Set()
  }
}

export function markHintDone(id: HintId): void {
  try {
    const done = new Set(readDoneHints()).add(id)
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify([...done]))
  } catch {
    /* tips simply show again next visit when storage is unavailable */
  }
}
