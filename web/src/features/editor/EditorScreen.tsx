import { useEffect, useMemo, useRef, useState } from 'react'
import type { EditChange, ImageRecord } from '@/api/client'
import { describeError, isKeyProblem } from '@/api/errors'
import { useProperties, useRenameProject, useSettings, useWidget } from '@/api/hooks'
import { toast } from '@/design/toastStore'
import { projectName } from '@/lib/versions'
import { describeChange, isUnchanged } from '@/lib/widgetMath'
import { editKey, useEditorStore } from '@/state/editorStore'
import { HistoryPanel } from '../history/HistoryPanel'
import { ApiKeyDialog } from '../settings/ApiKeyDialog'
import { SettingsDialog } from '../settings/SettingsDialog'
import { ActionBar } from './ActionBar'
import { AnalysisNotice } from './AnalysisNotice'
import { ApplyProgress } from './ApplyProgress'
import { CanvasStage } from './canvas/CanvasStage'
import type { CreateStage, StageHandle } from './canvas/stage'
import { CompareView } from './CompareView'
import { HintBanner } from './HintBanner'
import styles from './EditorScreen.module.css'
import { OutputPanel } from './OutputPanel'
import { buildPreviews, reachFor } from './previews'
import { PropertyMenu } from './PropertyMenu'
import { ReviewBar } from './ReviewBar'
import { TopBar } from './TopBar'
import { useAnalysis } from './useAnalysis'
import { useApplyEdits } from './useApplyEdits'
import { useVersionHistory } from './useVersionHistory'
import { WidgetLayer } from './widgets/WidgetLayer'

interface EditorScreenProps {
  image: ImageRecord
  /** Replaceable so tests can run without WebGL. */
  createStage?: CreateStage
}

export function EditorScreen({ image, createStage }: EditorScreenProps) {
  const [attempt, setAttempt] = useState(0)
  useAnalysis(image.id, attempt)

  const objects = useEditorStore((s) => s.objects)
  const hoveredId = useEditorStore((s) => s.hoveredId)
  const selectedId = useEditorStore((s) => s.selectedId)
  const activeProperty = useEditorStore((s) => s.activeProperty)
  const analysis = useEditorStore((s) => s.analysis)
  const analysisError = useEditorStore((s) => s.analysisError)
  const log = useEditorStore((s) => s.log)
  const outputOpen = useEditorStore((s) => s.outputOpen)
  const pendingEdits = useEditorStore((s) => s.pendingEdits)
  const apply = useEditorStore((s) => s.apply)
  const versions = useEditorStore((s) => s.versions)
  const cursor = useEditorStore((s) => s.cursor)
  const history = useVersionHistory()
  const rename = useRenameProject()
  const historyOpen = useEditorStore((s) => s.historyOpen)
  const { hover, select, selectNext, setActiveProperty, toggleOutput, toggleHistory, reset } =
    useEditorStore.getState()
  const { setPendingEdit, resetEdit, clearEdits } = useEditorStore.getState()

  const selected = objects.find((o) => o.id === selectedId) ?? null
  const properties = useProperties(image.id, selected?.id ?? null)
  const widget = useWidget(image.id, selected?.id ?? null, activeProperty)
  const property = properties.data?.properties.find((p) => p.key === activeProperty) ?? null
  const settings = useSettings()
  const models = settings.data?.models
  // A "faster draft" is only worth offering when it is a different model.
  const draftModel =
    models && models.draft_generator !== models.generator ? models.draft_generator : undefined

  const [keyRequested, setKeyRequested] = useState(false)
  const [dismissedError, setDismissedError] = useState<unknown>(null)
  const [settingsOpen, setSettingsOpen] = useState(false)

  const keyError = [properties.error, widget.error].find(isKeyProblem) ?? null
  const keyDialogOpen = keyRequested || (keyError !== null && keyError !== dismissedError)
  const setKeyDialogOpen = (open: boolean) => {
    setKeyRequested(open)
    if (!open) setDismissedError(keyError)
  }

  const stageRef = useRef<StageHandle | null>(null)
  const applying = useApplyEdits({ image, stageRef, onKeyProblem: () => setKeyRequested(true) })
  const editing = apply.status === 'idle'

  useEffect(() => {
    if (analysisError) toast.error(analysisError)
  }, [analysisError])

  const imageSize = useMemo(
    () => ({ width: image.width, height: image.height }),
    [image.width, image.height],
  )
  const previews = useMemo(
    () =>
      buildPreviews(
        pendingEdits,
        objects,
        imageSize,
        (id) => `/api/v1/images/${image.id}/objects/${id}/mask`,
      ),
    [pendingEdits, objects, imageSize, image.id],
  )

  const activeKey = selected && activeProperty ? editKey(selected.id, activeProperty) : null
  const activeChange = pendingEdits.find((e) => e.key === activeKey)?.edit.change ?? null
  const editedKeys = useMemo(
    () =>
      new Set(pendingEdits.filter((e) => e.objectId === selected?.id).map((e) => e.edit.property)),
    [pendingEdits, selected?.id],
  )

  const handleChange = (change: EditChange | null) => {
    if (!selected || !activeProperty || !widget.data || !activeKey) return
    if (change === null || isUnchanged(change, widget.data)) {
      resetEdit(activeKey)
      return
    }
    setPendingEdit({
      key: activeKey,
      objectId: selected.id,
      objectLabel: selected.label,
      propertyLabel: property?.label ?? activeProperty,
      widget: widget.data,
      edit: { object_id: selected.id, property: activeProperty, change },
    })
  }

  const items = pendingEdits.map((e) => ({
    key: e.key,
    label: `${e.propertyLabel} · ${e.objectLabel}`,
    summary: describeChange(e.edit.change),
  }))

  return (
    <div className={styles.editor}>
      <CanvasStage
        imageUrl={`/api/v1/images/${image.id}/original`}
        objects={objects}
        hoveredId={hoveredId}
        selectedId={selectedId}
        previews={previews}
        showPreviewNote={editing}
        quietSelection={editing && widget.data !== undefined && activeProperty !== null}
        stageRef={stageRef}
        overlay={({ view, imageSize: size }) => {
          if (apply.status === 'review') {
            return (
              <CompareView
                beforeUrl={`/api/v1/images/${image.id}/original`}
                afterUrl={`/api/v1/jobs/${apply.jobId}/result`}
                view={view}
                imageSize={size}
              />
            )
          }
          return editing && selected && activeProperty && widget.data ? (
            <WidgetLayer
              key={activeKey}
              widget={widget.data}
              change={activeChange}
              propertyLabel={property?.label ?? activeProperty}
              objectLabel={selected.label}
              view={view}
              imageSize={size}
              reach={reachFor(selected, size)}
              onChange={handleChange}
            />
          ) : null
        }}
        onHover={hover}
        onSelect={select}
        onCycle={selectNext}
        {...(createStage ? { createStage } : {})}
      />

      <TopBar
        projectName={projectName(image)}
        versions={versions}
        current={image}
        canUndo={history.canUndo}
        canRedo={history.canRedo}
        historyOpen={historyOpen}
        onRename={(name) =>
          rename.mutate(
            { id: image.id, name },
            {
              onSuccess: (renamed) => useEditorStore.getState().renameProject(renamed.name ?? name),
              onError: (error) => toast.error(describeError(error)),
            },
          )
        }
        onUndo={history.undo}
        onRedo={history.redo}
        onToggleHistory={toggleHistory}
        analysis={analysis}
        objectCount={objects.length}
        outputOpen={outputOpen}
        onNewImage={reset}
        onToggleOutput={toggleOutput}
        onOpenSettings={() => setSettingsOpen(true)}
      />

      <div className={styles.floating}>
        {editing && selected && (
          <PropertyMenu
            objectLabel={selected.label}
            properties={properties.data?.properties}
            loading={properties.isPending && properties.fetchStatus !== 'idle'}
            error={properties.error}
            activeKey={activeProperty}
            editedKeys={editedKeys}
            {...(activeProperty && widget.isPending && widget.fetchStatus !== 'idle'
              ? { status: 'Placing the control…' }
              : {})}
            onPick={setActiveProperty}
            onRetry={() => void properties.refetch()}
          />
        )}
        {editing && (
          <ActionBar
            items={items}
            onReset={resetEdit}
            onApply={() => void applying.run()}
            onCancel={clearEdits}
          />
        )}
        {apply.status === 'running' && (
          <ApplyProgress
            stage={apply.stage}
            startedAt={apply.startedAt}
            onCancel={() => void applying.cancel()}
          />
        )}
        {apply.status === 'review' && (
          <ReviewBar
            result={apply.result}
            draftModel={draftModel}
            onAccept={() => void applying.accept()}
            onRetry={() => void applying.run()}
            onDraft={() => void applying.run(draftModel)}
            onDiscard={applying.discard}
          />
        )}
      </div>

      {editing && (
        <HintBanner
          context={{
            hasObjects: objects.length > 0,
            hasSelection: selected !== null,
            hasActiveWidget: widget.data !== undefined && activeProperty !== null,
            hasPendingEdits: pendingEdits.length > 0,
          }}
        />
      )}

      <AnalysisNotice
        analysis={analysis}
        objectCount={objects.length}
        onRetry={() => setAttempt((count) => count + 1)}
        onNewImage={reset}
      />

      {historyOpen && (
        <HistoryPanel
          versions={versions}
          cursor={cursor}
          onSelect={useEditorStore.getState().goToVersion}
        />
      )}

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
          setKeyRequested(true)
        }}
      />
    </div>
  )
}
