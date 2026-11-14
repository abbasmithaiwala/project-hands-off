import type { ButtonHTMLAttributes } from 'react'
import { cn } from '@/lib/cn'
import styles from './Button.module.css'

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost'
  size?: 'default' | 'large'
}

export function Button({
  variant = 'secondary',
  size = 'default',
  className,
  type = 'button',
  ...rest
}: ButtonProps) {
  return (
    <button
      type={type}
      className={cn(styles.button, styles[variant], size === 'large' && styles.large, className)}
      {...rest}
    />
  )
}
