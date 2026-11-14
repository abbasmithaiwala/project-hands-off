import { useEffect, useMemo, useState } from 'react'
import type { EditChange, ImageRecord } from '@/api/client'
import { isKeyProblem } from '@/api/errors'
import { useProperties, useWidget } from '@/api/hooks'
import { toast } from '@/design/toastStore'
import { describeChange, isUnchanged } from '@/lib/widgetMath'
import { editKey, useEditorStore } from '@/state/editorStore'
import { ApiKeyDialog } from '../settings/ApiKeyDialog'
import { SettingsDialog } from '../settings/SettingsDialog'
import { ActionBar } from './ActionBar'
import { CanvasStage } from './canvas/CanvasStage'
import type { CreateStage } from './canvas/stage'
import styles from './EditorScreen.module.css'
import { OutputPanel } from './OutputPanel'
import { buildPreviews, reachFor } from './previews'
import { PropertyMenu } from './PropertyMenu'
import { TopBar } from './TopBar'
import { useAnalysis } from './useAnalysis'
import { WidgetLayer } from './widgets/WidgetLayer'

interface EditorScreenProps {
  image: ImageRecord
  /** Replaceable so tests can run without WebGL. */
  createStage?: CreateStage
}

const APPLY_UNAVAILABLE = 'Generating the edited image is not connected yet.'

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
  const { setPendingEdit, resetEdit, clearEdits } = useEditorStore.getState()

  const selected = objects.find((o) => o.id === selectedId) ?? null
  const properties = useProperties(image.id, selected?.id ?? null)
  const widget = useWidget(image.id, selected?.id ?? null, activeProperty)
  const property = properties.data?.properties.find((p) => p.key === activeProperty) ?? null

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
        overlay={({ view, imageSize: size }) =>
          selected && activeProperty && widget.data ? (
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
        }
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
            editedKeys={editedKeys}
            {...(activeProperty && widget.isPending && widget.fetchStatus !== 'idle'
              ? { status: 'Placing the control…' }
              : {})}
            onPick={setActiveProperty}
            onRetry={() => void properties.refetch()}
          />
        )}
        <ActionBar
          items={items}
          onReset={resetEdit}
          onApply={() => undefined}
          onCancel={clearEdits}
          applyUnavailable={APPLY_UNAVAILABLE}
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
          setKeyRequested(true)
        }}
      />
    </div>
  )
}
