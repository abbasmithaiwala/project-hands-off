import { useRef, useState, type KeyboardEvent, type PointerEvent } from 'react'
import { clamp } from '@/lib/numbers'
import type { Size, Viewport } from '@/lib/viewport'
import styles from './CompareView.module.css'

const STEP = 5
const FINE_STEP = 1

interface CompareViewProps {
  beforeUrl: string
  afterUrl: string
  view: Viewport
  imageSize: Size
}

/** The original and the edited image on top of each other, with a draggable divider. */
export function CompareView({ beforeUrl, afterUrl, view, imageSize }: CompareViewProps) {
  const [position, setPosition] = useState(50)
  const frame = useRef<HTMLDivElement>(null)
  const dragging = useRef(false)

  const moveTo = (clientX: number) => {
    const rect = frame.current?.getBoundingClientRect()
    if (!rect || rect.width === 0) return
    setPosition(clamp(((clientX - rect.left) / rect.width) * 100, 0, 100))
  }

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    dragging.current = true
    event.currentTarget.setPointerCapture(event.pointerId)
    moveTo(event.clientX)
  }
  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (dragging.current) moveTo(event.clientX)
  }
  const onPointerUp = (event: PointerEvent<HTMLDivElement>) => {
    dragging.current = false
    event.currentTarget.releasePointerCapture(event.pointerId)
  }

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.shiftKey ? FINE_STEP : STEP
    const next =
      event.key === 'ArrowLeft' || event.key === 'ArrowDown'
        ? position - step
        : event.key === 'ArrowRight' || event.key === 'ArrowUp'
          ? position + step
          : event.key === 'Home'
            ? 0
            : event.key === 'End'
              ? 100
              : null
    if (next === null) return
    event.preventDefault()
    setPosition(clamp(next, 0, 100))
  }

  return (
    <div
      ref={frame}
      className={styles.frame}
      style={{
        left: view.x,
        top: view.y,
        width: imageSize.width * view.scale,
        height: imageSize.height * view.scale,
      }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      data-testid="compare-view"
    >
      <img
        className={styles.image}
        src={afterUrl}
        alt="After: the edited photo"
        draggable={false}
      />
      <img
        className={styles.image}
        src={beforeUrl}
        alt="Before: the original photo"
        draggable={false}
        style={{ clipPath: `inset(0 ${100 - position}% 0 0)` }}
      />
      <span className={`${styles.tag} ${styles.before}`}>Before</span>
      <span className={`${styles.tag} ${styles.after}`}>After</span>
      <div className={styles.divider} style={{ left: `${position}%` }}>
        <div
          className={styles.grip}
          role="slider"
          tabIndex={0}
          aria-label="Compare before and after"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(position)}
          aria-valuetext={`${Math.round(position)} percent of the original shown`}
          style={{ pointerEvents: 'auto' }}
          onKeyDown={onKeyDown}
        >
          ⇆
        </div>
      </div>
    </div>
  )
}
