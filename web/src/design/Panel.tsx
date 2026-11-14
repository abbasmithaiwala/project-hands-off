import type { HTMLAttributes } from 'react'
import { cn } from '@/lib/cn'
import styles from './Panel.module.css'

interface PanelProps extends HTMLAttributes<HTMLDivElement> {
  glass?: boolean
}

export function Panel({ glass = false, className, ...rest }: PanelProps) {
  return <div className={cn(styles.panel, glass && styles.glass, className)} {...rest} />
}
