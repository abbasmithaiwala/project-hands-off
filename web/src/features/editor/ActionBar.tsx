import { Button } from '@/design/Button'
import { Panel } from '@/design/Panel'
import styles from './ActionBar.module.css'

interface ActionBarProps {
  pendingCount: number
  onApply: () => void
  onCancel: () => void
}

/** Floating Apply / Cancel pill, shown only while there are edits waiting to be applied. */
export function ActionBar({ pendingCount, onApply, onCancel }: ActionBarProps) {
  if (pendingCount === 0) return null
  return (
    <Panel glass className={styles.bar} role="group" aria-label="Pending edits">
      <span className={styles.summary}>
        {pendingCount === 1 ? '1 change' : `${pendingCount} changes`}
      </span>
      <Button variant="ghost" onClick={onCancel}>
        Cancel
      </Button>
      <Button variant="primary" onClick={onApply}>
        Apply
      </Button>
    </Panel>
  )
}
