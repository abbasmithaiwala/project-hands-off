import type { CSSProperties } from 'react'
import styles from './Skeleton.module.css'

interface SkeletonProps {
  width?: CSSProperties['width']
  height?: CSSProperties['height']
}

export function Skeleton({ width = '100%', height = 14 }: SkeletonProps) {
  return <span className={styles.skeleton} style={{ width, height }} aria-hidden="true" />
}
