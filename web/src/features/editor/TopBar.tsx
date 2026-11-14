import { Plus, ScrollText, Settings } from 'lucide-react'
import { IconButton } from '@/design/IconButton'
import { Panel } from '@/design/Panel'
import { cn } from '@/lib/cn'
import type { AnalysisStatus } from '@/state/editorStore'
import styles from './TopBar.module.css'
import { statusText } from './statusText'

interface TopBarProps {
  analysis: AnalysisStatus
  objectCount: number
  outputOpen: boolean
  onNewImage: () => void
  onToggleOutput: () => void
  onOpenSettings: () => void
}

export function TopBar({
  analysis,
  objectCount,
  outputOpen,
  onNewImage,
  onToggleOutput,
  onOpenSettings,
}: TopBarProps) {
  return (
    <header className={styles.bar}>
      <Panel glass className={styles.group}>
        <IconButton label="New image" onClick={onNewImage}>
          <Plus size={18} />
        </IconButton>
        <span className={styles.wordmark}>Hands-Off</span>
      </Panel>

      <Panel glass className={cn(styles.status, styles[analysis])} aria-live="polite">
        <span className={styles.dot} aria-hidden="true" />
        {statusText(analysis, objectCount)}
      </Panel>

      <Panel glass className={styles.group}>
        <IconButton label="Activity" aria-pressed={outputOpen} onClick={onToggleOutput}>
          <ScrollText size={18} />
        </IconButton>
        <IconButton label="Settings" onClick={onOpenSettings}>
          <Settings size={18} />
        </IconButton>
      </Panel>
    </header>
  )
}
