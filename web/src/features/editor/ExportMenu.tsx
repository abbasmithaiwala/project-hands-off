import { Download } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import type { ImageRecord } from '@/api/client'
import { Button } from '@/design/Button'
import { IconButton } from '@/design/IconButton'
import { Panel } from '@/design/Panel'
import { downloadText, downloadUrl } from '@/lib/download'
import { downloadName, editLog, projectName } from '@/lib/versions'
import styles from './ExportMenu.module.css'

interface ExportMenuProps {
  versions: readonly ImageRecord[]
  current: ImageRecord
}

/** Save the current image, or the log of every edit made to the project. */
export function ExportMenu({ versions, current }: ExportMenuProps) {
  const [open, setOpen] = useState(false)
  const root = useRef<HTMLDivElement>(null)
  const name = projectName(current)

  useEffect(() => {
    if (!open) return
    const onPointerDown = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false)
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false)
    }
    document.addEventListener('pointerdown', onPointerDown)
    document.addEventListener('keydown', onKeyDown)
    return () => {
      document.removeEventListener('pointerdown', onPointerDown)
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [open])

  return (
    <div ref={root} className={styles.root}>
      <IconButton label="Export" aria-expanded={open} onClick={() => setOpen((value) => !value)}>
        <Download size={18} />
      </IconButton>
      {open && (
        <Panel glass className={styles.menu} role="group" aria-label="Export options">
          <Button
            variant="ghost"
            onClick={() => {
              downloadUrl(
                `/api/v1/images/${current.id}/original`,
                downloadName(name, current.format === 'jpeg' ? 'jpg' : current.format),
              )
              setOpen(false)
            }}
          >
            Download image
          </Button>
          <Button
            variant="ghost"
            onClick={() => {
              downloadText(
                editLog(versions),
                downloadName(`${name}-edits`, 'json'),
                'application/json',
              )
              setOpen(false)
            }}
          >
            Download edit log
          </Button>
        </Panel>
      )}
    </div>
  )
}
