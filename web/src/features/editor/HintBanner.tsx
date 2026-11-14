import { X } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { IconButton } from '@/design/IconButton'
import { Panel } from '@/design/Panel'
import {
  currentHint,
  HINT_TEXT,
  markHintDone,
  readDoneHints,
  type HintContext,
  type HintId,
} from './hints'
import styles from './HintBanner.module.css'

/** A short first-run tip, retired for good once the user moves past it or dismisses it. */
export function HintBanner({ context }: { context: HintContext }) {
  const [done, setDone] = useState(readDoneHints)
  const hint = currentHint(context)
  const shown = useRef<HintId | null>(null)

  const retire = (id: HintId) => {
    markHintDone(id)
    setDone(readDoneHints())
  }

  useEffect(() => {
    const previous = shown.current
    shown.current = hint && !done.has(hint) ? hint : null
    if (previous && previous !== shown.current) retire(previous)
  }, [hint, done])

  if (!hint || done.has(hint)) return null

  return (
    <Panel glass className={styles.hint} role="status" aria-live="polite">
      <span>{HINT_TEXT[hint]}</span>
      <IconButton label="Dismiss tip" onClick={() => retire(hint)}>
        <X size={16} />
      </IconButton>
    </Panel>
  )
}
