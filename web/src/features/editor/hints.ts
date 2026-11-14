const STORAGE_KEY = 'hands-off.hints-done'

/** The first-run tips, in the order they are shown. */
export type HintId = 'pick-object' | 'drag-handle'

export const HINT_TEXT: Record<HintId, string> = {
  'pick-object': 'Hover over something in your photo, then click it to see what you can change.',
  'drag-handle': 'Drag the handle to reshape it. Nothing is generated until you press Apply.',
}

export interface HintContext {
  hasObjects: boolean
  hasSelection: boolean
  hasActiveWidget: boolean
  hasPendingEdits: boolean
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
