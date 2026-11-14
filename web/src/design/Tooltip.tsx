import * as RadixTooltip from '@radix-ui/react-tooltip'
import type { ReactElement } from 'react'
import styles from './Tooltip.module.css'

interface TooltipProps {
  content: string
  children: ReactElement
}

export function Tooltip({ content, children }: TooltipProps) {
  return (
    <RadixTooltip.Provider delayDuration={400}>
      <RadixTooltip.Root>
        <RadixTooltip.Trigger asChild>{children}</RadixTooltip.Trigger>
        <RadixTooltip.Portal>
          <RadixTooltip.Content className={styles.tooltip} sideOffset={8}>
            {content}
          </RadixTooltip.Content>
        </RadixTooltip.Portal>
      </RadixTooltip.Root>
    </RadixTooltip.Provider>
  )
}
