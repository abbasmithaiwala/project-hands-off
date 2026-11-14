import { X } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { IconButton } from '@/design/IconButton'
import { Panel } from '@/design/Panel'
import styles from './OutputPanel.module.css'

interface OutputPanelProps {
  lines: readonly string[]
  onClose: () => void
}

/** The activity log: what the app and the models are doing, line by line. */
export function OutputPanel({ lines, onClose }: OutputPanelProps) {
  const listRef = useRef<HTMLOListElement>(null)

  useEffect(() => {
    const list = listRef.current
    if (list) list.scrollTop = list.scrollHeight
  }, [lines.length])

  return (
    <Panel className={styles.panel} role="complementary" aria-label="Activity log">
      <div className={styles.header}>
        Activity
        <IconButton label="Close activity" onClick={onClose}>
          <X size={16} />
        </IconButton>
      </div>
      {lines.length === 0 ? (
        <p className={styles.empty}>Nothing yet. Progress appears here as the models work.</p>
      ) : (
        <ol ref={listRef} className={styles.lines}>
          {lines.map((line, index) => (
            <li key={index}>{line}</li>
          ))}
        </ol>
      )}
    </Panel>
  )
}
