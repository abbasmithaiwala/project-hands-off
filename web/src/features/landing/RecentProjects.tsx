import { Trash2 } from 'lucide-react'
import { useState } from 'react'
import type { ImageRecord } from '@/api/client'
import { describeError } from '@/api/errors'
import { useDeleteProject, useOpenProject, useProjects } from '@/api/hooks'
import { Button } from '@/design/Button'
import { Dialog } from '@/design/Dialog'
import { IconButton } from '@/design/IconButton'
import { Skeleton } from '@/design/Skeleton'
import { toast } from '@/design/toastStore'
import { projectName, timeAgo } from '@/lib/versions'
import { useEditorStore } from '@/state/editorStore'
import styles from './RecentProjects.module.css'

/** Saved projects, newest first. Renders nothing when there are none, so first runs stay calm. */
export function RecentProjects() {
  const projects = useProjects()
  const open = useOpenProject()
  const remove = useDeleteProject()
  const [pendingDelete, setPendingDelete] = useState<ImageRecord | null>(null)

  if (projects.isPending) {
    return (
      <div className={styles.row} role="status" aria-label="Loading recent projects">
        {[0, 1, 2].map((key) => (
          <Skeleton key={key} width={160} height={120} />
        ))}
      </div>
    )
  }
  if (!projects.data?.length) return null

  const openProject = (project: ImageRecord) =>
    open.mutate(project.id, {
      onSuccess: (versions) => useEditorStore.getState().openProject(versions),
      onError: (error) => toast.error(describeError(error)),
    })

  const confirmDelete = () => {
    if (!pendingDelete) return
    remove.mutate(pendingDelete.id, {
      onSuccess: () => setPendingDelete(null),
      onError: (error) => {
        setPendingDelete(null)
        toast.error(describeError(error))
      },
    })
  }

  return (
    <section className={styles.section} aria-labelledby="recent-heading">
      <h2 id="recent-heading" className={styles.heading}>
        Recent projects
      </h2>
      <ul className={styles.row}>
        {projects.data.map((project) => (
          <li key={project.id} className={styles.card}>
            <button
              type="button"
              className={styles.open}
              aria-label={`Open ${projectName(project)}`}
              disabled={open.isPending}
              onClick={() => openProject(project)}
            >
              <img
                className={styles.thumb}
                src={`/api/v1/images/${project.id}/thumbnail`}
                alt=""
                loading="lazy"
              />
              <span className={styles.name}>{projectName(project)}</span>
              <span className={styles.meta}>{timeAgo(project.created_at)}</span>
            </button>
            <IconButton
              label={`Delete ${projectName(project)}`}
              className={styles.delete}
              onClick={() => setPendingDelete(project)}
            >
              <Trash2 size={16} />
            </IconButton>
          </li>
        ))}
      </ul>

      <Dialog
        open={pendingDelete !== null}
        onOpenChange={(isOpen) => !isOpen && setPendingDelete(null)}
        title="Delete this project?"
        description="The photo and every version of it will be removed from this server. This cannot be undone."
      >
        <div className={styles.actions}>
          <Button variant="ghost" onClick={() => setPendingDelete(null)}>
            Keep it
          </Button>
          <Button variant="primary" disabled={remove.isPending} onClick={confirmDelete}>
            Delete
          </Button>
        </div>
      </Dialog>
    </section>
  )
}
