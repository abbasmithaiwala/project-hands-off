import { cn } from '@/lib/cn'
import styles from './ObjectLabel.module.css'

interface ObjectLabelProps {
  text: string
  /** Screen position of the object's top-left corner. */
  x: number
  y: number
  selected: boolean
}

export function ObjectLabel({ text, x, y, selected }: ObjectLabelProps) {
  return (
    <span className={cn(styles.label, selected && styles.selected)} style={{ left: x, top: y }}>
      {text}
    </span>
  )
}
