import { useEffect } from 'react'
import { useEditorStore } from '@/state/editorStore'

function isTyping(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))
  )
}

/**
 * Undo and redo between a project's versions, with Ctrl/Cmd+Z and Ctrl/Cmd+Shift+Z.
 *
 * Disabled while an edit is being applied or reviewed, when moving versions would discard it.
 */
export function useVersionHistory() {
  const cursor = useEditorStore((s) => s.cursor)
  const count = useEditorStore((s) => s.versions.length)
  const idle = useEditorStore((s) => s.apply.status === 'idle')
  const canUndo = idle && cursor > 0
  const canRedo = idle && cursor < count - 1

  const undo = () => useEditorStore.getState().goToVersion(cursor - 1)
  const redo = () => useEditorStore.getState().goToVersion(cursor + 1)

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'z') return
      if (isTyping(event.target)) return
      event.preventDefault()
      const state = useEditorStore.getState()
      if (state.apply.status !== 'idle') return
      state.goToVersion(state.cursor + (event.shiftKey ? 1 : -1))
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])

  return { canUndo, canRedo, undo, redo }
}
