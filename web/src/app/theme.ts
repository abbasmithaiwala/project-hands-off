import { create } from 'zustand'

export type ThemeChoice = 'system' | 'light' | 'dark'

const STORAGE_KEY = 'hands-off.theme'

export function readStoredTheme(): ThemeChoice {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY)
    return stored === 'light' || stored === 'dark' ? stored : 'system'
  } catch {
    return 'system'
  }
}

function applyToDocument(choice: ThemeChoice) {
  const root = document.documentElement
  if (choice === 'system') root.removeAttribute('data-theme')
  else root.setAttribute('data-theme', choice)
}

function persist(choice: ThemeChoice) {
  try {
    if (choice === 'system') window.localStorage.removeItem(STORAGE_KEY)
    else window.localStorage.setItem(STORAGE_KEY, choice)
  } catch {
    /* storage may be unavailable; the choice still applies for this visit */
  }
}

interface ThemeState {
  choice: ThemeChoice
  setChoice: (choice: ThemeChoice) => void
}

/** The user's theme choice: one shared value, applied to the document and remembered. */
export const useThemeStore = create<ThemeState>()((set) => ({
  choice: readStoredTheme(),
  setChoice: (choice) => {
    persist(choice)
    applyToDocument(choice)
    set({ choice })
  },
}))

/** Apply the remembered theme. Call once at startup, before the first render, to avoid a flash. */
export function applyStoredTheme() {
  const choice = readStoredTheme()
  useThemeStore.setState({ choice })
  applyToDocument(choice)
}
