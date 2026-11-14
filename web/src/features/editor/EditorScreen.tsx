import { useEffect, useState } from 'react'
import type { ImageRecord } from '@/api/client'
import { isKeyProblem } from '@/api/errors'
import { useProperties, useWidget } from '@/api/hooks'
import { toast } from '@/design/toastStore'
import { useEditorStore } from '@/state/editorStore'
import { ApiKeyDialog } from '../settings/ApiKeyDialog'
import { SettingsDialog } from '../settings/SettingsDialog'
import { ActionBar } from './ActionBar'
import { CanvasStage } from './canvas/CanvasStage'
import type { CreateStage } from './canvas/stage'
import styles from './EditorScreen.module.css'
import { OutputPanel } from './OutputPanel'
import { PropertyMenu } from './PropertyMenu'
import { TopBar } from './TopBar'
import { useAnalysis } from './useAnalysis'

interface EditorScreenProps {
  image: ImageRecord
  /** Replaceable so tests can run without WebGL. */
  createStage?: CreateStage
}

export function EditorScreen({ image, createStage }: EditorScreenProps) {
  useAnalysis(image.id)

  const objects = useEditorStore((s) => s.objects)
  const hoveredId = useEditorStore((s) => s.hoveredId)
  const selectedId = useEditorStore((s) => s.selectedId)
  const activeProperty = useEditorStore((s) => s.activeProperty)
  const analysis = useEditorStore((s) => s.analysis)
  const analysisError = useEditorStore((s) => s.analysisError)
  const log = useEditorStore((s) => s.log)
  const outputOpen = useEditorStore((s) => s.outputOpen)
  const pendingEdits = useEditorStore((s) => s.pendingEdits)
  const { hover, select, selectNext, setActiveProperty, toggleOutput, reset } =
    useEditorStore.getState()

  const selected = objects.find((o) => o.id === selectedId) ?? null
  const properties = useProperties(image.id, selected?.id ?? null)
  const widget = useWidget(image.id, selected?.id ?? null, activeProperty)

  const [keyRequested, setKeyRequested] = useState(false)
  const [dismissedError, setDismissedError] = useState<unknown>(null)
  const [settingsOpen, setSettingsOpen] = useState(false)

  const keyError = [properties.error, widget.error].find(isKeyProblem) ?? null
  const keyDialogOpen = keyRequested || (keyError !== null && keyError !== dismissedError)
  const setKeyDialogOpen = (open: boolean) => {
    setKeyRequested(open)
    if (!open) setDismissedError(keyError)
  }

  useEffect(() => {
    if (analysisError) toast.error(analysisError)
  }, [analysisError])

  return (
    <div className={styles.editor}>
      <CanvasStage
        imageUrl={`/api/v1/images/${image.id}/original`}
        objects={objects}
        hoveredId={hoveredId}
        selectedId={selectedId}
        widget={activeProperty ? (widget.data ?? null) : null}
        onHover={hover}
        onSelect={select}
        onCycle={selectNext}
        {...(createStage ? { createStage } : {})}
      />

      <TopBar
        analysis={analysis}
        objectCount={objects.length}
        outputOpen={outputOpen}
        onNewImage={reset}
        onToggleOutput={toggleOutput}
        onOpenSettings={() => setSettingsOpen(true)}
      />

      <div className={styles.floating}>
        {selected && (
          <PropertyMenu
            objectLabel={selected.label}
            properties={properties.data?.properties}
            loading={properties.isPending && properties.fetchStatus !== 'idle'}
            error={properties.error}
            activeKey={activeProperty}
            onPick={setActiveProperty}
            onRetry={() => void properties.refetch()}
          />
        )}
        <ActionBar
          pendingCount={pendingEdits.length}
          onApply={() => undefined}
          onCancel={() => undefined}
        />
      </div>

      {outputOpen && <OutputPanel lines={log} onClose={toggleOutput} />}

      <div className="visually-hidden" role="status" aria-live="polite">
        {selected ? `Selected ${selected.label}` : ''}
      </div>

      <ApiKeyDialog open={keyDialogOpen} onOpenChange={setKeyDialogOpen} />
      <SettingsDialog
        open={settingsOpen}
        onOpenChange={setSettingsOpen}
        onChangeKey={() => {
          setSettingsOpen(false)
          setKeyDialogOpen(true)
        }}
      />
    </div>
  )
}
