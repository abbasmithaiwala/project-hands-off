import { useEffect, useRef } from 'react'
import { startBlobField } from './blobField'
import styles from './BlobBackdrop.module.css'

const DARK_QUERY = '(prefers-color-scheme: dark)'
const REDUCED_MOTION_QUERY = '(prefers-reduced-motion: reduce)'

/** Whether the page is showing its dark theme right now, honouring an explicit choice. */
function isDarkTheme(): boolean {
  const chosen = document.documentElement.getAttribute('data-theme')
  if (chosen === 'dark') return true
  if (chosen === 'light') return false
  return window.matchMedia(DARK_QUERY).matches
}

/** Full-screen drifting blobs behind the landing page. The CSS gradient shows if WebGL cannot. */
export function BlobBackdrop() {
  const canvas = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    const element = canvas.current
    if (!element) return
    const still = window.matchMedia(REDUCED_MOTION_QUERY).matches
    const stop = startBlobField(element, { still, isDark: isDarkTheme })
    if (!stop) return undefined
    element.dataset['ready'] = 'true'
    return () => {
      delete element.dataset['ready']
      stop()
    }
  }, [])

  return (
    <div className={styles.backdrop} aria-hidden="true">
      <canvas ref={canvas} className={styles.canvas} data-testid="blob-canvas" />
    </div>
  )
}
