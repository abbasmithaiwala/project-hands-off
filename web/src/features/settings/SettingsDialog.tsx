import { useState } from 'react'
import type { ModelSettings } from '@/api/client'
import { describeError } from '@/api/errors'
import { useClearApiKey, useSaveModels, useSettings } from '@/api/hooks'
import { useThemeStore, type ThemeChoice } from '@/app/theme'
import { Button } from '@/design/Button'
import { Dialog } from '@/design/Dialog'
import styles from './SettingsDialog.module.css'

const THEMES: Array<{ value: ThemeChoice; label: string }> = [
  { value: 'system', label: 'System' },
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
]

const MODEL_FIELDS: Array<{ key: keyof ModelSettings; label: string }> = [
  { key: 'suggester', label: 'Suggests what can be changed' },
  { key: 'ui_planner', label: 'Places the controls' },
  { key: 'summariser', label: 'Summarises edits' },
  { key: 'generator', label: 'Generates the result' },
  { key: 'draft_generator', label: 'Faster draft when retrying' },
  { key: 'fallback', label: 'Fallback when a model fails' },
]

const SUGGESTIONS = [
  'google/gemini-3.1-flash-lite',
  'google/gemini-3-pro-image',
  'google/gemini-3.1-flash-image',
  'anthropic/claude-haiku-5.5',
  'typesafe/jev-router',
]

interface SettingsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onChangeKey: () => void
}

function ModelsForm({ initial }: { initial: ModelSettings }) {
  const [models, setModels] = useState(initial)
  const saveModels = useSaveModels()

  return (
    <form
      className={styles.fields}
      onSubmit={(event) => {
        event.preventDefault()
        saveModels.mutate(models)
      }}
    >
      {MODEL_FIELDS.map(({ key, label }) => (
        <label key={key} className={styles.field}>
          {label}
          <input
            list="model-suggestions"
            value={models[key]}
            onChange={(event) => setModels({ ...models, [key]: event.target.value })}
          />
        </label>
      ))}
      <datalist id="model-suggestions">
        {SUGGESTIONS.map((id) => (
          <option key={id} value={id} />
        ))}
      </datalist>
      {saveModels.error && (
        <p className={styles.error} role="alert">
          {describeError(saveModels.error)}
        </p>
      )}
      <div className={styles.footer}>
        <Button variant="secondary" type="submit" disabled={saveModels.isPending}>
          Save models
        </Button>
      </div>
    </form>
  )
}

export function SettingsDialog({ open, onOpenChange, onChangeKey }: SettingsDialogProps) {
  const theme = useThemeStore((state) => state.choice)
  const setTheme = useThemeStore((state) => state.setChoice)
  const settings = useSettings()
  const clearKey = useClearApiKey()
  const origin = settings.data?.openrouter.origin
  const keyText =
    origin === 'demo'
      ? 'Demo mode: no key needed'
      : origin === 'environment'
        ? 'Set on the server'
        : origin === 'stored'
          ? 'Saved'
          : 'Not connected'

  return (
    <Dialog open={open} onOpenChange={onOpenChange} title="Settings">
      <div className={styles.section}>
        <h3 className={styles.heading}>Appearance</h3>
        <div className={styles.segmented} role="radiogroup" aria-label="Theme">
          {THEMES.map((option) => (
            <button
              key={option.value}
              role="radio"
              aria-checked={theme === option.value}
              className={styles.option}
              onClick={() => setTheme(option.value)}
            >
              {option.label}
            </button>
          ))}
        </div>
      </div>

      <div className={styles.section}>
        <h3 className={styles.heading}>OpenRouter</h3>
        <div className={styles.row}>
          <span>{keyText}</span>
          <span>
            {origin === 'stored' && (
              <Button variant="ghost" onClick={() => clearKey.mutate()}>
                Remove
              </Button>
            )}
            {origin !== 'demo' && origin !== 'environment' && (
              <Button variant="secondary" onClick={onChangeKey}>
                {origin === 'stored' ? 'Replace key' : 'Add key'}
              </Button>
            )}
          </span>
        </div>
      </div>

      <div className={styles.section}>
        <details className={styles.advanced}>
          <summary>Advanced: models</summary>
          {settings.data && (
            <ModelsForm key={JSON.stringify(settings.data.models)} initial={settings.data.models} />
          )}
        </details>
      </div>
    </Dialog>
  )
}
