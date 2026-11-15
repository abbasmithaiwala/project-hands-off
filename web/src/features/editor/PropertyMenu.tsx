import { Move, RotateCw, Scaling, SlidersHorizontal, Spline, Waypoints } from 'lucide-react'
import type { ComponentType } from 'react'
import type { Property } from '@/api/client'
import { describeError } from '@/api/errors'
import { Button } from '@/design/Button'
import { Panel } from '@/design/Panel'
import { Skeleton } from '@/design/Skeleton'
import styles from './PropertyMenu.module.css'

const ICONS: Record<
  Property['widget'],
  ComponentType<{ size?: number; 'aria-hidden'?: boolean }>
> = {
  slider: SlidersHorizontal,
  control_points: Spline,
  multi_handle: Waypoints,
  rotate: RotateCw,
  scale: Scaling,
  move: Move,
}

interface PropertyMenuProps {
  objectLabel: string
  properties: readonly Property[] | undefined
  loading: boolean
  error: unknown
  activeKey: string | null
  /** Properties that already have a pending edit. */
  editedKeys?: ReadonlySet<string>
  /** A short message shown under the chips, for example while a control is being placed. */
  status?: string
  onPick: (key: string) => void
  onRetry: () => void
}

/** The floating list of things that can be changed on the selected object. */
export function PropertyMenu({
  objectLabel,
  properties,
  loading,
  error,
  activeKey,
  editedKeys,
  status,
  onPick,
  onRetry,
}: PropertyMenuProps) {
  return (
    <Panel glass className={styles.menu} role="region" aria-label={`Edit ${objectLabel}`}>
      <div className={styles.title}>
        <span>Change</span>
        <strong>{objectLabel}</strong>
      </div>

      {loading && (
        <div className={styles.skeletons} aria-label="Loading suggestions">
          <Skeleton width={110} height={36} />
          <Skeleton width={130} height={36} />
          <Skeleton width={96} height={36} />
        </div>
      )}

      {Boolean(error) && !loading && (
        <div className={styles.error} role="alert">
          <span>{describeError(error)}</span>
          <Button variant="secondary" onClick={onRetry}>
            Try again
          </Button>
        </div>
      )}

      {properties && (
        <ul className={styles.chips}>
          {properties.map((property) => {
            const Icon = ICONS[property.widget]
            return (
              <li key={property.key}>
                <button
                  className={styles.chip}
                  aria-pressed={property.key === activeKey}
                  onClick={() => onPick(property.key)}
                >
                  <Icon size={16} aria-hidden />
                  {property.label}
                  {editedKeys?.has(property.key) && (
                    <span className={styles.edited} role="img" aria-label="edited" />
                  )}
                </button>
              </li>
            )
          })}
        </ul>
      )}
      {status && (
        <p className={styles.status} role="status">
          {status}
        </p>
      )}
    </Panel>
  )
}
