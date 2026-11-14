import { useState, type FormEvent } from 'react'
import styles from './ProjectName.module.css'

interface ProjectNameProps {
  name: string
  onRename: (name: string) => void
}

/** The project's name; click to rename, Enter to save, Escape to cancel. */
export function ProjectName({ name, onRename }: ProjectNameProps) {
  const [draft, setDraft] = useState<string | null>(null)

  const commit = (event?: FormEvent) => {
    event?.preventDefault()
    const next = draft?.trim()
    setDraft(null)
    if (next && next !== name) onRename(next)
  }

  if (draft === null) {
    return (
      <button
        type="button"
        className={styles.name}
        aria-label={`Project name: ${name}. Rename`}
        onClick={() => setDraft(name)}
      >
        {name}
      </button>
    )
  }
  return (
    <form onSubmit={commit}>
      <input
        className={styles.input}
        aria-label="Project name"
        value={draft}
        maxLength={80}
        autoFocus
        onChange={(event) => setDraft(event.target.value)}
        onBlur={() => commit()}
        onKeyDown={(event) => {
          if (event.key === 'Escape') setDraft(null)
        }}
      />
    </form>
  )
}
