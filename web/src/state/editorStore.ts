import { create } from 'zustand'
import type { DetectedObject, Edit, EditResult, ImageRecord, Widget } from '@/api/client'
import { cycle } from '@/lib/geometry'

export type AnalysisStatus = 'idle' | 'running' | 'done' | 'failed'

/** An edit waiting to be applied, with what is needed to describe and preview it. */
export interface PendingEdit {
  key: string
  objectId: number
  objectLabel: string
  propertyLabel: string
  /** The control geometry the edit was made with. */
  widget: Widget
  edit: Edit
}

/** Where an Apply is up to. */
export type ApplyState =
  | { status: 'idle' }
  | { status: 'running'; jobId: string | null; stage: string; startedAt: number }
  | { status: 'review'; jobId: string; result: EditResult }

export function editKey(objectId: number, property: string): string {
  return `${objectId}:${property}`
}

const MAX_LOG_LINES = 500

interface EditorState {
  image: ImageRecord | null
  objects: DetectedObject[]
  hoveredId: number | null
  selectedId: number | null
  activeProperty: string | null
  analysis: AnalysisStatus
  analysisError: string | null
  log: string[]
  outputOpen: boolean
  pendingEdits: PendingEdit[]
  apply: ApplyState
}

interface EditorActions {
  openImage: (image: ImageRecord) => void
  addObject: (object: DetectedObject) => void
  setAnalysis: (status: AnalysisStatus, error?: string) => void
  appendLog: (line: string) => void
  hover: (id: number | null) => void
  select: (id: number | null) => void
  setActiveProperty: (key: string | null) => void
  selectNext: (direction: 1 | -1) => void
  setPendingEdit: (pending: PendingEdit) => void
  resetEdit: (key: string) => void
  clearEdits: () => void
  beginApply: () => void
  setApplyJob: (jobId: string) => void
  setApplyStage: (stage: string) => void
  finishApply: (jobId: string, result: EditResult) => void
  closeApply: () => void
  toggleOutput: () => void
  reset: () => void
}

const initial: EditorState = {
  image: null,
  objects: [],
  hoveredId: null,
  selectedId: null,
  activeProperty: null,
  analysis: 'idle',
  analysisError: null,
  log: [],
  outputOpen: false,
  pendingEdits: [],
  apply: { status: 'idle' },
}

export const useEditorStore = create<EditorState & EditorActions>()((set, get) => ({
  ...initial,
  openImage: (image) => set({ ...initial, outputOpen: get().outputOpen, image }),
  addObject: (object) =>
    set((state) =>
      state.objects.some((o) => o.id === object.id)
        ? state
        : { objects: [...state.objects, object] },
    ),
  setAnalysis: (analysis, error) =>
    set((state) => ({
      analysis,
      analysisError: error ?? null,
      objects: analysis === 'running' ? [] : state.objects,
      selectedId: analysis === 'running' ? null : state.selectedId,
      activeProperty: analysis === 'running' ? null : state.activeProperty,
      pendingEdits: analysis === 'running' ? [] : state.pendingEdits,
    })),
  appendLog: (line) => set((state) => ({ log: [...state.log, line].slice(-MAX_LOG_LINES) })),
  hover: (hoveredId) => set({ hoveredId }),
  select: (selectedId) => set({ selectedId, activeProperty: null }),
  setActiveProperty: (activeProperty) => set({ activeProperty }),
  selectNext: (direction) =>
    set((state) => ({
      activeProperty: null,
      selectedId: cycle(
        state.objects.map((o) => o.id),
        state.selectedId,
        direction,
      ),
    })),
  setPendingEdit: (pending) =>
    set((state) => ({
      pendingEdits: state.pendingEdits.some((e) => e.key === pending.key)
        ? state.pendingEdits.map((e) => (e.key === pending.key ? pending : e))
        : [...state.pendingEdits, pending],
    })),
  resetEdit: (key) =>
    set((state) => ({ pendingEdits: state.pendingEdits.filter((e) => e.key !== key) })),
  clearEdits: () => set({ pendingEdits: [] }),
  beginApply: () =>
    set({ apply: { status: 'running', jobId: null, stage: 'analysing', startedAt: Date.now() } }),
  setApplyJob: (jobId) =>
    set((state) =>
      state.apply.status === 'running' ? { apply: { ...state.apply, jobId } } : state,
    ),
  setApplyStage: (stage) =>
    set((state) =>
      state.apply.status === 'running' ? { apply: { ...state.apply, stage } } : state,
    ),
  finishApply: (jobId, result) => set({ apply: { status: 'review', jobId, result } }),
  closeApply: () => set({ apply: { status: 'idle' } }),
  toggleOutput: () => set((state) => ({ outputOpen: !state.outputOpen })),
  reset: () => set({ ...initial }),
}))
