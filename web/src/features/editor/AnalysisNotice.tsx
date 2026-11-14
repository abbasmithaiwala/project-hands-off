import { Button } from '@/design/Button'
import { Panel } from '@/design/Panel'
import type { AnalysisStatus } from '@/state/editorStore'
import styles from './AnalysisNotice.module.css'

interface AnalysisNoticeProps {
  analysis: AnalysisStatus
  objectCount: number
  onRetry: () => void
  onNewImage: () => void
}

/** What to do next when analysis found nothing or did not finish. Renders nothing otherwise. */
export function AnalysisNotice({
  analysis,
  objectCount,
  onRetry,
  onNewImage,
}: AnalysisNoticeProps) {
  if (analysis === 'failed') {
    return (
      <Panel glass className={styles.notice} role="alert">
        <p>We could not finish looking at this photo.</p>
        <div className={styles.actions}>
          <Button variant="ghost" onClick={onNewImage}>
            Choose another photo
          </Button>
          <Button variant="primary" onClick={onRetry}>
            Try again
          </Button>
        </div>
      </Panel>
    )
  }
  if (analysis === 'done' && objectCount === 0) {
    return (
      <Panel glass className={styles.notice} role="status">
        <p>Nothing editable was found. Photos with a clear main subject work best.</p>
        <div className={styles.actions}>
          <Button variant="primary" onClick={onNewImage}>
            Choose another photo
          </Button>
        </div>
      </Panel>
    )
  }
  return null
}
