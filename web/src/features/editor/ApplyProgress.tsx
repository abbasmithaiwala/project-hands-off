import { useEffect, useState } from 'react'
import { Button } from '@/design/Button'
import { Panel } from '@/design/Panel'
import styles from './ApplyProgress.module.css'
import { stageLabel } from './formatting'

/** Whole seconds elapsed since `startedAt`, updated every second. */
function useElapsedSeconds(startedAt: number): number {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000)
    return () => window.clearInterval(timer)
  }, [])
  return Math.max(0, Math.floor((now - startedAt) / 1000))
}

interface ApplyProgressProps {
  stage: string
  startedAt: number
  onCancel: () => void
}

/** Shown while an edit is being generated: what stage it is at, how long it has taken, Cancel. */
export function ApplyProgress({ stage, startedAt, onCancel }: ApplyProgressProps) {
  const seconds = useElapsedSeconds(startedAt)
  return (
    <Panel
      glass
      className={styles.bar}
      role="status"
      aria-live="polite"
      aria-label="Applying your edits"
    >
      <span className={styles.dot} aria-hidden="true" />
      <span className={styles.text}>
        <span className={styles.stage}>{stageLabel(stage)}…</span>
        <span className={styles.detail}>{seconds}s · usually about 20 seconds</span>
      </span>
      <Button variant="ghost" onClick={onCancel}>
        Cancel
      </Button>
    </Panel>
  )
}
