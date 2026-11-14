import { ChevronUp, RotateCcw } from 'lucide-react'
import { useId, useState } from 'react'
import { Button } from '@/design/Button'
import { IconButton } from '@/design/IconButton'
import { Panel } from '@/design/Panel'
import { cn } from '@/lib/cn'
import styles from './ActionBar.module.css'

export interface PendingItem {
  key: string
  /** For example "Couch length". */
  label: string
  /** What the user changed, for example "80%". */
  summary: string
}

interface ActionBarProps {
  items: readonly PendingItem[]
  onReset: (key: string) => void
  onApply: () => void
  onCancel: () => void
  /** When set, Apply is disabled and this explains why. */
  applyUnavailable?: string
}

/** Floating pill listing the changes waiting to be applied, with Apply and Cancel. */
export function ActionBar({ items, onReset, onApply, onCancel, applyUnavailable }: ActionBarProps) {
  const [open, setOpen] = useState(false)
  const listId = useId()
  const reasonId = useId()
  if (items.length === 0) return null

  return (
    <Panel glass className={styles.wrapper} role="group" aria-label="Pending edits">
      {open && (
        <ul id={listId} className={styles.list} aria-label="Pending changes">
          {items.map((item) => (
            <li key={item.key} className={styles.item}>
              <span className={styles.itemLabel}>{item.label}</span>
              <span className={styles.itemSummary}>{item.summary}</span>
              <IconButton label={`Reset ${item.label}`} onClick={() => onReset(item.key)}>
                <RotateCcw size={14} />
              </IconButton>
            </li>
          ))}
        </ul>
      )}
      <div className={styles.bar}>
        <button
          className={styles.summary}
          aria-expanded={open}
          aria-controls={listId}
          onClick={() => setOpen((value) => !value)}
        >
          {items.length === 1 ? '1 change' : `${items.length} changes`}
          <ChevronUp
            size={14}
            className={cn(styles.chevron, open && styles.chevronOpen)}
            aria-hidden
          />
        </button>
        <Button variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
        <Button
          variant="primary"
          disabled={Boolean(applyUnavailable)}
          aria-describedby={applyUnavailable ? reasonId : undefined}
          onClick={onApply}
        >
          Apply
        </Button>
      </div>
      {applyUnavailable && (
        <p id={reasonId} className={styles.reason}>
          {applyUnavailable}
        </p>
      )}
    </Panel>
  )
}
