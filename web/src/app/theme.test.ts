import { afterEach, describe, expect, it } from 'vitest'
import { applyStoredTheme, readStoredTheme, useThemeStore } from './theme'

const root = document.documentElement

afterEach(() => {
  window.localStorage.clear()
  root.removeAttribute('data-theme')
  useThemeStore.setState({ choice: 'system' })
})

describe('theme', () => {
  it('follows the system by default', () => {
    expect(readStoredTheme()).toBe('system')
    expect(root.hasAttribute('data-theme')).toBe(false)
  })

  it('applies and remembers an explicit choice', () => {
    useThemeStore.getState().setChoice('light')
    expect(root.getAttribute('data-theme')).toBe('light')
    expect(readStoredTheme()).toBe('light')
    expect(useThemeStore.getState().choice).toBe('light')
  })

  it('restores the remembered choice at startup', () => {
    window.localStorage.setItem('hands-off.theme', 'dark')
    applyStoredTheme()
    expect(root.getAttribute('data-theme')).toBe('dark')
    expect(useThemeStore.getState().choice).toBe('dark')
  })

  it('returns to the system theme and forgets the choice', () => {
    useThemeStore.getState().setChoice('dark')
    useThemeStore.getState().setChoice('system')
    expect(root.hasAttribute('data-theme')).toBe(false)
    expect(window.localStorage.getItem('hands-off.theme')).toBeNull()
  })

  it('ignores unknown stored values', () => {
    window.localStorage.setItem('hands-off.theme', 'neon')
    expect(readStoredTheme()).toBe('system')
  })
})
