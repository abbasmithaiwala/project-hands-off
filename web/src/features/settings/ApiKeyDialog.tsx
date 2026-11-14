import { useState, type FormEvent } from 'react'
import { useSaveApiKey } from '@/api/hooks'
import { describeError } from '@/api/errors'
import { Button } from '@/design/Button'
import { Dialog } from '@/design/Dialog'
import styles from './ApiKeyDialog.module.css'

interface ApiKeyDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/** Asks for the OpenRouter key at the moment it is first needed. */
export function ApiKeyDialog({ open, onOpenChange }: ApiKeyDialogProps) {
  const [key, setKey] = useState('')
  const save = useSaveApiKey()

  const submit = (event: FormEvent) => {
    event.preventDefault()
    save.mutate(key.trim(), {
      onSuccess: () => {
        setKey('')
        onOpenChange(false)
      },
    })
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="Connect OpenRouter"
      description="The AI models run through OpenRouter. Paste your API key once and it is kept on this server."
    >
      <form className={styles.form} onSubmit={submit}>
        <label className="visually-hidden" htmlFor="api-key">
          OpenRouter API key
        </label>
        <input
          id="api-key"
          className={styles.input}
          type="password"
          autoComplete="off"
          spellCheck={false}
          placeholder="sk-or-v1-…"
          value={key}
          onChange={(event) => setKey(event.target.value)}
        />
        {save.error && (
          <p className={styles.error} role="alert">
            {describeError(save.error)}
          </p>
        )}
        <p className={styles.hint}>
          Need one? Create it at{' '}
          <a href="https://openrouter.ai/keys" target="_blank" rel="noreferrer">
            openrouter.ai/keys
          </a>
          .
        </p>
        <div className={styles.actions}>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Not now
          </Button>
          <Button
            variant="primary"
            type="submit"
            disabled={key.trim().length === 0 || save.isPending}
          >
            {save.isPending ? 'Saving…' : 'Save key'}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
