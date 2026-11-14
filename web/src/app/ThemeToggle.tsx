import { Moon, Sun } from 'lucide-react'
import { IconButton } from '@/design/IconButton'
import { useThemeStore } from './theme'
import { useIsDark } from './useResolvedTheme'

/** Switches between the light and dark theme; the choice is remembered. */
export function ThemeToggle({ className }: { className?: string }) {
  const isDark = useIsDark()
  const setChoice = useThemeStore((state) => state.setChoice)
  return (
    <IconButton
      label={isDark ? 'Switch to light theme' : 'Switch to dark theme'}
      {...(className ? { className } : {})}
      onClick={() => setChoice(isDark ? 'light' : 'dark')}
    >
      {isDark ? <Sun size={18} /> : <Moon size={18} />}
    </IconButton>
  )
}
