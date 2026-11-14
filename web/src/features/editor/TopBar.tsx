import { History, Plus, Redo2, ScrollText, Settings, Undo2 } from 'lucide-react'
import type { ImageRecord } from '@/api/client'
import { ThemeToggle } from '@/app/ThemeToggle'
import { IconButton } from '@/design/IconButton'
import { Panel } from '@/design/Panel'
import { cn } from '@/lib/cn'
import type { AnalysisStatus } from '@/state/editorStore'
import { ExportMenu } from './ExportMenu'
import { ProjectName } from './ProjectName'
import styles from './TopBar.module.css'
import { statusText } from './statusText'

interface TopBarProps {
  projectName: string
  versions: readonly ImageRecord[]
  current: ImageRecord
  canUndo: boolean
  canRedo: boolean
  historyOpen: boolean
  onRename: (name: string) => void
  onUndo: () => void
  onRedo: () => void
  onToggleHistory: () => void
  analysis: AnalysisStatus
  objectCount: number
  outputOpen: boolean
  onNewImage: () => void
  onToggleOutput: () => void
  onOpenSettings: () => void
}

export function TopBar({
  projectName,
  versions,
  current,
  canUndo,
  canRedo,
  historyOpen,
  onRename,
  onUndo,
  onRedo,
  onToggleHistory,
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
        <ProjectName name={projectName} onRename={onRename} />
        <IconButton label="Undo" disabled={!canUndo} onClick={onUndo}>
          <Undo2 size={18} />
        </IconButton>
        <IconButton label="Redo" disabled={!canRedo} onClick={onRedo}>
          <Redo2 size={18} />
        </IconButton>
      </Panel>

      <Panel glass className={cn(styles.status, styles[analysis])} aria-live="polite">
        <span className={styles.dot} aria-hidden="true" />
        {statusText(analysis, objectCount)}
      </Panel>

      <Panel glass className={styles.group}>
        <IconButton label="History" aria-pressed={historyOpen} onClick={onToggleHistory}>
          <History size={18} />
        </IconButton>
        <ExportMenu versions={versions} current={current} />
        <ThemeToggle />
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
