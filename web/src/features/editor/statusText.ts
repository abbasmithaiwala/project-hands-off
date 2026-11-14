import type { AnalysisStatus } from '@/state/editorStore'

export function statusText(analysis: AnalysisStatus, objectCount: number): string {
  switch (analysis) {
    case 'running':
      return objectCount > 0 ? `Finding objects · ${objectCount} so far` : 'Finding objects…'
    case 'done':
      return objectCount === 1 ? '1 object found' : `${objectCount} objects found`
    case 'failed':
      return 'Analysis failed'
    case 'idle':
      return 'Ready'
  }
}
