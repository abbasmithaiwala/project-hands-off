import type { EditResult } from '@/api/client'

const STAGES: Record<string, string> = {
  analysing: 'Preparing your edits',
  planning: 'Describing the changes',
  generating: 'Generating the image',
  compositing: 'Blending it into your photo',
  done: 'Almost there',
}

/** A person-friendly name for a pipeline stage. */
export function stageLabel(stage: string): string {
  return STAGES[stage] ?? 'Working'
}

/** For example "google/gemini-3-pro-image" becomes "gemini-3-pro-image". */
function shortModel(model: string): string {
  return model.split('/').pop() ?? model
}

/** One line on how an edit was produced: model, time and cost. */
export function describeRun(result: EditResult): string {
  const parts = [shortModel(result.model), `${(result.generation_ms / 1000).toFixed(1)}s`]
  if (result.cost_usd != null) parts.push(`$${result.cost_usd.toFixed(2)}`)
  return parts.join(' · ')
}
