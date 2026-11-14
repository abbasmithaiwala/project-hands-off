import { act, renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it } from 'vitest'
import type { ImageRecord } from '@/api/client'
import { useEditorStore } from '@/state/editorStore'
import { useVersionHistory } from './useVersionHistory'

const versions = ['a', 'b', 'c'].map((id) => ({ id }) as ImageRecord)

function press(key: string, init: KeyboardEventInit = {}) {
  act(() => {
    window.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, ...init }))
  })
}

beforeEach(() => {
  useEditorStore.getState().reset()
  useEditorStore.getState().openProject(versions)
})

describe('useVersionHistory', () => {
  it('knows when there is something to undo or redo', () => {
    const { result } = renderHook(() => useVersionHistory())
    expect(result.current).toMatchObject({ canUndo: true, canRedo: false })
    act(() => result.current.undo())
    expect(result.current).toMatchObject({ canUndo: true, canRedo: true })
    act(() => result.current.undo())
    expect(result.current).toMatchObject({ canUndo: false, canRedo: true })
  })

  it('undoes and redoes with the keyboard', () => {
    renderHook(() => useVersionHistory())
    press('z', { ctrlKey: true })
    expect(useEditorStore.getState().image?.id).toBe('b')
    press('z', { metaKey: true, shiftKey: true })
    expect(useEditorStore.getState().image?.id).toBe('c')
  })

  it('leaves the shortcut alone while typing in a field', () => {
    renderHook(() => useVersionHistory())
    const input = document.createElement('input')
    document.body.append(input)
    act(() => {
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'z', ctrlKey: true, bubbles: true }))
    })
    expect(useEditorStore.getState().image?.id).toBe('c')
    input.remove()
  })

  it('does nothing while an edit is being applied', () => {
    const { result } = renderHook(() => useVersionHistory())
    act(() => useEditorStore.getState().beginApply())
    expect(result.current.canUndo).toBe(false)
    press('z', { ctrlKey: true })
    expect(useEditorStore.getState().image?.id).toBe('c')
  })
})
