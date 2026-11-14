import { useSyncExternalStore } from 'react'
import { useThemeStore } from './theme'

const DARK_QUERY = '(prefers-color-scheme: dark)'

function subscribeToSystem(onChange: () => void): () => void {
  const query = window.matchMedia(DARK_QUERY)
  query.addEventListener('change', onChange)
  return () => query.removeEventListener('change', onChange)
}

/** Whether the page is dark right now: the user's choice, or the system's when they have none. */
export function useIsDark(): boolean {
  const choice = useThemeStore((state) => state.choice)
  const systemIsDark = useSyncExternalStore(
    subscribeToSystem,
    () => window.matchMedia(DARK_QUERY).matches,
    () => false,
  )
  return choice === 'system' ? systemIsDark : choice === 'dark'
}
