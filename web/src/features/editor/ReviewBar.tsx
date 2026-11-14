import type { EditResult } from '@/api/client'
import { Button } from '@/design/Button'
import { Panel } from '@/design/Panel'
import { describeRun } from './formatting'
import styles from './ReviewBar.module.css'

interface ReviewBarProps {
  result: EditResult
  /** The faster model offered for a quick retry, if one is configured. */
  draftModel: string | undefined
  onAccept: () => void
  onRetry: () => void
  onDraft: () => void
  onDiscard: () => void
}

/** What changed, what it cost, and the choices: Accept, Retry, try a faster draft, or Discard. */
export function ReviewBar({
  result,
  draftModel,
  onAccept,
  onRetry,
  onDraft,
  onDiscard,
}: ReviewBarProps) {
  return (
    <Panel glass className={styles.panel} role="region" aria-label="Review your edit">
      <span className={styles.title}>Review your edit</span>
      <p className={styles.summary}>{result.summary}</p>
      <ul className={styles.changes} aria-label="Changes made">
        {result.changes.map((change) => (
          <li key={change.edit_index}>{change.text}</li>
        ))}
      </ul>
      <p className={styles.meta}>{describeRun(result)}</p>
      <div className={styles.actions}>
        <Button variant="ghost" onClick={onDiscard}>
          Discard
        </Button>
        <span className={styles.spacer} />
        {draftModel && (
          <Button variant="secondary" onClick={onDraft}>
            Faster draft
          </Button>
        )}
        <Button variant="secondary" onClick={onRetry}>
          Retry
        </Button>
        <Button variant="primary" onClick={onAccept}>
          Accept
        </Button>
      </div>
    </Panel>
  )
}
