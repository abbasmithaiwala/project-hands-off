import type { ImageRecord } from '@/api/client'
import { Panel } from '@/design/Panel'
import { cn } from '@/lib/cn'
import { timeAgo, versionTitle } from '@/lib/versions'
import styles from './HistoryPanel.module.css'

interface HistoryPanelProps {
  versions: readonly ImageRecord[]
  cursor: number
  onSelect: (index: number) => void
}

/** The project's versions, newest first, each with what its edit changed. */
export function HistoryPanel({ versions, cursor, onSelect }: HistoryPanelProps) {
  return (
    <Panel glass className={styles.panel} role="complementary" aria-label="Version history">
      <h2 className={styles.header}>History</h2>
      <ol className={styles.list}>
        {versions
          .map((version, index) => ({ version, index }))
          .reverse()
          .map(({ version, index }) => (
            <li key={version.id}>
              <button
                type="button"
                className={cn(styles.item, index === cursor && styles.current)}
                aria-current={index === cursor ? 'step' : undefined}
                onClick={() => onSelect(index)}
              >
                <img
                  className={styles.thumb}
                  src={`/api/v1/images/${version.id}/thumbnail`}
                  alt=""
                  loading="lazy"
                  width={48}
                  height={48}
                />
                <span className={styles.text}>
                  <span className={styles.title}>{versionTitle(version)}</span>
                  <span className={styles.meta}>
                    {index === 0 ? 'Start' : `Version ${index + 1}`} · {timeAgo(version.created_at)}
                  </span>
                </span>
              </button>
            </li>
          ))}
      </ol>
    </Panel>
  )
}
