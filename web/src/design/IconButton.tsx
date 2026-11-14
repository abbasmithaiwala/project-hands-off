import type { ButtonHTMLAttributes, ReactNode } from 'react'
import { cn } from '@/lib/cn'
import styles from './IconButton.module.css'
import { Tooltip } from './Tooltip'

interface IconButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'aria-label'> {
  /** Accessible name, also shown as a tooltip. */
  label: string
  children: ReactNode
}

export function IconButton({
  label,
  children,
  className,
  type = 'button',
  ...rest
}: IconButtonProps) {
  return (
    <Tooltip content={label}>
      <button type={type} aria-label={label} className={cn(styles.iconButton, className)} {...rest}>
        {children}
      </button>
    </Tooltip>
  )
}
