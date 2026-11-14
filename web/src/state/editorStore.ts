import { create } from 'zustand'
import type { DetectedObject, Edit, ImageRecord } from '@/api/client'
import { cycle } from '@/lib/geometry'

export type AnalysisStatus = 'idle' | 'running' | 'done' | 'failed'

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
  pendingEdits: Edit[]
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
  toggleOutput: () => set((state) => ({ outputOpen: !state.outputOpen })),
  reset: () => set({ ...initial }),
}))
